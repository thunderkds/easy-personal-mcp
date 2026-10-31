"""Fake `systemctl` and `crontab` for the scheduled-jobs reader (HUN-163).

`install(bindir, ...)` writes two executables into `bindir`; put it alone on PATH and the real
tools are never reached. Each call is logged to `<bindir>/argv.log` (one JSON list per line). Only
the read subcommands are answered: any other one exits 3, so a test can also see it in the log.
"""

import json
import sys
from pathlib import Path

_SYSTEMCTL = """#!{python}
import json, sys, time
from pathlib import Path

here = Path(__file__).resolve().parent
data = json.loads((here / "fake.json").read_text())
argv = sys.argv[1:]
with (here / "argv.log").open("a") as log:
    log.write(json.dumps(["systemctl", *argv]) + "\\n")
if data.get("hang"):
    time.sleep(data["hang"])
if not data["bus"]:
    sys.stderr.write("Failed to connect to bus: No medium found\\n")
    sys.exit(1)
if argv[:1] != ["--user"]:
    sys.exit(2)
sub = argv[1]
if sub == "list-timers":
    if "--output=json" in argv:
        if not data["json"]:
            sys.stderr.write("Unknown output format\\n")
            sys.exit(1)
        print(json.dumps(data["timers"]))
    else:
        for t in data["timers"]:
            print("Sat 2026-10-31 18:00:00 +07 2h 43min Sat 2026-10-31 15:00:06 +07 16min ago "
                  + t["unit"] + " " + t["activates"])
elif sub == "show":
    wanted = set(argv[argv.index("-p") + 1].split(","))
    units = [a for a in argv[argv.index("-p") + 2:]]
    blocks = []
    for unit in units:
        props = {{"Id": unit, **data["props"].get(unit, {{}})}}
        blocks.append("\\n".join(f"{{k}}={{v}}" for k, v in props.items() if k in wanted))
    print("\\n\\n".join(blocks))
else:
    sys.exit(3)
"""

_CRONTAB = """#!{python}
import json, sys, time
from pathlib import Path

here = Path(__file__).resolve().parent
data = json.loads((here / "fake.json").read_text())
with (here / "argv.log").open("a") as log:
    log.write(json.dumps(["crontab", *sys.argv[1:]]) + "\\n")
if data.get("hang"):
    time.sleep(data["hang"])
if sys.argv[1:] != ["-l"]:
    sys.exit(3)
if data["crontab"] is None:
    sys.stderr.write("no crontab for tester\\n")
    sys.exit(1)
sys.stdout.write(data["crontab"])
"""


def exec_start(argv: str) -> str:
    """A `systemctl show` ExecStart value for the command line `argv` (space-joined, as systemd prints it)."""
    first = argv.split()[0]
    return f"{{ path={first} ; argv[]={argv} ; ignore_errors=no ; start_time=[n/a] ; pid=0 ; code=(null) ; status=0/0 }}"


def install(
    bindir: Path,
    *,
    timers: list[dict] | None = None,
    props: dict[str, dict[str, str]] | None = None,
    crontab: str | None = None,
    json_output: bool = True,
    bus: bool = True,
    hang: float = 0,
    with_systemctl: bool = True,
    with_crontab: bool = True,
) -> Path:
    """Write the fakes and their data into `bindir`; returns the argv log path."""
    bindir.mkdir(parents=True, exist_ok=True)
    (bindir / "fake.json").write_text(
        json.dumps(
            {
                "timers": timers or [],
                "props": props or {},
                "crontab": crontab,
                "json": json_output,
                "bus": bus,
                "hang": hang,
            }
        )
    )
    for name, body, wanted in (
        ("systemctl", _SYSTEMCTL, with_systemctl),
        ("crontab", _CRONTAB, with_crontab),
    ):
        if wanted:
            path = bindir / name
            path.write_text(body.format(python=sys.executable))
            path.chmod(0o755)
    log = bindir / "argv.log"
    log.write_text("")
    return log


def calls(log: Path) -> list[list[str]]:
    return [json.loads(line) for line in log.read_text().splitlines()]
