#!/usr/bin/env python3
"""Schedule oracle (T034 / RS-003): run the real Python `easy-personal schedule install|remove` and freeze what it did.

Every scenario runs the CLI entry (`easy_personal.surfaces.cli:main`) in a child process inside a throwaway case folder:
`HOME`, `XDG_CONFIG_HOME` and `EASY_PERSONAL_HOME` point into it, and `PATH` is that folder's `bin/` alone, which holds a
stub `easy-personal` and (when the scenario wants one) `fake-systemctl.sh` as `systemctl`. The real `systemctl` can never
be reached, and nothing is written outside the case. Per run it records argv, exit code, stdout, the kind of stderr, the
files and folders left under the case, and the `systemctl` calls the fake logged, in `rust/parity/schedule/golden/`.

    rust/parity/.venv/bin/python rust/parity/gen_schedule.py           # (re)write rust/parity/schedule/golden/
    rust/parity/.venv/bin/python rust/parity/gen_schedule.py --check   # regenerate in a temp folder and compare: exit 0 equal, 1 different

Python crashes with a traceback where the Rust port exits 1 with one clean line (a failing `systemctl`, an unwritable
folder): the golden keeps the exit code, the files, the calls and the exception class, never the traceback text.
The stub's path is `<CMD>` in every golden but the quoting scenario (Q01), where the Rust binary is put at the same path.
It refuses to run when an env var or TMPDIR points at the owner's real ~/.easy-personal, ~/.claude*, ~/.codex*. POSIX only.
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True  # importing gen_golden leaves no .pyc beside it
PARITY = Path(__file__).resolve().parent
REPO = PARITY.parents[1]
sys.path.insert(0, str(PARITY))
import gen_golden  # noqa: E402  (the real-home guard)

SCHEDULE = PARITY / "schedule"
GOLDEN = SCHEDULE / "golden"
FAKE = SCHEDULE / "fake-systemctl.sh"
TEMPLATES = REPO / "src" / "easy_personal" / "data" / "systemd"
GENERATOR = "rust/parity/gen_schedule.py"
ENTRY = "import sys; from easy_personal.surfaces.cli import main; sys.exit(main())"  # what the console script runs
CHILD_TIMEOUT = 60
SNAP = ("xdg", "rel", "data", "home")  # the folders of a case whose content is recorded (home is always there)
FIXED_ENV = {"TZ": "UTC", "LC_ALL": "C.UTF-8", "LANG": "C.UTF-8", "PYTHONHASHSEED": "0", "PYTHONPATH": str(REPO / "src"),
             "PYTHONDONTWRITEBYTECODE": "1", "PYTHONIOENCODING": "utf-8", "NO_COLOR": "1", "PYTHON_COLORS": "0"}  # fmt: skip
QUOTE_DIRS = ["plain", "sp ace", "it's", 'dq"uote', "ünï", "dollar$x", "back\\slash", "pct%x", "star*",
              "tab\tchar", "semi;colon", "safe=a@b+c,d%e-f_g.h"]  # fmt: skip


def refuse(name: str, value: Path) -> None:
    """Exit 1 when `value` is, or is inside, a real home's .claude* / .codex* / .easy-* folder, or is a real home."""
    target = value.expanduser().resolve()
    for home in gen_golden._real_homes():
        if target == home:
            sys.exit(f"refused: {name} is the real home folder (NFR-003); nothing was opened")
        if home in target.parents and target.relative_to(home).parts[0].startswith(gen_golden.PROTECTED_PREFIXES):
            sys.exit(f"refused: {name} points inside the real {target.relative_to(home).parts[0]} (NFR-003)")


class Case:
    """Fresh folders for one scenario; `bin/` is the whole PATH of every child."""

    def __init__(self, base: Path) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="c-", dir=base)).resolve()
        for name in ("home", "bin", "data", "log"):
            (self.root / name).mkdir()
        stub = self.root / "bin" / "easy-personal"
        stub.write_text("#!/bin/sh\nexit 0\n")
        stub.chmod(0o755)
        self.log = self.root / "log" / "calls"
        self.pending: list[dict] = []  # steps done since the last run; the next run records them as `before`
        self.runs: list[dict] = []
        self.has_systemctl = False

    # -- inputs
    def systemctl(self, present: bool) -> None:
        target = self.root / "bin" / "systemctl"
        if present:
            shutil.copyfile(FAKE, target)
            target.chmod(0o755)
        else:
            target.unlink(missing_ok=True)
        self.has_systemctl = present
        self.pending.append({"op": "systemctl", "present": present})

    def put(self, rel: str, text: str = "stale\n") -> None:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        self.pending.append({"op": "put", "path": rel, "text": text})

    def mkdir(self, rel: str) -> None:
        (self.root / rel).mkdir(parents=True)
        self.pending.append({"op": "mkdir", "path": rel})

    def rm(self, rel: str) -> None:
        path = self.root / rel
        path.rmdir() if path.is_dir() else path.unlink()
        self.pending.append({"op": "rm", "path": rel})

    def stub(self, folder: str) -> None:
        """A folder holding an executable `easy-personal` (the Rust test puts the binary there)."""
        (self.root / folder).mkdir()
        stub = self.root / folder / "easy-personal"
        stub.write_text("#!/bin/sh\nexit 0\n")
        stub.chmod(0o755)
        self.pending.append({"op": "stub", "dir": folder})

    # -- running
    def tok(self, text: str, cmd: bool) -> str:
        if cmd:
            text = text.replace(str(self.root / "bin" / "easy-personal"), "<CMD>")
        return text.replace(str(self.root), "<CASE>")

    def run(self, label: str, argv: list[str], env: dict | None = None, fail: str | None = None,
            no_session: bool = False, cmd_dir: str | None = None) -> dict:  # fmt: skip
        """Run `easy-personal <argv>`; `cmd_dir` names another folder (with its own stub) as the whole PATH."""
        child = {"PATH": str(self.root / (cmd_dir or "bin")), "HOME": str(self.root / "home"),
                 "XDG_CONFIG_HOME": str(self.root / "xdg"), "EASY_PERSONAL_HOME": str(self.root / "data"),
                 "FAKE_SYSTEMCTL_LOG": str(self.log), **FIXED_ENV}  # fmt: skip
        if fail:
            child["FAKE_SYSTEMCTL_FAIL"] = fail
        if no_session:
            child["FAKE_SYSTEMCTL_NO_SESSION"] = "1"
        for key, value in (env or {}).items():
            child.pop(key, None) if value is None else child.update({key: value})
        self.log.write_text("")
        proc = subprocess.run([sys.executable, "-P", "-c", ENTRY, *argv], cwd=self.root, env=child, capture_output=True,
                              timeout=CHILD_TIMEOUT)  # fmt: skip
        cmd = cmd_dir is None
        err = proc.stderr.decode("utf-8")
        files, dirs = {}, []
        for top in SNAP:
            base = self.root / top
            if base.exists() and top not in ("home", "data"):
                dirs.append(top)
            for path in sorted(base.rglob("*")) if base.exists() else []:
                rel = str(path.relative_to(self.root))
                if path.is_dir():
                    dirs.append(rel)
                else:
                    files[rel] = self.tok(path.read_text(), cmd)
        result = {
            "before": self.pending, "label": label, "cmd_dir": cmd_dir or "bin", "argv": ["easy-personal", *argv], "env_extra": env or {},
            "fake_systemctl": {"on_path": self.has_systemctl and cmd_dir is None, "fail": fail, "no_session": no_session},
            "exit_code": proc.returncode, "stdout": self.tok(proc.stdout.decode("utf-8"), cmd),
            "stderr": self.stderr_kind(err), "files": files, "dirs": dirs,
            "systemctl_calls": self.log.read_text().splitlines(),
        }  # fmt: skip
        self.pending = []
        self.runs.append(result)
        return result

    @staticmethod
    def stderr_kind(err: str) -> str:
        if not err:
            return "empty"
        if "Traceback (most recent call last):" in err:
            return "traceback: " + [line for line in err.splitlines() if line.strip()][-1].split(":")[0]
        if err.startswith("usage: easy-personal"):
            return "usage error (argparse text is CPython's, not compared)"
        raise RuntimeError(f"unexpected stderr: {err!r}")


# -- scenarios -------------------------------------------------------------------------------------------------

INSTALL, REMOVE = ["schedule", "install"], ["schedule", "remove"]


def s_dry_run(c: Case) -> str:
    c.systemctl(True)
    c.run("install dry run", [*INSTALL, "--dry-run"])
    c.run("install dry run, retention 30", [*INSTALL, "--retention-days", "30", "--dry-run"])
    c.run("remove dry run", [*REMOVE, "--dry-run"])
    c.systemctl(False)
    c.run("install dry run, no systemctl", [*INSTALL, "--dry-run"])
    return "the three dry runs (nothing written, no systemctl call), and the install dry run with no systemctl"


def s_install_remove(c: Case) -> str:
    c.systemctl(True)
    c.run("install", INSTALL)
    c.run("install again, retention 30", [*INSTALL, "--retention-days", "30"])
    c.run("remove", REMOVE)
    c.run("remove again (nothing installed)", REMOVE)
    return "a real install, a second install with --retention-days, remove, remove with nothing installed"


def s_remove_partial(c: Case) -> str:
    c.systemctl(True)
    c.put("xdg/systemd/user/easy-personal-ingest.timer")
    c.run("remove with only the timer file", REMOVE)
    c.put("xdg/systemd/user/easy-personal-ingest.service")
    c.run("remove with only the service file", REMOVE)
    c.run("remove with neither", REMOVE)
    return "remove calls systemctl only when the timer file existed; a missing file is fine"


def s_no_systemctl(c: Case) -> str:
    c.run("install, no systemctl", INSTALL)
    c.run("install, no systemctl, retention 7", [*INSTALL, "--retention-days", "7"])
    c.run("remove, nothing installed, no systemctl", REMOVE)
    return "no systemctl on PATH: the message and the cron line, exit 1, nothing written"


def s_no_session(c: Case) -> str:
    c.systemctl(True)
    c.run("install, no user session", INSTALL, no_session=True)
    c.run("install, no user session, retention 7", [*INSTALL, "--retention-days", "7"], no_session=True)
    return "systemctl present but `--user show-environment` fails: the same message, only that one call"


def s_install_fails(c: Case) -> str:
    c.systemctl(True)
    c.run("enable fails", INSTALL, fail="enable")
    c.run("daemon-reload fails", INSTALL, fail="daemon-reload")
    return "Python: traceback, exit 1; the unit files are already written; the calls stop at the failing one"


def s_remove_fails(c: Case) -> str:
    c.systemctl(True)
    c.run("install", INSTALL)
    c.run("disable fails", REMOVE, fail="disable")
    c.run("final daemon-reload fails", REMOVE, fail="daemon-reload")
    c.systemctl(False)
    c.put("xdg/systemd/user/easy-personal-ingest.timer")
    c.run("timer present, no systemctl", REMOVE)
    return "Python: traceback, exit 1; files stay when disable fails, are gone when the last daemon-reload fails"


def s_unwritable(c: Case) -> str:
    c.systemctl(True)
    c.put("xdg/systemd", "i am a file\n")
    c.run("systemd folder is a file", INSTALL)
    c.rm("xdg/systemd")
    c.mkdir("xdg/systemd/user/easy-personal-ingest.service")
    c.run("service path is a folder", INSTALL)
    c.rm("xdg/systemd/user/easy-personal-ingest.service")
    c.mkdir("xdg/systemd/user/easy-personal-ingest.timer")
    c.run("timer path is a folder (remove)", REMOVE)
    return "Python: traceback, exit 1; install stops before systemctl runs, remove stops after `disable`"


def s_xdg(c: Case) -> str:
    c.systemctl(True)
    c.run("remove dry run, XDG_CONFIG_HOME unset", [*REMOVE, "--dry-run"], env={"XDG_CONFIG_HOME": None})
    c.run("remove dry run, XDG_CONFIG_HOME empty", [*REMOVE, "--dry-run"], env={"XDG_CONFIG_HOME": ""})
    c.run("remove dry run, relative", [*REMOVE, "--dry-run"], env={"XDG_CONFIG_HOME": "rel"})
    c.run("remove dry run, ./rel/", [*REMOVE, "--dry-run"], env={"XDG_CONFIG_HOME": "./rel/"})
    c.run("install, XDG_CONFIG_HOME unset (units under HOME/.config)", INSTALL, env={"XDG_CONFIG_HOME": None})
    return "the unit folder: XDG_CONFIG_HOME, else HOME/.config (empty counts as unset); a relative one stays relative"


def s_usage(c: Case) -> str:
    c.systemctl(True)
    for label, argv in [
        ("retention 0", [*INSTALL, "--retention-days", "0"]), ("retention -5", [*INSTALL, "--retention-days", "-5"]),
        ("retention abc", [*INSTALL, "--retention-days", "abc"]), ("retention 1.5", [*INSTALL, "--retention-days", "1.5"]),
        ("retention without a value", [*INSTALL, "--retention-days"]), ("unknown flag", [*INSTALL, "--bogus"]),
        ("remove takes no retention", [*REMOVE, "--retention-days", "5"]), ("no action", ["schedule"]),
        ("unknown action", ["schedule", "frobnicate"]),
    ]:  # fmt: skip
        c.run(label, argv)
    return "bad arguments: exit 2, nothing written, no systemctl call"


def s_quoting(c: Case) -> str:
    """Q01: one run per folder name; the stub sits in that folder and is the whole PATH (the command is not normalised)."""
    c.systemctl(False)
    for index, name in enumerate(QUOTE_DIRS):
        c.stub(name)
        extra = ["--retention-days", "5"] if index % 2 else []
        c.run(f"dry run from {name!r}", [*INSTALL, *extra, "--dry-run"], cmd_dir=name)
    c.run("no systemd, from 'sp ace'", [*INSTALL, "--retention-days", "7"], cmd_dir="sp ace")
    return "the command is shlex-quoted: plain, space, quotes, non-ASCII, shell characters, and the characters that stay bare"


SCENARIOS = [
    ("D01_dry_run", s_dry_run), ("I01_install_remove", s_install_remove), ("R01_remove_partial", s_remove_partial),
    ("N01_no_systemctl", s_no_systemctl), ("N02_no_session", s_no_session), ("F01_install_fails", s_install_fails),
    ("F02_remove_fails", s_remove_fails), ("W01_unwritable", s_unwritable), ("X01_xdg", s_xdg),
    ("U01_usage_errors", s_usage), ("Q01_quoting", s_quoting),
]  # fmt: skip


def render(value) -> str:
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


def generate(out: Path, base: Path) -> list[str]:
    import easy_personal

    if Path(easy_personal.__file__).resolve().parent != REPO / "src" / "easy_personal":
        raise RuntimeError(f"easy_personal imports from {easy_personal.__file__}, not src/")
    out.mkdir(parents=True)
    texts, scenarios = {}, []
    for scenario_id, function in SCENARIOS:
        case = Case(base)
        what = function(case)
        texts[f"{scenario_id}.json"] = render({"scenario": scenario_id, "what": what, "runs": case.runs})
        scenarios.append({"id": scenario_id, "what": what, "exit_codes": [r["exit_code"] for r in case.runs]})
        for name, text in texts.items():
            if any(str(h) in text for h in {base, *gen_golden._real_homes()}) or "FAKE-TOOL-OUTPUT" in text:
                raise RuntimeError(f"{name}: a temp path, a home path or the fake tool's output got into a golden")
    manifest = {
        "generator": GENERATOR,
        "note": "Python `easy-personal schedule` runs frozen as goldens (T034). Per scenario file: runs; "
        "each run has before (the steps done since the previous run), argv, env_extra, fake_systemctl, exit_code, stdout, stderr (kind only), files, dirs, systemctl_calls.",
        "tokens": {"<CASE>": "the scenario's temp folder (home/, xdg/, data/, bin/)", "<CMD>": "<CASE>/bin/easy-personal, "
                   "the stub the command is found at (every scenario but Q01)"},  # fmt: skip
        "units_sha256": {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(TEMPLATES.iterdir())},
        "fake_systemctl_sha256": hashlib.sha256(FAKE.read_bytes()).hexdigest(),
        "scenarios": scenarios,
        "files": {name: hashlib.sha256(text.encode()).hexdigest() for name, text in sorted(texts.items())},
    }
    texts["manifest.json"] = render(manifest)
    for name, text in texts.items():
        (out / name).write_text(text)
    return sorted(texts)


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
    for name in gen_golden.ENV_PATHS:  # the child gets a built env, but the generator itself must not point at real data
        if os.environ.get(name):
            refuse(name, Path(os.environ[name]))
    gen_golden.refuse_real_paths(GOLDEN)
    base = Path(tempfile.gettempdir()).resolve()
    refuse("TMPDIR", base)
    if GOLDEN.exists() and not args.check:
        try:
            ours = json.loads((GOLDEN / "manifest.json").read_text())["generator"] == GENERATOR
        except (OSError, ValueError, LookupError, TypeError):
            ours = False
        if not ours:
            sys.exit(f"refused: {GOLDEN} was not made by this generator; nothing was deleted")
    os.umask(0o022)
    sys.path.insert(0, str(REPO / "src"))
    with tempfile.TemporaryDirectory(prefix="schedule-oracle-", dir=base) as tmp:
        run_root = Path(tmp).resolve()
        names = generate(run_root / "golden", run_root)
        if args.check:
            diffs = compare(run_root / "golden", GOLDEN)
            for line in diffs:
                print(line)
            print(f"--check: {len(names)} files, {len(diffs)} different")
            sys.exit(1 if diffs else 0)
        GOLDEN.mkdir(parents=True, exist_ok=True)
        old = set(json.loads((GOLDEN / "manifest.json").read_text())["files"]) | {"manifest.json"} if (GOLDEN / "manifest.json").is_file() else set()
        for name in sorted(old - set(names)):  # only files an earlier run of this generator wrote
            (GOLDEN / name).unlink(missing_ok=True)
        for name in names:
            shutil.copyfile(run_root / "golden" / name, GOLDEN / name)
    print(f"{len(names)} golden files in {GOLDEN.relative_to(REPO)} ({len(SCENARIOS)} scenarios)")


if __name__ == "__main__":
    main()
