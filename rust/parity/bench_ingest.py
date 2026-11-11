#!/usr/bin/env python3
"""Ingest speed and memory benchmark (T032, NFR-005): the release Rust `ingest` beside the Python `ingest`.

Generates invented transcript data (seeded, so every run makes the same bytes) and runs both implementations over it:

    A  200 MiB over many files, cold (a fresh data folder each run)       gate: >= 8 MB/s
    C  set A again, unchanged, into the store the cold run of A just made  gate: < 2 s
    B  one 12 MiB file, cold                                               gate: peak RSS < 8 MiB above the idle baseline

    cd rust && cargo build --release && ./parity/.venv/bin/python parity/bench_ingest.py
    ./parity/.venv/bin/python parity/bench_ingest.py --impl rust --runs 1     # quick look at Rust only

MB is 2**20 bytes, as `tests/sources/test_ingest_perf.py` counts it (`total / 1024 / 1024 / seconds`). Each cold run is
repeated `--runs` times (default 3) and the median is reported. Peak RSS is the program's own `ru_maxrss` as GNU
`/usr/bin/time -f %M` reports it (why not our own `wait4`: `Runner.run`); the idle baseline is the same program running `--version`. Before every cold run the dataset files are dropped from the
OS page cache with `posix_fadvise(DONTNEED)` (best effort, no root needed; the program, its libraries and, on a VM, the
host's disk cache are not dropped). Every child has a hard time limit (`--timeout`); a run that hits it is reported as
TIMEOUT, never as a number. A speed number counts only when the `events`, `tool_signals` and `source_files` tables of
every cold run equal those of the first Python run (row id, machine_id and inode left out; the paths are the same).

Everything is written under `--root` (default `rust/target/bench/`, git-ignored): `datasets/` (reused while its
manifest matches) and one new `runs/<UTC time>/` folder per invocation. Nothing is deleted. The script refuses to run
with less than 1 GiB free there, or when a path points at a real home's `.claude*` / `.codex*` / `.easy-*` folders.
Exit 0: every Rust gate passed and every table matched; 1: a gate missed, a table differed or a run failed.
"""

import argparse
import hashlib
import json
import os
import platform
import random
import resource
import shutil
import signal
import sqlite3
import statistics
import subprocess
import sys
import threading
import time
from pathlib import Path

sys.dont_write_bytecode = True
PARITY = Path(__file__).resolve().parent
RUST = PARITY.parent
REPO = RUST.parent
sys.path.insert(0, str(PARITY))
import gen_golden  # noqa: E402  (the real-home list and the sqlite guard)

MIB = 1024 * 1024
SEED = 32032
GEN_VERSION = 1  # bump when the generator changes: the datasets are made again
A_BYTES, B_BYTES = 200 * MIB, 12 * MIB
GATE_RATE, GATE_RERUN_S, GATE_MEM_MIB = 8.0, 2.0, 8.0  # NFR-005; never tuned here
MIN_FREE = 1024**3
BINARY = RUST / "target" / "release" / "easy-personal-rs"
TIME = "/usr/bin/time"  # GNU time: the child's peak RSS, see Runner.run
ENTRY = "import sys; from easy_personal.surfaces.cli import main; sys.exit(main())"  # the console script
DEAD_CONTEXT = "http://127.0.0.1:9/mcp"  # nothing listens there (as tests/conftest.py)
TABLES = ("events", "tool_signals", "source_files")
DROPPED = {"id", "machine_id", "inode"}
WORDS = (
    "invented filler bench sample widget module value render parse check table event queue report signal layer "
    "token shape record window branch status build index cache tests output result config helper"
).split()


# -- guards ----------------------------------------------------------------------------------------------------


def refuse(name: str, value: Path) -> None:
    target = value.expanduser().resolve()
    for home in gen_golden._real_homes():
        if target == home:
            sys.exit(f"refused: {name} is the real home folder; nothing was opened")
        if home in target.parents and target.relative_to(home).parts[0].startswith(gen_golden.PROTECTED_PREFIXES):
            sys.exit(f"refused: {name} points inside the real {target.relative_to(home).parts[0]}")


def filesystem(path: Path) -> tuple[str, str, str]:
    """(mount point, fs type, device) of the mount holding `path` (/proc/mounts, longest prefix)."""
    best = ("/", "?", "?")
    target = str(path.resolve())
    for line in Path("/proc/mounts").read_text().splitlines():
        device, mount, fstype = line.split()[:3]
        mount = mount.replace("\\040", " ")
        inside = target == mount or target.startswith(mount.rstrip("/") + "/")
        if inside and len(mount) >= len(best[0]):
            best = (mount, fstype, device)
    return best


# -- the generator ---------------------------------------------------------------------------------------------


class Gen:
    """Invented transcript lines shaped like `docs/spikes/transcript-shapes.md`: about 3.8 KB per line on average,
    user prompts, assistant records split per content block (same `message.id`, same usage), tool calls and their
    results (a few large), attachments, system lines and metadata lines without `version`. No real text, path or secret.
    """

    def __init__(self, seed: int) -> None:
        self.rng = random.Random(seed)
        words = [self.rng.choice(WORDS) for _ in range(40_000)]
        for i in range(0, len(words), 97):  # escapes and non-ASCII, as real text has
            words[i] = self.rng.choice(['"quoted"', "tab\there", "line\nbreak", "café", "back\\slash"])
        self.block = " ".join(words)
        self.n = 0

    def text(self, size: int) -> str:
        size = min(size, len(self.block) - 1)
        start = self.rng.randrange(len(self.block) - size)
        return self.block[start : start + size]

    def uid(self) -> str:
        h = f"{self.rng.getrandbits(128):032x}"
        return f"{h[:8]}-{h[8:12]}-4{h[13:16]}-8{h[17:20]}-{h[20:]}"

    def session(self, cwd: str, start: int):
        """Lines of one session, forever (the caller stops at a size)."""
        rng, sid, clock, parent = self.rng, self.uid(), start, None

        def stamp() -> str:
            nonlocal clock
            clock += rng.randint(1, 40)
            return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(clock)) + f".{rng.randrange(1000):03d}Z"

        def base(kind: str, **extra) -> dict:
            nonlocal parent
            uid = self.uid()
            rec = {"parentUuid": parent, "isSidechain": rng.random() < 0.03, "userType": "external", "cwd": cwd,
                   "sessionId": sid, "version": "2.1.295", "gitBranch": "main", "entrypoint": "cli", "type": kind,
                   "uuid": uid, "timestamp": stamp(), **extra}  # fmt: skip
            parent = uid
            return rec

        yield {"type": "agent-name", "agentName": "bench-agent", "sessionId": sid}
        yield {"type": "custom-title", "customTitle": self.text(40), "sessionId": sid}
        while True:
            yield base("user", message={"role": "user", "content": self.text(rng.randint(80, 2500))})
            for _ in range(rng.randint(1, 4)):
                self.n += 1
                mid = f"msg_bench_{self.n:08d}"
                usage = {"input_tokens": rng.randint(1, 9000), "output_tokens": rng.randint(1, 3000),
                         "cache_creation_input_tokens": rng.randint(0, 20000),
                         "cache_read_input_tokens": rng.randint(0, 200000),
                         "cache_creation": {"ephemeral_5m_input_tokens": 0, "ephemeral_1h_input_tokens": 0},
                         "service_tier": "standard"}  # fmt: skip
                blocks = [{"type": "thinking", "thinking": self.text(rng.randint(100, 2000)), "signature": "sig"}]
                blocks.append({"type": "text", "text": self.text(rng.randint(50, 1500))})
                tools = [self.tool() for _ in range(rng.choice((0, 1, 1, 2)))]
                blocks += tools
                for block in blocks:  # one record per content block, identical usage
                    extra = {}
                    if block.get("name", "").startswith("mcp__"):
                        extra = {"attributionMcpServer": "bench", "attributionMcpTool": "lookup"}
                    msg = {"id": mid, "type": "message", "role": "assistant", "model": "claude-bench-1",
                           "content": [block], "stop_reason": None, "usage": usage}  # fmt: skip
                    yield base("assistant", message=msg, requestId=f"req_{self.n:08d}", **extra)
                for tool in tools:
                    size = rng.randint(200_000, 600_000) if rng.random() < 0.01 else rng.randint(300, 9000)
                    result = {"type": "tool_result", "tool_use_id": tool["id"], "is_error": rng.random() < 0.08,
                              "content": self.text(size)}  # fmt: skip
                    yield base("user", message={"role": "user", "content": [result]},
                               toolUseResult={"stdout": self.text(size // 3), "stderr": ""})  # fmt: skip
                if rng.random() < 0.3:
                    yield base("attachment", attachment={"type": "hook_output", "content": self.text(400)})
            if rng.random() < 0.02:
                msg = {"id": f"msg_err_{self.n}", "role": "assistant", "model": "<synthetic>",
                       "content": [{"type": "text", "text": "API Error: invented"}],
                       "usage": {"input_tokens": 0, "output_tokens": 0}}  # fmt: skip
                yield base("assistant", message=msg, error="rate_limit", isApiErrorMessage=True)
            if rng.random() < 0.2:
                yield base("system", subtype="turn_duration", durationMs=rng.randint(100, 90000), isMeta=False)
            if rng.random() < 0.2:
                yield {"type": "last-prompt", "lastPrompt": self.text(120), "sessionId": sid}
                yield {"type": "file-history-snapshot", "messageId": self.uid(), "snapshot": {"files": self.text(300)}}
                yield {"type": "queue-operation", "operation": "enqueue", "content": self.text(80), "sessionId": sid}

    def tool(self) -> dict:
        rng = self.rng
        name = rng.choice(("Bash", "Read", "Edit", "Grep", "Glob", "Write", "TodoWrite", "Agent", "mcp__bench__lookup"))
        return {"type": "tool_use", "id": f"toolu_{self.uid().replace('-', '')[:24]}", "name": name,
                "input": {"command": self.text(60), "description": self.text(40)}}  # fmt: skip

    def file(self, path: Path, size: int, cwd: str) -> int:
        path.parent.mkdir(parents=True, exist_ok=True)
        written = lines = 0
        with open(path, "wb") as out:
            for rec in self.session(cwd, 1767225600 + self.rng.randrange(80 * 86400)):  # 2026-01-01 + < 80 days
                data = (json.dumps(rec, ensure_ascii=False, separators=(",", ":")) + "\n").encode()
                out.write(data)
                written, lines = written + len(data), lines + 1
                if written >= size:
                    break
            out.flush()
            os.fsync(out.fileno())  # clean pages, so posix_fadvise can drop them
        return lines


def datasets(root: Path) -> dict:
    """Make (or reuse) set A (`root/A/`) and set B (`root/B/`); return the manifest."""
    want = {"generator": GEN_VERSION, "seed": SEED, "a_bytes": A_BYTES, "b_bytes": B_BYTES}
    manifest = root / "manifest.json"
    if manifest.exists():
        have = json.loads(manifest.read_text())
        if {k: have.get(k) for k in want} == want and all(
            sum(p.stat().st_size for p in (root / s).rglob("*.jsonl")) == have[f"{s}_total"] for s in "AB"
        ):
            print(f"datasets: reused {root}")
            return have
    for s in "AB":
        if (root / s).exists():
            sys.exit(f"refused: {root / s} exists but does not match the manifest; move it away and run again")
    gen, lines, files, total = Gen(SEED), 0, 0, 0
    while total < A_BYTES:
        size = min(gen.rng.randint(MIB // 4, 5 * MIB), A_BYTES - total)
        project = gen.rng.randrange(12)
        path = root / "A" / f"-work-bench-{project:02d}" / f"{gen.uid()}.jsonl"
        lines += gen.file(path, size, f"/work/bench-{project:02d}")
        total += path.stat().st_size
        files += 1
    b_path = root / "B" / "-work-bench-big" / f"{gen.uid()}.jsonl"
    b_lines = gen.file(b_path, B_BYTES, "/work/bench-big")
    out = {**want, "A_total": total, "A_files": files, "A_lines": lines,
           "B_total": b_path.stat().st_size, "B_files": 1, "B_lines": b_lines}  # fmt: skip
    manifest.write_text(json.dumps(out, indent=2) + "\n")
    print(f"datasets: generated {root}")
    return out


def evict(folder: Path) -> None:
    for path in folder.rglob("*.jsonl"):
        fd = os.open(path, os.O_RDONLY)
        try:
            os.posix_fadvise(fd, 0, 0, os.POSIX_FADV_DONTNEED)
        finally:
            os.close(fd)


# -- runs ------------------------------------------------------------------------------------------------------


class Runner:
    def __init__(self, run_dir: Path, timeout: int) -> None:
        self.run_dir, self.timeout, self.count = run_dir, timeout, 0
        self.nowhere, self.agents = run_dir / "nowhere", run_dir / "agents"  # agents: empty, so no account row
        for folder in (self.nowhere, self.agents, run_dir / "logs"):
            folder.mkdir(parents=True)

    def argv(self, impl: str, *args: str) -> list[str]:
        if impl == "true":
            return ["/bin/true"]
        if impl == "rust":
            return [str(BINARY), *args]
        return [sys.executable, "-P", "-c", ENTRY, *args]

    def env(self, data: Path) -> dict:
        env = {
            "PATH": "/usr/bin:/bin", "HOME": str(self.nowhere), "XDG_CONFIG_HOME": str(self.nowhere / ".config"),
            "EASY_PERSONAL_HOME": str(data), "EASY_PERSONAL_AGENT_ROOT": str(self.agents),
            "EASY_PERSONAL_VERIFIER_REPORTS": str(self.run_dir / "verifier-none"),
            "EASY_PERSONAL_UI_REPORTS": str(self.run_dir / "ui-none"), "EASY_MEMORY_HOME": str(self.run_dir / "memory-none"),
            "EASY_PERSONAL_CONTEXT_URL": DEAD_CONTEXT, "EASY_PERSONAL_LOCK_TIMEOUT": "30",
            "TZ": "UTC", "LC_ALL": "C.UTF-8", "LANG": "C.UTF-8", "PYTHONHASHSEED": "0", "PYTHONPATH": str(REPO / "src"),
            "PYTHONDONTWRITEBYTECODE": "1", "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
        }  # fmt: skip
        for name in gen_golden.ENV_PATHS + ("HOME",):
            if name in env:
                refuse(f"child {name}", Path(env[name]))
        return env

    def run(self, impl: str, data: Path, *args: str) -> dict:
        """One child: wall seconds, its own peak RSS (MiB), exit code, parsed stdout; killed at the time limit.

        The peak comes from `/usr/bin/time -f %M` (its child's `ru_maxrss`), not from our own `wait4`: a forked child
        starts with its parent's high-water mark and Linux keeps it across `execve`, so `wait4` here would report this
        (large) script's RSS. `time` is small, so the program it forks starts from about 2 MiB (see `floor` below).
        """
        self.count += 1
        log = self.run_dir / "logs" / f"{self.count:02d}-{impl}-{'-'.join(a.strip('-') for a in args[:1])}"
        argv = [TIME, "-f", "%M", "-o", f"{log}.rss", *self.argv(impl, *args)]
        with open(f"{log}.out", "wb") as out, open(f"{log}.err", "wb") as err:
            start = time.perf_counter()
            proc = subprocess.Popen(argv, env=self.env(data), cwd=self.nowhere, stdin=subprocess.DEVNULL,
                                    stdout=out, stderr=err, start_new_session=True)  # fmt: skip
            timer = threading.Timer(self.timeout, os.killpg, (proc.pid, signal.SIGKILL))
            timer.start()
            code = proc.wait()
            secs = time.perf_counter() - start
            timer.cancel()
        text = Path(f"{log}.out").read_text()
        try:
            parsed = json.loads(text) if text.strip().startswith("{") else None
        except ValueError:
            parsed = None
        kib = Path(f"{log}.rss").read_text().split() if Path(f"{log}.rss").exists() else []
        rss = int(kib[-1]) / 1024 if kib and kib[-1].isdigit() else float("nan")
        timed_out = code == -signal.SIGKILL and secs >= self.timeout
        return {"secs": secs, "rss_mib": rss, "exit": code, "timed_out": timed_out, "out": parsed, "log": str(log)}

    def floor(self) -> float:
        """The peak `time` reports for `/bin/true`: what the measurement adds before the program's own memory."""
        return self.run("true", self.run_dir / "data-floor")["rss_mib"]


def tables(store: Path) -> tuple[str, dict]:
    """sha256 and row counts of the three tables, sorted, without id / machine_id / inode."""
    digest, counts = hashlib.sha256(), {}
    conn = sqlite3.connect(store)
    try:
        for table in TABLES:
            cur = conn.execute(f"SELECT * FROM {table}")
            keep = [i for i, d in enumerate(cur.description) if d[0] not in DROPPED]
            names = [cur.description[i][0] for i in keep]
            rows = sorted((tuple(r[i] for i in keep) for r in cur), key=repr)
            counts[table] = len(rows)
            digest.update(json.dumps([table, names, rows], ensure_ascii=False).encode())
    finally:
        conn.close()
    return digest.hexdigest(), counts


def machine(root: Path, run_dir: Path) -> dict:
    cpu = next((ln.split(":", 1)[1].strip() for ln in Path("/proc/cpuinfo").read_text().splitlines()
                if ln.startswith("model name")), "?")  # fmt: skip
    mem = next(ln.split(":", 1)[1].strip() for ln in Path("/proc/meminfo").read_text().splitlines()
               if ln.startswith("MemTotal"))  # fmt: skip
    mount, fstype, device = filesystem(root)
    git = subprocess.run(["git", "--version"], capture_output=True, text=True, timeout=30).stdout.strip()
    lock = (RUST / "Cargo.lock").read_text().split('name = "libsqlite3-sys"\nversion = "', 1)[1].split('"', 1)[0]
    return {"cpu": cpu, "cores": os.cpu_count(), "memory": mem, "kernel": platform.release(),
            "filesystem": f"{fstype} ({device} on {mount})", "fstype": fstype, "git": git,
            "sqlite_python": sqlite3.sqlite_version, "sqlite_rust": f"bundled by rusqlite (libsqlite3-sys {lock})",
            "python": platform.python_version(), "run_dir": str(run_dir)}  # fmt: skip


def median(values: list[float]) -> float:
    return statistics.median(values) if values else float("nan")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--root", type=Path, default=RUST / "target" / "bench")
    parser.add_argument("--impl", choices=("both", "rust", "python"), default="both")
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--timeout", type=int, default=900, help="seconds per child run")
    args = parser.parse_args()
    root = args.root.resolve()
    for name in gen_golden.ENV_PATHS:
        os.environ.pop(name, None)
    refuse("--root", root)
    gen_golden.guard_sqlite()
    root.mkdir(parents=True, exist_ok=True)
    free = shutil.disk_usage(root).free
    need = A_BYTES + B_BYTES + 2 * args.runs * 80 * MIB  # datasets + one store per cold run (about 40 MiB each)
    print(f"disk: {free / 1024**3:.1f} GiB free on {' '.join(filesystem(root))}; this run needs about {need / MIB:.0f} MiB")
    if free < MIN_FREE:
        sys.exit("refused: less than 1 GiB free on the chosen filesystem")
    if not Path(TIME).exists():
        sys.exit(f"no {TIME} (GNU time): it measures the peak RSS of each run")
    if args.impl != "python" and not BINARY.exists():
        sys.exit(f"no release binary at {BINARY}: run `cargo build --release` in rust/ first")
    sets = root / "datasets"
    sets.mkdir(exist_ok=True)
    manifest = datasets(sets)
    run_dir = root / "runs" / time.strftime("%Y%m%dT%H%M%SZ", time.gmtime())
    runner = Runner(run_dir, args.timeout)
    facts = machine(root, run_dir)
    own = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024
    facts["floor"] = f"{runner.floor():.1f} MiB for /bin/true under {TIME} (this script's own peak: {own:.0f} MiB)"
    if facts["fstype"] in ("tmpfs", "ramfs"):
        print("WARNING: the data is on a RAM-backed filesystem; the speed numbers would be inflated")
    impls = ["rust", "python"] if args.impl == "both" else [args.impl]
    results: dict = {}  # (set, impl) -> list of runs
    problems: list[str] = []
    digests: dict = {}  # (set, impl, i) -> (sha, counts)
    for impl in impls:
        base = [runner.run(impl, run_dir / "data-base", "--version") for _ in range(args.runs)]
        results[("baseline", impl)] = base
        for name in ("B", "A"):
            for i in range(args.runs):
                data = run_dir / f"data-{impl}-{name}-{i + 1}"
                evict(sets / name)
                cold = runner.run(impl, data, "ingest", "--source", str(sets / name))
                results.setdefault((name, impl), []).append(cold)
                store = data / "store.sqlite"
                if cold["exit"] == 0 and store.exists():
                    digests[(name, impl, i)] = tables(store)
                if name == "A":
                    results.setdefault(("C", impl), []).append(
                        runner.run(impl, data, "ingest", "--source", str(sets / name))
                    )
                print(f"  {impl} {name} run {i + 1}: {cold['secs']:.2f} s, {cold['rss_mib']:.1f} MiB, exit {cold['exit']}")

    # -- equality: every cold run against the first Python run (or the first Rust run with --impl rust)
    ref_impl = "python" if "python" in impls else "rust"
    equal = {}
    for name in ("A", "B"):
        ref = digests.get((name, ref_impl, 0))
        for (s, impl, i), (sha, counts) in sorted(digests.items()):
            if s == name:
                same = ref is not None and sha == ref[0]
                equal[(s, impl, i)] = same
                if not same:
                    problems.append(f"set {name} {impl} run {i + 1}: tables differ from {ref_impl} run 1 {counts}")
        runs_done = sum(1 for k in digests if k[0] == name)
        if runs_done < len(impls) * args.runs:
            problems.append(f"set {name}: {len(impls) * args.runs - runs_done} cold runs left no store to compare")

    # -- report
    print()
    print("## Machine")
    for key in ("cpu", "cores", "memory", "kernel", "filesystem", "git", "sqlite_python", "sqlite_rust", "python", "floor"):
        print(f"- {key}: {facts[key]}")
    print(f"- datasets: A {manifest['A_total']} bytes in {manifest['A_files']} files ({manifest['A_lines']} lines, "
          f"{manifest['A_total'] / manifest['A_lines']:.0f} B/line); B {manifest['B_total']} bytes in 1 file "
          f"({manifest['B_lines']} lines, {manifest['B_total'] / manifest['B_lines']:.0f} B/line); seed {SEED}")  # fmt: skip
    print("- page cache: dataset files dropped with posix_fadvise(DONTNEED) before each cold run (A, B); "
          "C reads the store and stats only")  # fmt: skip
    print(f"- runs: median of {args.runs}; MB = 2^20 bytes; peak RSS = the program's ru_maxrss as /usr/bin/time -f %M reports it")
    print()
    print("| run | impl | seconds (median) | MB/s | peak RSS MiB | above baseline MiB | new_events | each run (s) | tables |")
    print("|---|---|---|---|---|---|---|---|---|")
    summary: dict = {}
    for name, size in (("baseline", 0), ("B", manifest["B_total"]), ("A", manifest["A_total"]), ("C", 0)):
        for impl in impls:
            runs = results.get((name, impl), [])
            bad = [r for r in runs if r["exit"] != 0]
            for r in bad:
                why = "TIMEOUT" if r["timed_out"] else f"exit {r['exit']}"
                problems.append(f"{name} {impl}: {why} (log {r['log']})")
            ok = [r for r in runs if r["exit"] == 0]
            secs, rss = median([r["secs"] for r in ok]), median([r["rss_mib"] for r in ok])
            base = median([r["rss_mib"] for r in results.get(("baseline", impl), []) if r["exit"] == 0])
            events = sorted({r["out"].get("new_events") for r in ok if r["out"]})
            rate = f"{size / MIB / secs:.1f}" if size and ok else "-"
            check = "-"
            if name in ("A", "B"):
                flags = [equal.get((name, impl, i)) for i in range(len(runs))]
                check = "equal" if flags and all(flags) else "DIFFERENT/MISSING"
            above = f"{rss - base:.1f}" if name != "baseline" and ok else "-"
            each = ", ".join("TIMEOUT" if r["timed_out"] else f"{r['secs']:.2f}" for r in runs)
            ev = ",".join(map(str, events)) if events else "-"
            print(f"| {name} | {impl} | {secs:.2f} | {rate} | {rss:.1f} | {above} | {ev} | {each} | {check} |")
            summary[(name, impl)] = {"secs": secs, "rss": rss, "base": base, "size": size, "ok": len(ok) == len(runs)}

    print()
    print("## Gates (NFR-005)")
    failed = False
    for impl in impls:
        a, c, b = summary[("A", impl)], summary[("C", impl)], summary[("B", impl)]
        rate = a["size"] / MIB / a["secs"] if a["ok"] else float("nan")
        gates = [
            (f"cold ingest of A >= {GATE_RATE:g} MB/s", rate >= GATE_RATE, f"{rate:.1f} MB/s"),
            (f"unchanged re-run (C) < {GATE_RERUN_S:g} s", c["ok"] and c["secs"] < GATE_RERUN_S, f"{c['secs']:.2f} s"),
            (f"peak RSS of B above baseline < {GATE_MEM_MIB:g} MiB", b["ok"] and b["rss"] - b["base"] < GATE_MEM_MIB,
             f"{b['rss'] - b['base']:.1f} MiB ({b['rss']:.1f} - {b['base']:.1f})"),
        ]  # fmt: skip
        for label, passed, value in gates:
            print(f"- {impl}: {label}: {'PASS' if passed else 'FAIL'} ({value})")
            failed |= impl == "rust" and not passed
        print(f"- {impl}: memory of A (for growth with size): {a['rss'] - a['base']:.1f} MiB above baseline")
    used = sum(p.stat().st_size for p in root.rglob("*") if p.is_file())
    print()
    print(f"disk used under {root}: {used / MIB:.0f} MiB")
    for problem in problems:
        print(f"PROBLEM: {problem}")
    return 1 if failed or problems else 0


if __name__ == "__main__":
    sys.exit(main())
