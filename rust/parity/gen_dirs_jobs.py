"""Makes `rust/parity/golden-jobs/`: the oracle of `GET /api/dirs` and `GET /api/jobs` (T044).

Python's own `folders.list_dirs` runs over a temp folder tree (`tree.json` says how to build it again) and Python's
`jobs.list_jobs` runs with the fake `systemctl` / `crontab` of `tests/fakes/jobs_fakes.py` (imported read-only) alone on
`PATH`, at a frozen local time under `TZ=UTC`. Nothing here touches the owner's data folder, `~/.claude*`, a real
`systemctl` or a real crontab. Temp paths are written as `<ROOT>` in the files, as `gen_ingest_kit.py` normalises its own.

    rust/parity/.venv/bin/python rust/parity/gen_dirs_jobs.py

Files: `tree.json`, `dirs.json` (cases and Python's answers), `jobs.json` (cases, the fakes' recorded calls and Python's answers).
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from datetime import datetime
from pathlib import Path

sys.dont_write_bytecode = True
PARITY = Path(__file__).resolve().parent
REPO = PARITY.parent.parent
sys.path.insert(0, str(REPO / "src"))
sys.path.insert(0, str(REPO / "tests"))
OUT = PARITY / "golden-jobs"
NOW = (2026, 11, 11, 10, 0, 0)  # the frozen local time (TZ=UTC): a Wednesday

TREE = [  # (kind, relative path, link target or None); `many/` holds 501 sub-folders, made in code
    ("dir", "main/with space", None),
    ("dir", "main/ünï", None),
    ("dir", "main/Éclair", None),
    ("dir", "main/apple", None),
    ("dir", "main/Banana", None),
    ("dir", "main/zeta", None),
    ("dir", "main/.hidden", None),
    ("file", "main/file.txt", None),
    ("link", "main/link_to_dir", "apple"),
    ("link", "main/broken", "nowhere"),
    ("link", "main/loop", "loop"),
    ("dir", "main/repo/.git", None),
    ("file", "main/gitfile/.git", None),
    ("dir", "main/locked/inner", None),
    ("dir", "home", None),
]

DIR_CASES = [  # (name, path template or None for the working folder, needs an unprivileged user)
    ("listing", "<ROOT>/main", False),
    ("trailing_slash", "<ROOT>/main/", False),
    ("dotdot", "<ROOT>/main/apple/../zeta", False),
    ("through_link", "<ROOT>/main/link_to_dir", False),
    ("truncated", "<ROOT>/many", False),
    ("file", "<ROOT>/main/file.txt", False),
    ("relative", "relative/path", False),
    ("missing", "<ROOT>/main/nope", False),
    ("nul", "<ROOT>/main\x00", False),
    ("loop", "<ROOT>/main/loop", False),
    ("broken", "<ROOT>/main/broken", False),
    ("cwd", None, False),
    ("empty", "", False),
    ("locked", "<ROOT>/main/locked", True),
]


def build_tree(root: Path) -> None:
    for kind, rel, target in TREE:
        path = root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        if kind == "dir":
            path.mkdir(parents=True, exist_ok=True)
        elif kind == "file":
            path.write_text("x")
        else:
            path.symlink_to(target)
    for i in range(501):
        (root / "many" / f"d{i:03d}").mkdir(parents=True, exist_ok=True)
    (root / "many" / "d007" / ".git").mkdir()


def norm(value, root: str):
    if isinstance(value, str):
        return value.replace(root, "<ROOT>")
    if isinstance(value, list):
        return [norm(v, root) for v in value]
    if isinstance(value, dict):
        return {k: norm(v, root) for k, v in value.items()}
    return value


def dirs_cases(root: Path) -> list[dict]:
    from easy_personal.core import folders

    os.environ["HOME"] = str(root / "home")
    os.chdir(root / "main")
    (root / "main" / "locked").chmod(0o000)
    cases = []
    try:
        for name, template, unprivileged in DIR_CASES:
            path = None if template is None else template.replace("<ROOT>", str(root))
            try:
                status, body = 200, folders.list_dirs(path)
            except ValueError as exc:
                status, body = 400, {"error": str(exc)}
            except OSError:
                status, body = 403, {"error": "Cannot read this folder"}
            if name == "nul":  # CPython's own text ("lstat: embedded null character in path"): status and the `error` key only
                body = {"error": None}
            cases.append({"case": name, "path": template, "unprivileged": unprivileged, "status": status, "body": norm(body, str(root))})
    finally:
        (root / "main" / "locked").chmod(0o755)
    return cases


CRON = """# a comment
MAILTO=nobody@example.invalid

0 7 * * 1-5 claude -p "hello there"
@daily /usr/bin/easy-personal ingest --token SECRET
@hourly easy-ui check
@reboot easy-ui check
*/15 * * * * env FOO=bar sudo claude
5 4 * * * /usr/bin/backup --all
0 0 31 2 * claude -p never
0 9 * * mon claude -p names
30 2 1 * 1 easy-verifier run
*/0 * * * * claude -p bad-step
0 0 * * * bash -c "easy-context index && rm -rf /x"
0 1 * * * python3 -m easy_personal ingest
0 5 * * * claude -p "unterminated
7 7 * * * FOO=1 /opt/bin/easy-memory save
0 8 * * 7 claude -p sunday
15 14 29 2 * easy-personal ingest
 10  *  *  *  *   claude    spaced
@weekly
1 2 3 4
"""

CRON_STEPS = """1/9223372036854775807 * * * * claude -p huge-step
0 1/9223372036854775807 * * * claude -p huge-hour-step
*/9223372036854775807 * * * * claude -p huge-star-step
1-30/9223372036854775807 * * * * claude -p huge-range-step
1/99999999999999999999999 * * * * claude -p step-beyond-64-bits
*/0 * * * * claude -p step-zero
5/0 * * * * claude -p first-step-zero
*/-1 * * * * claude -p step-minus-one
*/-9223372036854775808 * * * * claude -p step-min
*/100 * * * * claude -p step-wider-than-range
30/100 * * * * claude -p first-step-wider-than-range
*/20 * * * * claude -p ordinary-step
"""

TIMERS = [
    {"unit": "easy-personal-ingest.timer", "activates": "easy-personal-ingest.service", "next": 1_793_444_400_000_000, "last": 1_793_433_606_000_000},
    {"unit": "kit-check.timer", "activates": "kit-check.service", "next": 1_793_448_000_999_999, "last": 1_793_430_000_000_000},
    {"unit": "tidy.timer", "activates": "tidy.service", "next": 1_793_444_400_000_000, "last": 0},
    {"unit": "orphan.timer", "activates": "orphan.service", "next": None, "last": None},
    {"unit": "-evil.timer", "activates": "evil.service", "next": 1, "last": 1},
    {"unit": "a b.timer", "activates": "ab.service", "next": 1, "last": 1},
    {"unit": "x@1.timer", "activates": "x@1.service", "next": 1_793_444_400_000_000, "last": 1_793_433_606_000_000},
    {"unit": "flag.timer", "activates": "-flag.service", "next": 1_793_444_400_000_000, "last": 1_793_433_606_000_000},
    {"unit": "float.timer", "activates": "float.service", "next": 1.5e15, "last": -5},
    {"unit": "plain.service", "activates": "plain.service", "next": 1, "last": 1},
]
PROPS = {
    "easy-personal-ingest.timer": {"TimersCalendar": "{ OnCalendar=*-*-* 03:00:00 ; next_elapse=Thu 2026-11-12 03:00:00 UTC }"},
    "easy-personal-ingest.service": {"ExecStart": "{ path=/usr/bin/easy-personal ; argv[]=/usr/bin/easy-personal ingest --token SECRET ; ignore_errors=no ; start_time=[n/a] }", "ExecMainStatus": "0"},
    "kit-check.timer": {"TimersCalendar": "{ OnCalendar=Mon..Fri *-*-* 09:00:00 ; next_elapse=n/a } { OnCalendar=Sat *-*-* 10:00:00 ; next_elapse=n/a }", "TimersMonotonic": "{ OnBootSec=5min ; next_elapse=n/a } { OnUnitActiveSec=1h 30min ; next_elapse=n/a }"},
    "kit-check.service": {"ExecStart": "{ path=/usr/bin/env ; argv[]=/usr/bin/env FOO=1 /home/u/.local/bin/easy-verifier run --secret ; ignore_errors=no }", "ExecMainStatus": "3"},
    "tidy.service": {"ExecStart": "{ path=/usr/bin/rm ; argv[]=/usr/bin/rm -rf /tmp/x ; ignore_errors=no }", "ExecMainStatus": "0"},
    "x@1.timer": {"TimersMonotonic": "{ OnCalendar=hourly ; x }"},
    "x@1.service": {"ExecStart": "{ path=/bin/bash ; argv[]=/bin/bash -c 'claude -p hi' ; ignore_errors=no }", "ExecMainStatus": "x1"},
    "float.service": {"ExecStart": "{ path=/usr/bin/claude ; argv[]=claude ; ignore_errors=no }", "ExecMainStatus": "1"},
}


def jobs_cases(root: Path) -> list[dict]:
    import easy_personal.core.jobs as jobs
    from fakes import jobs_fakes

    class Frozen(datetime):
        @classmethod
        def now(cls, tz=None):
            return cls(*NOW)

    jobs.datetime = Frozen
    os.environ["EASY_PERSONAL_HOME"] = str(root / "home")  # no config.yaml: the default allow-list
    variants = [
        ("mixed", dict(timers=TIMERS, props=PROPS, crontab=CRON)),
        ("table_fallback", dict(timers=TIMERS[:3], props=PROPS, crontab=None, json_output=False)),
        ("no_bus", dict(timers=TIMERS, props=PROPS, crontab=CRON, bus=False)),
        ("no_crontab", dict(timers=TIMERS[:3], props=PROPS, crontab=None)),
        ("no_tools", dict(with_systemctl=False, with_crontab=False)),
        ("only_crontab", dict(crontab="0 7 * * * claude\n", with_systemctl=False)),
        ("empty", dict(timers=[], crontab="")),
        ("cron_steps", dict(crontab=CRON_STEPS, with_systemctl=False)),  # T052: steps no 64-bit integer can add twice
    ]
    cases = []
    for name, kwargs in variants:
        bindir = root / f"bin-{name}"
        log = jobs_fakes.install(bindir, **kwargs)
        os.environ["PATH"] = str(bindir)
        body = jobs.list_jobs()
        calls = []
        for argv in jobs_fakes.calls(log):
            done = subprocess.run([str(bindir / argv[0]), *argv[1:]], capture_output=True, text=True, env={"PATH": str(bindir)}, check=False)
            calls.append({"argv": argv, "stdout": done.stdout, "exit": done.returncode})
        tools = {"systemctl": (bindir / "systemctl").exists(), "crontab": (bindir / "crontab").exists()}
        cases.append({"case": name, "tools": tools, "calls": calls, "body": norm(body, str(root))})
    return cases


def main() -> None:
    os.environ["TZ"] = "UTC"
    time.tzset()
    root = Path(tempfile.mkdtemp(prefix="t044-gen-")).resolve()
    try:
        build_tree(root)
        OUT.mkdir(exist_ok=True)
        dump = lambda name, data: (OUT / name).write_text(json.dumps(data, indent=2, sort_keys=True, ensure_ascii=False) + "\n")
        dump("tree.json", {"entries": TREE, "many": 501, "many_repo": "many/d007"})
        dump("dirs.json", {"cases": dirs_cases(root)})
        dump("jobs.json", {"now": list(NOW), "tz": "UTC", "cases": jobs_cases(root)})
    finally:
        shutil.rmtree(root, ignore_errors=True)  # the one mode-000 folder is restored in `dirs_cases`


if __name__ == "__main__":
    main()
