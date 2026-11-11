#!/usr/bin/env python3
"""Ingest oracle (T026 / RS-003): run the real Python `easy-personal ingest` and freeze what it wrote, as goldens.

Every scenario runs the CLI entry (`easy_personal.surfaces.cli:main`, the `easy-personal` console script) in a child
process with fresh temp folders for `EASY_PERSONAL_HOME`, the agent root and the work root, over the committed
`tests/fixtures/claude_code/` and the invented set in `rust/parity/ingest/fixtures/` (made by
`make_fixtures_ingest.py`). It records the exit code, stdout, stderr and the store tables after each run, plus the
schema of a fresh v11 store, in `rust/parity/ingest/golden/`.

    rust/parity/.venv/bin/python rust/parity/gen_ingest.py           # (re)write rust/parity/ingest/golden/
    rust/parity/.venv/bin/python rust/parity/gen_ingest.py --check   # regenerate in a temp folder and compare: exit 0 equal, 1 different

It only reads: the app is imported from src/ and never changed; the fixtures are copied, never written. It refuses to
run when an env var points at the owner's real ~/.easy-personal, ~/.claude*, ~/.codex* (the gen_golden.py guard).
POSIX only (`fcntl`, symlinks, `git`).
"""

import argparse
import fcntl
import hashlib
import json
import os
import platform
import re
import shutil
import sqlite3
import subprocess
import sys
import sysconfig
import tempfile
from importlib import metadata
from pathlib import Path

sys.dont_write_bytecode = True  # importing gen_golden leaves no .pyc beside it
PARITY = Path(__file__).resolve().parent
REPO = PARITY.parents[1]
sys.path.insert(0, str(PARITY))
import gen_golden  # noqa: E402  (the real-home guard, the sqlite guard, the fixture-folder check, tree_sha256)

COMMITTED = REPO / "tests" / "fixtures" / "claude_code"
FIXTURES = PARITY / "ingest" / "fixtures"
GOLDEN = PARITY / "ingest" / "golden"
GENERATOR = "rust/parity/gen_ingest.py"
REPOS = json.loads((FIXTURES / "repos.json").read_text())
WORK_LEN = REPOS["work_path_length"]
PLACEHOLDER = REPOS["placeholder"].encode()
ENTRY = "import sys; from easy_personal.surfaces.cli import main; sys.exit(main())"  # what the console script runs
DEAD_CONTEXT = "http://127.0.0.1:9/mcp"  # easy-context URL with nothing behind it (as tests/conftest.py)
CHILD_TIMEOUT = 120
FIXED_ENV = {  # the child's environment besides the folders; nothing else of the caller's env is passed on
    "TZ": "UTC", "LC_ALL": "C.UTF-8", "LANG": "C.UTF-8", "PYTHONHASHSEED": "0", "PYTHONPATH": str(REPO / "src"),
    "PYTHONDONTWRITEBYTECODE": "1", "PYTHONIOENCODING": "utf-8", "NO_COLOR": "1", "PYTHON_COLORS": "0",
    "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null",
}  # fmt: skip
LEAK = re.compile(rb"/home/|/Users/|@|ghp_|\bsk-(?!FAKE)|AKIA(?!FAKE)|-----BEGIN|\bxox[a-z]-")
CANARIES = (b"FAKE", b"INVENTED")  # planted in the fixtures' text; never in a golden
TOKEN_RUN = re.compile(rb"[A-Za-z0-9]{32,}")  # token-shaped text; the manifest's sha256 values are exempt
VENV, STDLIB = sys.prefix, sysconfig.get_paths()["stdlib"]
VOLATILE = [  # (pattern, replacement, why): text of a run that differs between two runs on one machine
    (re.compile(r"Stack overflow \(used \d+ kB\)"), "Stack overflow (used <N> kB)",
     "CPython's RecursionError message names the C stack it used; the figure changes from run to run (S06a)"),
]  # fmt: skip

PATH_NOTE = "absolute path of this run, tokenised (<HOME>, <AGENT_ROOT>, <WORK>, <CASE>)"
INODE_NOTE = (
    "<INODE_OF_FILE>: equals the inode of the file at that path after the run; <INODE_OTHER>: another inode "
    "(the file was replaced); <INODE_FILE_GONE>: no file at that path any more"
)
MACHINE_NOTE = "random uuid4 per store (meta.machine_id); machine_id_check says every row carries meta's value"
TABLES = {  # table -> (sort key, dropped columns, replaced columns); None = rows in id order
    "events": (
        ("source", "source_path", "inode", "byte_offset"),
        {"id": "row id (insertion order)", "machine_id": MACHINE_NOTE},
        {"source_path": PATH_NOTE, "inode": INODE_NOTE,
         "agent_account_id": "the account's folder (tokenised) instead of its row id; null stays null"},
    ),
    "tool_signals": (
        ("source_path", "inode", "byte_offset", "kind", "value"),
        {"machine_id": MACHINE_NOTE},
        {"source_path": PATH_NOTE, "inode": INODE_NOTE},
    ),
    "source_files": (("source", "path"), {"machine_id": MACHINE_NOTE}, {"path": PATH_NOTE, "inode": INODE_NOTE}),
    "agent_accounts": (
        None,
        {"id": "row id; rows are listed in id order and events name the account by its folder"},
        {"folder": PATH_NOTE, "config_dir_env": PATH_NOTE},
    ),
}  # fmt: skip


# -- guards ----------------------------------------------------------------------------------------------------


def refuse(name: str, value: Path) -> None:
    """Exit 1 when `value` is, or is inside, a real home's .claude* / .codex* / .easy-* folder, or is a real home."""
    target = value.expanduser().resolve()
    for home in gen_golden._real_homes():
        if target == home:
            sys.exit(f"refused: {name} is the real home folder (NFR-003); nothing was opened")
        if home in target.parents and target.relative_to(home).parts[0].startswith(gen_golden.PROTECTED_PREFIXES):
            sys.exit(f"refused: {name} points inside the real {target.relative_to(home).parts[0]} (NFR-003)")


def leak_scan(name: str, data: bytes, golden: bool) -> None:
    """No home path, no email, no token-shaped text; a golden also holds none of the planted canaries."""
    hit = LEAK.search(data)
    if hit:
        raise RuntimeError(f"{name}: home path, email or secret-like text: {hit.group()!r}")
    if golden:
        for canary in CANARIES:
            if canary in data:
                raise RuntimeError(f"{name}: holds the planted text {canary!r}")
        if name != "manifest.json" and (run := TOKEN_RUN.search(data)):
            raise RuntimeError(f"{name}: token-shaped text {run.group()[:12]!r}...")


# -- one throwaway case ----------------------------------------------------------------------------------------


def git_version() -> str:
    out = subprocess.run(["git", "--version"], capture_output=True, text=True, timeout=30, check=True)
    return out.stdout.strip()


class Case:
    """Fresh folders for one scenario: the data dir, the agent root, the work root (padded to WORK_LEN), a fake HOME."""

    def __init__(self, run_root: Path, name: str) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="c-", dir=run_root)).resolve()  # short: the work root is padded
        self.home = self.root / "home"  # not created: the CLI creates it (mode 0700) as on a first run
        self.agents = self.root / "agents"
        self.projects = self.agents / ".claude" / "projects"
        self.nowhere = self.root / "nowhere"
        self.verifier = self.root / "verifier-none"
        self.memory = self.root / "memory-none"
        pad = WORK_LEN - len(str(self.root)) - 1
        if pad < 4:
            sys.exit(f"refused: the temp folder path is too long for a {WORK_LEN}-character work root; set TMPDIR=/tmp")
        self.work = self.root / ("w" * pad)
        for folder in (self.agents, self.nowhere, self.verifier, self.memory, self.work):
            folder.mkdir(parents=True)
        pairs = [(self.work, "<WORK>"), (self.agents, "<AGENT_ROOT>"), (self.home, "<HOME>"), (self.root, "<CASE>"),
                 (VENV, "<VENV>"), (STDLIB, "<STDLIB>"), (REPO, "<REPO>")]  # fmt: skip
        self.tokens = sorted(((str(p), t) for p, t in pairs), key=lambda pair: -len(pair[0]))
        self.setup: list[str] = []
        self.runs: list[dict] = []

    def tok(self, text: str) -> str:
        for needle, token in self.tokens:
            text = text.replace(needle, token)
        return text

    def tok_obj(self, value):
        if isinstance(value, str):
            return self.tok(value)
        if isinstance(value, list):
            return [self.tok_obj(v) for v in value]
        if isinstance(value, dict):
            return {k: self.tok_obj(v) for k, v in value.items()}
        return value

    def env(self, extra: dict | None = None) -> dict:
        env = {
            "PATH": os.environ.get("PATH", "/usr/bin:/bin"), "HOME": str(self.nowhere),
            "XDG_CONFIG_HOME": str(self.nowhere / ".config"), "EASY_PERSONAL_HOME": str(self.home),
            "EASY_PERSONAL_AGENT_ROOT": str(self.agents), "EASY_PERSONAL_VERIFIER_REPORTS": str(self.verifier),
            "EASY_MEMORY_HOME": str(self.memory), "EASY_PERSONAL_CONTEXT_URL": DEAD_CONTEXT, **FIXED_ENV,
        }  # fmt: skip
        env.update(extra or {})
        return env

    # -- inputs

    def copy(self, source: Path, target: Path, note: str) -> None:
        """Copy a fixture folder or file; `%WORK%` in a file becomes the work root (the originals are only read)."""
        files = [source] if source.is_file() else sorted(p for p in source.rglob("*") if p.is_file())
        for path in files:
            dest = target if source.is_file() else target / path.relative_to(source)
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(path.read_bytes().replace(PLACEHOLDER, str(self.work).encode()))
        self.note(f"copy {note} -> {self.tok(str(target))}")

    def apply(self, ops: list[dict]) -> None:
        """Run layout steps of repos.json (mkdir, rmdir, write, symlink, git) with this case's folders."""
        names = {"<WORK>": self.work, "<PROJECTS>": self.projects, "<AGENT_ROOT>": self.agents}

        def real(text: str) -> str:
            for token, path in names.items():
                text = text.replace(token, str(path))
            return text

        git_env = {**REPOS["git_env"], "PATH": os.environ.get("PATH", "/usr/bin:/bin"), "HOME": str(self.nowhere)}
        for op in ops:
            path = Path(real(op.get("path", op.get("cwd", ""))))
            if op["op"] == "mkdir":
                path.mkdir(parents=True)
            elif op["op"] == "rmdir":
                path.rmdir()
            elif op["op"] == "write":
                path.write_text(op["text"])
            elif op["op"] == "symlink":
                path.parent.mkdir(parents=True, exist_ok=True)
                os.symlink(real(op["target"]), path)
            elif op["op"] == "git":
                subprocess.run(["git", "-C", str(path), *[real(a) for a in op["args"]]], env=git_env,
                               capture_output=True, timeout=30, check=True)  # fmt: skip
            else:
                raise RuntimeError(f"unknown layout op {op['op']}")
            self.note(f"{op['op']} " + " ".join(f"{k}={v}" for k, v in sorted(op.items()) if k != "op"))

    def note(self, text: str) -> None:
        self.setup.append(text)

    # -- running the CLI

    def run(self, label: str, argv: list[str], extra_env: dict | None = None) -> dict:
        """Run `easy-personal <argv>` in a child process (cwd <WORK>); record what it printed and what it stored."""
        cmd = [sys.executable, "-P", "-c", ENTRY, *argv]
        result = {"label": label, "argv": ["easy-personal", *self.tok_obj(argv)], "env_extra": extra_env or {}}
        try:
            proc = subprocess.run(cmd, cwd=self.work, env=self.env(extra_env), capture_output=True, timeout=CHILD_TIMEOUT)
        except subprocess.TimeoutExpired:
            result.update(timed_out=True, exit_code=None)
        else:
            out, err = proc.stdout.decode("utf-8"), proc.stderr.decode("utf-8", "backslashreplace")
            for pattern, replacement, _ in VOLATILE:
                err = pattern.sub(replacement, err)
            parsed = None
            try:
                parsed = json.loads(out) if out else None
            except ValueError:
                pass
            result.update(
                timed_out=False, exit_code=proc.returncode,
                stdout_json=self.tok_obj(parsed),
                stdout_equals_json_dumps=parsed is not None and json.dumps(parsed, indent=2, sort_keys=True) + "\n" == out,
                stdout_text=None if parsed is not None else self.tok(out),
                stderr_text_tokenised=self.tok(err),
                exception=self.exception(err),
            )  # fmt: skip
        result["home_files"] = sorted(p.name for p in self.home.iterdir()) if self.home.is_dir() else None
        result["home_mode"] = oct(self.home.stat().st_mode & 0o777) if self.home.is_dir() else None
        result["tables_after"] = self.dump()
        self.runs.append(result)
        return result

    def exception(self, err: str) -> dict | None:
        if "Traceback (most recent call last):" not in err:
            return None
        last = [line for line in err.splitlines() if line.strip()][-1]
        return {"class": last.split(":", 1)[0], "line": self.tok(last)}

    def snapshot(self, label: str) -> None:
        """Tables as they are now (after a setup step of the generator), between two runs."""
        self.runs.append({"label": label, "tables_after": self.dump()})

    # -- reading the store

    def inode_token(self, path: str, inode: int) -> str:
        try:
            return "<INODE_OF_FILE>" if os.stat(path).st_ino == inode else "<INODE_OTHER>"
        except FileNotFoundError:
            return "<INODE_FILE_GONE>"

    def dump(self) -> dict | None:
        path = self.home / "store.sqlite"
        if not path.exists():
            return None
        if self.root not in path.parents:
            raise RuntimeError("refused to read a store outside the case folder")
        conn = sqlite3.connect(path)
        try:
            return self._dump(conn)
        finally:
            conn.close()

    def _dump(self, conn: sqlite3.Connection) -> dict:
        tables = [r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")]
        out = {
            "user_version": conn.execute("PRAGMA user_version").fetchone()[0],
            "row_counts": {t: conn.execute(f"SELECT COUNT(*) FROM {t}").fetchone()[0] for t in tables},
        }
        if "meta" not in tables:
            return out
        meta = conn.execute("SELECT value FROM meta WHERE key = 'machine_id'").fetchone()
        folders = dict(conn.execute("SELECT id, folder FROM agent_accounts")) if "agent_accounts" in tables else {}
        out["machine_id_check"] = {}
        for table, (order, dropped, replaced) in TABLES.items():
            if table not in tables:
                continue
            cur = conn.execute(f"SELECT * FROM {table} ORDER BY rowid")
            columns = [d[0] for d in cur.description]
            rows, same_machine = [], True
            for raw in cur:
                row = dict(zip(columns, raw))
                if "machine_id" in row:
                    same_machine &= meta is not None and row["machine_id"] == meta[0]
                if "inode" in row:
                    row["inode"] = self.inode_token(row.get("source_path", row.get("path")), row["inode"])
                if table == "events" and row["agent_account_id"] is not None:
                    row["agent_account_id"] = folders[row["agent_account_id"]]
                for column in dropped:
                    row.pop(column)
                rows.append(self.tok_obj(row))
            if "machine_id" in columns:
                out["machine_id_check"][table] = same_machine
            if order:
                key = lambda r: tuple((r[c] is None, "" if r[c] is None else r[c]) for c in order)  # noqa: E731
                rows.sort(key=key)
                keys = [key(r) for r in rows]
                if len(set(keys)) != len(keys):
                    raise RuntimeError(f"{table}: two rows share the sort key {order}")
            out[table] = {
                "columns": [c for c in columns if c not in dropped],
                "order": list(order) if order else "id",
                "dropped": dropped,
                "replaced": replaced,
                "rows": rows,
            }
        return out

    def result(self, scenario: str, what: str) -> dict:
        return {"scenario": scenario, "what": what, "setup": self.setup, "runs": self.runs}


# -- scenarios -------------------------------------------------------------------------------------------------


def ingest(case: Case, label: str, *args: str, env: dict | None = None) -> dict:
    return case.run(label, ["ingest", *args], env)


def committed(case: Case, target: Path | None = None) -> None:
    case.copy(COMMITTED, target or case.projects, "tests/fixtures/claude_code")


def s01(case):
    committed(case)
    ingest(case, "cold ingest")
    return "cold ingest of the committed fixtures (default account <AGENT_ROOT>/.claude, detected by the CLI)"


def s02(case):
    committed(case)
    ingest(case, "first run")
    ingest(case, "second run, nothing changed (expect new_events 0)")
    return "the same ingest twice"


def s03(case):
    case.copy(FIXTURES / "s3" / "session.jsonl", case.projects / "s3" / "session.jsonl", "ingest/fixtures/s3/session.jsonl")
    ingest(case, "first run")
    with open(case.projects / "s3" / "session.jsonl", "ab") as handle:
        handle.write((FIXTURES / "s3" / "append.jsonl").read_bytes())
    case.note("append ingest/fixtures/s3/append.jsonl (one line, entrypoint cli) to <AGENT_ROOT>/.claude/projects/s3/session.jsonl")
    ingest(case, "second run after one line was appended (incremental; the earlier rows take the session's entrypoint)")
    return "a line appended to one file"


RESET_SQL = "UPDATE events SET entrypoint = NULL, tool_error_count = NULL; UPDATE source_files SET byte_offset = 0;"


def s04(case):
    committed(case)
    ingest(case, "first run")
    conn = sqlite3.connect(case.home / "store.sqlite")
    with conn:
        conn.executescript(RESET_SQL)
    conn.close()
    case.note(f"generator SQL on the store (what migrations 7 and 8 left behind): {RESET_SQL}")
    case.snapshot("after the reset, before the second run")
    ingest(case, "second run from offset 0 (the back-fill path: no new row, entrypoint and tool_error_count filled)")
    return "offsets reset to 0: the conditional upsert back-fills entrypoint and tool_error_count"


def s05(case):
    target = case.projects / "s5" / "session.jsonl"
    case.copy(FIXTURES / "s5" / "session.jsonl", target, "ingest/fixtures/s5/session.jsonl")
    ingest(case, "first run (the last line has no newline yet)")
    with open(target, "ab") as handle:
        handle.write((FIXTURES / "s5" / "rest.part").read_bytes())
    case.note("append ingest/fixtures/s5/rest.part (the rest of the last line and its newline)")
    ingest(case, "second run after the last line was completed")
    return "a file whose last line was truncated at the first run and completed at the second"


def s06(case):
    if subprocess.run(["git", "-C", str(case.work), "rev-parse", "--show-toplevel"], capture_output=True,
                      env={**REPOS["git_env"], "PATH": os.environ.get("PATH", "")}, timeout=30).returncode == 0:  # fmt: skip
        sys.exit("refused: the temp folder is inside a git repo; the project-resolution cases would differ (set TMPDIR)")
    case.apply(REPOS["work"])
    case.copy(FIXTURES / "synthetic", case.projects, "ingest/fixtures/synthetic")
    case.copy(FIXTURES / "outside", case.agents / "outside", "ingest/fixtures/outside")
    case.apply(REPOS["synthetic"])
    ingest(case, "cold ingest of the synthetic set")
    return "cold ingest of the synthetic set (every edge case that fits in a transcript; repos.json layout under <WORK>)"


def crash_scenario(folder: str, what: str):
    def scenario(case):
        case.copy(FIXTURES / folder, case.projects, f"ingest/fixtures/{folder}")
        if folder == "crash-dir":
            case.apply(REPOS["crash-dir"])
        ingest(case, "first run")
        ingest(case, "second run (does it get past the file?)")
        return what

    return scenario


def s07(case):
    for entry in sorted((FIXTURES / "s7" / "agents").iterdir()):
        case.copy(entry, case.agents / f".{entry.name}", f"ingest/fixtures/s7/agents/{entry.name}")
    case.run("setup: confirm the work account the way the owner does (the CLI detects, then confirms)",
             ["account", "confirm", str(case.agents / ".claude-work"), "--label", "Claude work"])  # fmt: skip
    ingest(case, "ingest of every confirmed Claude account")
    return "two confirmed accounts (.claude by detection, .claude-work by `account confirm`), one detected only, a Codex folder"


def s08a(case):
    committed(case)
    ingest(case, "ingest with --retention-days 36500 (cutoff a century ago: nothing pruned)", "--retention-days", "36500")
    return "retention that prunes nothing"


def s08b(case):
    committed(case)
    ingest(case, "ingest with --retention-days 1 (every fixture date is older: all timestamped rows pruned)", "--retention-days", "1")
    return "retention that prunes every row with a timestamp; NULL-timestamp rows are kept"


def s08c(case):
    committed(case)
    ingest(case, "ingest with --retention-days 0 (argparse refuses it)", "--retention-days", "0")
    return "a retention value the CLI refuses"


def s09(case):
    committed(case)
    case.home.mkdir(mode=0o700)
    case.home.chmod(0o700)
    with open(case.home / "ingest.lock", "a") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX)
        case.note("generator: mkdir <HOME> (0700), open <HOME>/ingest.lock for append, flock LOCK_EX held during the run")
        ingest(case, "ingest while the lock is held, EASY_PERSONAL_LOCK_TIMEOUT=0.5", env={"EASY_PERSONAL_LOCK_TIMEOUT": "0.5"})
        fcntl.flock(handle, fcntl.LOCK_UN)
    return "a busy lock: exit 75 and the exact stderr line, nothing read or stored"


def s10a(case):
    case.copy(FIXTURES / "s3" / "session.jsonl", case.projects / "own" / "session.jsonl", "ingest/fixtures/s3/session.jsonl")
    committed(case, case.work / "override")
    ingest(case, "ingest --source <WORK>/override (replaces the default account's folder)", "--source", str(case.work / "override"))
    return "--source PATH with a default account: the override is read and tagged with it; the account's own folder is not read"


def s10b(case):
    committed(case, case.work / "override")
    ingest(case, "ingest --source <WORK>/override with no account folder", "--source", str(case.work / "override"))
    return "--source PATH with no account: the override is read untagged (agent_account_id null)"


def s11(case):
    ingest(case, "setup: first run on an empty agent root (creates a v11 store)")
    conn = sqlite3.connect(case.home / "store.sqlite")
    conn.execute("PRAGMA user_version = 12")
    conn.close()
    case.note("generator SQL: PRAGMA user_version = 12")
    committed(case)
    ingest(case, "ingest on a store newer than the program")
    return "a store with user_version 12: Python refuses"


def s12(case):
    from easy_personal.core import store
    from easy_personal.core.migrations import MIGRATIONS

    case.home.mkdir(mode=0o700)
    case.home.chmod(0o700)
    path = case.home / "store.sqlite"
    conn = sqlite3.connect(path)
    store.migrate(conn, path, MIGRATIONS[:10])
    conn.close()
    case.note("generator (in-process): mkdir <HOME> (0700); store.migrate(conn, path, MIGRATIONS[:10]) on a new file: a v10 store with no row")
    case.snapshot("the v10 store before the run")
    committed(case)
    ingest(case, "ingest on a v10 store (Python backs it up, migrates to 11, then ingests)")
    return "an older store (user_version 10): Python migrates it, keeping <store>.bak-v10"


def s13(case):
    target = case.projects / "s13" / "session.jsonl"
    case.copy(FIXTURES / "s3" / "session.jsonl", target, "ingest/fixtures/s3/session.jsonl")
    ingest(case, "first run")
    copy = target.with_name("copy.tmp")
    copy.write_bytes(target.read_bytes())
    os.replace(copy, target)
    case.note("generator: write a byte-identical copy beside the file and rename it over the file (same path, new inode)")
    ingest(case, "second run after the file was replaced by a copy")
    return "a file replaced by an identical copy (new inode): Python reads it again from offset 0"


def s14(case):
    case.copy(FIXTURES / "s3" / "session.jsonl", case.projects / "s14" / "a.jsonl", "ingest/fixtures/s3/session.jsonl")
    case.copy(FIXTURES / "s3" / "append.jsonl", case.projects / "s14" / "b.jsonl", "ingest/fixtures/s3/append.jsonl")
    ingest(case, "first run")
    (case.projects / "s14" / "b.jsonl").unlink()
    case.note("generator: delete <AGENT_ROOT>/.claude/projects/s14/b.jsonl (cleanup of an old transcript)")
    ingest(case, "second run after one file was deleted")
    return "a file deleted between runs: its offset row is retired, its events stay"


SCENARIOS = [  # (id, function)
    ("S01_cold_committed", s01),
    ("S02_rerun_unchanged", s02),
    ("S03_line_appended", s03),
    ("S04_offsets_reset_backfill", s04),
    ("S05_truncated_then_completed", s05),
    ("S06_cold_synthetic", s06),
    ("S06a_crash_deep_nesting", crash_scenario("crash-deep", "a line of 100,000 [: does the run survive, which files are stored")),
    ("S06b_crash_int_beyond_int64", crash_scenario("crash-bigint", "a usage field of 2**63: does the run survive, which files are stored")),
    ("S06c_crash_directory_named_jsonl", crash_scenario("crash-dir", "a folder named x.jsonl: does the run survive, which files are stored")),
    ("S06d_crash_unhashable_type", crash_scenario("crash-type", "a record whose type is a list: does the run survive, which files are stored")),
    ("S07_two_accounts", s07),
    ("S08a_retention_36500", s08a),
    ("S08b_retention_1", s08b),
    ("S08c_retention_0_refused", s08c),
    ("S09_lock_busy", s09),
    ("S10a_source_override_default_account", s10a),
    ("S10b_source_override_no_account", s10b),
    ("S11_store_newer_v12", s11),
    ("S12_store_older_v10", s12),
    ("S13_file_replaced_new_inode", s13),
    ("S14_file_deleted", s14),
]


# -- the fresh v11 schema --------------------------------------------------------------------------------------


def literal(value) -> str:
    if value is None:
        return "NULL"
    if isinstance(value, int):
        return str(value)
    return "'" + str(value).replace("'", "''") + "'"


def schema_snapshot(run_root: Path) -> tuple[dict, str]:
    """`store.opened` on a new file; every sqlite_master row by name, seed rows, the pragmas. Proven re-creatable."""
    from easy_personal.core import store

    path = Path(tempfile.mkdtemp(prefix="schema-", dir=run_root)) / "store.sqlite"
    with store.opened(path) as conn:
        master = [list(r) for r in conn.execute("SELECT type, name, tbl_name, sql FROM sqlite_master ORDER BY name")]
        pragmas = {p: conn.execute(f"PRAGMA {p}").fetchone()[0] for p in ("user_version", "journal_mode", "foreign_keys", "busy_timeout")}
        tables = [name for kind, name, _, _ in master if kind == "table"]
        seeds = {}
        for table in tables:
            cur = conn.execute(f"SELECT * FROM {table} ORDER BY 1")  # some tables are WITHOUT ROWID
            columns = [d[0] for d in cur.description]
            rows = [list(r) for r in cur]
            if table == "meta":
                rows = [r for r in rows if r[0] != "machine_id"]
            if rows:
                seeds[table] = {"columns": columns, "rows": rows}
        meta_keys = [r[0] for r in conn.execute("SELECT key FROM meta ORDER BY key")]
    if meta_keys != ["machine_id"]:
        raise RuntimeError(f"a fresh store's meta holds {meta_keys}, not just machine_id: update the snapshot")
    lines = [
        "-- Schema v11 of a fresh Python store (`store.opened` on a new file), frozen by rust/parity/gen_ingest.py (T026).",
        "-- Executable as one script on an empty file. One block per sqlite_master row, sorted by name:",
        "-- `-- <type> <name> ON <tbl_name>`, then the SQL text SQLite stored, verbatim, plus `;`.",
        "-- Rows with no SQL (sqlite_autoindex_*) are comments only: SQLite makes them for PRIMARY KEY / UNIQUE.",
        f"-- Python sets on every connection: PRAGMA busy_timeout = {pragmas['busy_timeout']}; PRAGMA foreign_keys = ON "
        f"(reads {pragmas['foreign_keys']}); PRAGMA journal_mode = WAL (reads {pragmas['journal_mode']}; kept in the file once set).",
        "-- meta.machine_id is a random uuid4 that store.connect writes on the first open: not in this file.",
        "",
    ]
    for kind, name, table, sql in master:
        lines.append(f"-- {kind} {name} ON {table}")
        if sql is not None:
            lines.append(sql + ";")
        lines.append("")
    lines.append("-- seed rows")
    for table, seed in seeds.items():
        for row in seed["rows"]:
            lines.append(f"INSERT INTO {table} ({', '.join(seed['columns'])}) VALUES ({', '.join(literal(v) for v in row)});")
    lines += ["", f"PRAGMA user_version = {pragmas['user_version']};", ""]
    sql_text = "\n".join(lines)
    # proof: the script alone makes the same sqlite_master, seed rows and user_version
    check = path.parent / "from-script.sqlite"
    copy = sqlite3.connect(check)
    copy.executescript(sql_text)
    again = [list(r) for r in copy.execute("SELECT type, name, tbl_name, sql FROM sqlite_master ORDER BY name")]
    seeds_again = {t: [list(r) for r in copy.execute(f"SELECT * FROM {t} ORDER BY 1")] for t in seeds}
    version_again = copy.execute("PRAGMA user_version").fetchone()[0]
    copy.close()
    if again != master or seeds_again != {t: s["rows"] for t, s in seeds.items()} or version_again != pragmas["user_version"]:
        raise RuntimeError("schema-v11.sql does not re-create the fresh store's sqlite_master")
    snapshot = {
        "note": "a fresh Python store after store.opened: sqlite_master rows sorted by name as [type, name, tbl_name, sql], "
        "seed rows (meta.machine_id excluded: random), and the pragmas read on Python's connection. "
        "schema-v11.sql re-creates the same sqlite_master (checked by the generator).",
        "sqlite_master": master,
        "seed_rows": seeds,
        "pragmas": pragmas,
        "meta_excluded": {"machine_id": "random uuid4 written by store.connect on the first open"},
    }
    return snapshot, sql_text


# -- writing ---------------------------------------------------------------------------------------------------


def render(obj) -> str:
    """JSON, 2-space indent, sorted keys; each table row on one line (rows are many, and diff better that way)."""
    rows: list[str] = []

    def mark(value, key=None):
        if isinstance(value, dict):
            return {k: mark(v, k) for k, v in value.items()}
        if isinstance(value, list):
            if key == "rows" and all(isinstance(r, dict) for r in value):
                marked = []
                for row in value:
                    rows.append(json.dumps(row, sort_keys=True, ensure_ascii=False))
                    marked.append(f"\x01ROW{len(rows) - 1}\x01")
                return marked
            return [mark(v) for v in value]
        return value

    text = json.dumps(mark(obj), indent=2, sort_keys=True, ensure_ascii=False)
    for number, row in enumerate(rows):
        text = text.replace(json.dumps(f"\x01ROW{number}\x01"), row, 1)
    if json.loads(text) != obj:
        raise RuntimeError("render changed the content")
    return text + "\n"


def versions() -> dict:
    return {
        "python": platform.python_version(),
        "sqlite": sqlite3.sqlite_version,
        "git": git_version(),
        "easy-personal-mcp": metadata.version("easy-personal-mcp"),
        "platform": platform.system(),
    }


def check_fixture_types() -> None:
    """The synthetic set names every METADATA_TYPES entry of the app (a new type means: re-make the fixtures)."""
    from easy_personal.sources import claude_code

    sys.path.insert(0, str(PARITY))
    import make_fixtures_ingest

    if set(make_fixtures_ingest.METADATA_TYPES) != claude_code.METADATA_TYPES:
        raise RuntimeError("make_fixtures_ingest.METADATA_TYPES differs from the app: re-make the fixtures")


def generate(out: Path, run_root: Path) -> list[str]:
    """Write every golden into `out`; returns the file names."""
    import easy_personal

    if Path(easy_personal.__file__).resolve().parent != REPO / "src" / "easy_personal":
        raise RuntimeError(f"easy_personal imports from {easy_personal.__file__}, not src/")
    check_fixture_types()
    for path in sorted(FIXTURES.rglob("*")) + sorted(COMMITTED.rglob("*")):
        if path.is_file():
            leak_scan(str(path.relative_to(REPO)), path.read_bytes(), golden=False)
    out.mkdir(parents=True)
    texts: dict[str, str] = {}
    snapshot, sql_text = schema_snapshot(run_root)
    texts["schema-v11.sql"], texts["schema-v11.json"] = sql_text, render(snapshot)
    scenarios = []
    for scenario_id, function in SCENARIOS:
        case = Case(run_root, scenario_id)
        what = function(case)
        texts[f"{scenario_id}.json"] = render(case.result(scenario_id, what))
        scenarios.append({"id": scenario_id, "file": f"{scenario_id}.json", "what": what,
                          "exit_codes": [r["exit_code"] for r in case.runs if "exit_code" in r]})  # fmt: skip
        if any(str(run_root) in text for text in texts.values()):
            raise RuntimeError(f"{scenario_id}: a temp path survived the tokenising")
    probe = Case(run_root, "env")
    manifest = {
        "generator": GENERATOR,
        "note": "Python `easy-personal ingest` runs frozen as goldens (T026). Per scenario file: setup steps, then runs; "
        "each run has argv, exit_code, stdout_json, stderr_text_tokenised, exception, home_files, tables_after. "
        "Object keys are sorted; rows are sorted by the key in `order`.",
        "versions": versions(),
        "fixtures_sha256": {
            "tests/fixtures/claude_code": gen_golden.tree_sha256(COMMITTED),
            "rust/parity/ingest/fixtures": gen_golden.tree_sha256(FIXTURES),
        },
        "work_path_length": WORK_LEN,
        "child": {
            "entry": ENTRY,
            "cwd": "<WORK>",
            "env": {k: ("<PATH of the caller>" if k == "PATH" else probe.tok(v)) for k, v in sorted(probe.env().items())},
            "unset": ["EASY_PERSONAL_CLAUDE_PROJECTS", "EASY_PERSONAL_UI_REPORTS", "EASY_PERSONAL_LOCK_TIMEOUT (but S09)"],
        },
        "tokens": {
            "<HOME>": "EASY_PERSONAL_HOME (the data dir; the CLI creates it)",
            "<AGENT_ROOT>": "EASY_PERSONAL_AGENT_ROOT; the default account is <AGENT_ROOT>/.claude",
            "<WORK>": f"the work root, exactly {WORK_LEN} characters; the ingest process cwd",
            "<CASE>": "the scenario's temp folder (parent of the three above)",
            "<REPO>": "this checkout", "<VENV>": "sys.prefix of the parity venv", "<STDLIB>": "the Python stdlib folder",
        },  # fmt: skip
        "volatile": [{"pattern": p.pattern, "token": r, "reason": why} for p, r, why in VOLATILE],
        "scenarios": scenarios,
        "files": {name: hashlib.sha256(text.encode("utf-8")).hexdigest() for name, text in sorted(texts.items())},
    }
    texts["manifest.json"] = json.dumps(manifest, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    for name, text in sorted(texts.items()):
        data = text.encode("utf-8")
        leak_scan(name, data, golden=True)
        (out / name).write_bytes(data)
    return sorted(texts)


def isolate(run_root: Path) -> None:
    """The generator process itself: no real home reachable, the app from src/, stores only under the run folder."""
    gen_golden.guard_sqlite()  # before HOME changes: it lists the real home's stores
    os.umask(0o022)
    nowhere = run_root / "nowhere"
    nowhere.mkdir()
    for name in gen_golden.ENV_PATHS + ("EASY_PERSONAL_LOCK_TIMEOUT", "EASY_PERSONAL_CONTEXT_URL"):
        os.environ.pop(name, None)
    os.environ.update(HOME=str(nowhere), TZ="UTC", EASY_PERSONAL_HOME=str(nowhere / "home"))
    sys.path.insert(0, str(REPO / "src"))


def compare(new: Path, old: Path) -> list[str]:
    names = {p.name for p in new.iterdir()} | ({p.name for p in old.iterdir()} if old.is_dir() else set())
    diffs = []
    for name in sorted(names):
        a, b = new / name, old / name
        if not b.is_file():
            diffs.append(f"missing in the committed goldens: {name}")
        elif not a.is_file():
            diffs.append(f"not made by this run: {name}")
        elif a.read_bytes() != b.read_bytes():
            diffs.append(f"differs: {name}")
    return diffs


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--check", action="store_true", help="regenerate in a temp folder, compare with the committed goldens")
    args = parser.parse_args()
    gen_golden.refuse_real_paths(GOLDEN)  # env vars, and the output folder
    gen_golden.refuse_fixture_folder()  # the committed fixtures' cwd /work/demo-app must not exist
    base = Path(tempfile.gettempdir()).resolve()
    refuse("TMPDIR", base)
    if str(base).startswith("/mnt/"):
        sys.exit("refused: the temp folder is on /mnt (WSL drive: flock may not hold); set TMPDIR=/tmp")
    if GOLDEN.exists() and not args.check:
        try:
            ours = json.loads((GOLDEN / "manifest.json").read_text())["generator"] == GENERATOR
        except (OSError, ValueError, LookupError, TypeError):
            ours = False
        if not ours:
            sys.exit(f"refused: {GOLDEN} was not made by this generator; nothing was deleted")
    with tempfile.TemporaryDirectory(prefix="ingest-oracle-", dir=base) as tmp:
        run_root = Path(tmp).resolve()
        isolate(run_root)
        new = run_root / "golden"
        names = generate(new, run_root)
        if args.check:
            diffs = compare(new, GOLDEN)
            for line in diffs:
                print(line)
            print(f"--check: {len(names)} files, {len(diffs)} different")
            sys.exit(1 if diffs else 0)
        GOLDEN.mkdir(parents=True, exist_ok=True)
        old = set(json.loads((GOLDEN / "manifest.json").read_text())["files"]) | {"manifest.json"} if (GOLDEN / "manifest.json").is_file() else set()
        for name in sorted(old - set(names)):  # only files an earlier run of this generator wrote
            (GOLDEN / name).unlink(missing_ok=True)
        for name in names:
            shutil.copyfile(new / name, GOLDEN / name)
    print(f"{len(names)} golden files in {GOLDEN.relative_to(REPO)} ({len(SCENARIOS)} scenarios)")


if __name__ == "__main__":
    main()
