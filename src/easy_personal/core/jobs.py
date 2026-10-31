"""Scheduled jobs (FR-040): systemd user timers and crontab lines that run an agent command.

Read-only. Only the command's first word (basename) leaves this module: arguments, the full
ExecStart and environment lines are parsed in memory to find that word and then dropped.
Each kind that cannot be read (no tool, no user bus, timeout) is `no data`; the other still lists.
"""

import json
import os
import re
import shlex
import shutil
import subprocess
from datetime import UTC, date, datetime, time, timedelta
from pathlib import PurePath

from easy_personal.core import config

TIMEOUT = 3.0  # seconds, per subprocess call
# Tool -> the only subcommands that may run. `systemctl` is always called with `--user` first.
_READ_ONLY = {"systemctl": {"list-timers", "show"}, "crontab": {"-l"}}
_UNIT = re.compile(r"[A-Za-z0-9:_.@\\][A-Za-z0-9:_.@\\-]*")  # never starts with "-": it would pass as an option
_ASSIGNMENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*=")
_SHELLS = {"sh", "bash", "zsh", "dash"}
_ALIASES = {
    "@hourly": "0 * * * *",
    "@daily": "0 0 * * *",
    "@midnight": "0 0 * * *",
    "@weekly": "0 0 * * 0",
    "@monthly": "0 0 1 * *",
    "@yearly": "0 0 1 1 *",
    "@annually": "0 0 1 1 *",
}
NO_DATA, OK = "no data", "ok"


def _run(argv: list[str]) -> str | None:
    """stdout of one read-only command; None when the tool is missing, fails or times out."""
    tool = argv[0]
    sub = argv[2] if tool == "systemctl" and argv[1:2] == ["--user"] else argv[1:2] and argv[1]
    if sub not in _READ_ONLY.get(tool, ()):
        raise ValueError(f"not a read-only job command: {tool} {sub}")
    if not shutil.which(tool):
        return None
    try:
        done = subprocess.run(
            argv,
            capture_output=True,
            text=True,
            errors="replace",
            stdin=subprocess.DEVNULL,
            env={**os.environ, "LC_ALL": "C"},
            timeout=TIMEOUT,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    return done.stdout if done.returncode == 0 else None


def _systemctl(*args: str) -> str | None:
    return _run(["systemctl", "--user", *args])


def _words(line: str) -> list[str]:
    try:
        return shlex.split(line)
    except ValueError:
        return line.split()


def _first_word(words: list[str], depth: int = 0) -> str | None:
    """Basename of the command in `words`, looking through `env`, `VAR=x`, `sh -c` and `python -m easy_personal`."""
    i = 0
    while i < len(words):
        word = words[i]
        if _ASSIGNMENT.match(word):
            i += 1
        elif PurePath(word).name == "env":
            i += 1
            while i < len(words) and (words[i].startswith("-") or _ASSIGNMENT.match(words[i])):
                i += 1
        else:
            break
    if i >= len(words):
        return None
    name = PurePath(words[i]).name
    rest = words[i + 1 :]
    if name in _SHELLS and "-c" in rest and depth < 3 and rest.index("-c") + 1 < len(rest):
        return _first_word(_words(rest[rest.index("-c") + 1]), depth + 1)
    if name.startswith("python") and rest[:1] == ["-m"] and rest[1:2] and rest[1].split(".")[0] == "easy_personal":
        return "easy-personal"  # the fallback command `schedule.command()` writes when the script is not on PATH
    return name or None


def command_name(line: str) -> str | None:
    """The first word (basename) of a command line, or None when there is no command."""
    return _first_word(_words(line))


# --- crontab ----------------------------------------------------------------------------------


def _field(text: str, low: int, high: int) -> set[int] | None:
    out: set[int] = set()
    try:
        for part in text.split(","):
            span, slash, step_text = part.partition("/")
            step = int(step_text) if slash else 1
            if step < 1:
                return None
            if span == "*":
                first, last = low, high
            elif "-" in span:
                a, b = span.split("-", 1)
                first, last = int(a), int(b)
            else:
                first = int(span)
                last = high if slash else first
            if not (low <= first <= last <= high):
                return None
            out.update(range(first, last + 1, step))
    except ValueError:
        return None
    return out


def next_cron(expr: str, after: datetime) -> datetime | None:
    """Next local time strictly after `after` that a 5-field cron expression fires; None if unreadable.

    Names (`mon`, `jan`) and `@reboot` are not read: None, shown as "unknown".
    """
    fields = _ALIASES.get(expr, expr).split()
    if len(fields) != 5:
        return None
    parsed = [_field(f, lo, hi) for f, (lo, hi) in zip(fields, ((0, 59), (0, 23), (1, 31), (1, 12), (0, 7)))]
    if any(p is None for p in parsed):
        return None
    minutes, hours, days, months, weekdays = parsed
    weekdays = {d % 7 for d in weekdays}
    start = after.replace(second=0, microsecond=0) + timedelta(minutes=1)
    either = fields[2] != "*" and fields[4] != "*"  # both restricted: a match on either field fires
    for offset in range(366 * 5):
        day: date = start.date() + timedelta(days=offset)
        if day.month not in months:
            continue
        in_day, in_week = day.day in days, day.isoweekday() % 7 in weekdays
        if not (in_day or in_week if either else in_day and in_week):
            continue
        for hour in sorted(hours):
            for minute in sorted(minutes):
                moment = datetime.combine(day, time(hour, minute))
                if moment >= start:
                    return moment
    return None


def _utc(moment: datetime) -> str:
    return moment.astimezone(UTC).strftime("%Y-%m-%dT%H:%M:%SZ")


def _cron_jobs(allow: tuple[str, ...]) -> tuple[list[dict], int, str]:
    text = _run(["crontab", "-l"])
    if text is None:
        return [], 0, NO_DATA
    now, rows, hidden = datetime.now(), [], 0
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or _ASSIGNMENT.match(line):
            continue
        if line.startswith("@"):
            parts = line.split(None, 1)
            schedule, command = parts[0], parts[1] if len(parts) > 1 else ""
        else:
            parts = line.split(None, 5)
            schedule, command = " ".join(parts[:5]), parts[5] if len(parts) > 5 else ""
        word = command_name(command)
        if word is None:
            continue
        if word not in allow:
            hidden += 1
            continue
        following = next_cron(schedule, now)
        rows.append(
            {
                "name": f"(crontab) {word}",
                "kind": "cron",
                "command": word,
                "schedule": schedule,
                "next_run": _utc(following) if following else None,
                "last_run": None,
                "last_exit": None,
            }
        )
    return rows, hidden, OK


# --- systemd ----------------------------------------------------------------------------------


def _timers() -> list[dict] | None:
    """`[{unit, activates, next, last}]` (times in microseconds since the epoch, or None); None = no data."""
    out = _systemctl("list-timers", "--all", "--output=json", "--no-pager")
    try:
        rows = json.loads(out) if out is not None else None
    except ValueError:
        rows = None  # an older systemd ignores --output and prints the table
    if isinstance(rows, list):
        return [
            {"unit": r.get("unit"), "activates": r.get("activates"), "next": r.get("next"), "last": r.get("last")}
            for r in rows
            if isinstance(r, dict) and isinstance(r.get("unit"), str)
        ]
    table = _systemctl("list-timers", "--all", "--no-legend", "--no-pager")
    if table is None:
        return None
    found = []
    for line in table.splitlines():
        words = line.split()
        unit = next((w for w in reversed(words) if w.endswith(".timer")), None)
        if unit:
            after = words[words.index(unit) + 1 :]
            service = after[0] if after and after[0].endswith(".service") else None
            found.append({"unit": unit, "activates": service, "next": None, "last": None})
    return found


def _properties(units: list[str]) -> dict[str, dict[str, str]]:
    """One `systemctl show` call for every unit: `{Id: {property: value}}`."""
    wanted = "Id,TimersCalendar,TimersMonotonic,ExecStart,ExecMainStatus"
    out = _systemctl("show", "-p", wanted, *units) if units else None
    found: dict[str, dict[str, str]] = {}
    for block in (out or "").split("\n\n"):
        props = dict(line.partition("=")[::2] for line in block.splitlines() if "=" in line)
        if "Id" in props:
            found[props["Id"]] = props
    return found


def _schedule(props: dict[str, str]) -> str | None:
    calendar = re.findall(r"OnCalendar=([^;}]*?)\s*(?:;|\})", props.get("TimersCalendar", ""))
    monotonic = re.findall(r"(On\w+Sec=[^;}]*?)\s*(?:;|\})", props.get("TimersMonotonic", ""))
    return ", ".join([*(f"OnCalendar={c}" if len(calendar) > 1 else c for c in calendar), *monotonic]) or None


def _exec_command(props: dict[str, str]) -> str | None:
    match = re.search(r"argv\[\]=(.*?) ; ignore_errors=", props.get("ExecStart", ""))
    return command_name(match.group(1)) if match else None


def _micro(value) -> str | None:
    return _utc(datetime.fromtimestamp(value / 1_000_000, UTC)) if isinstance(value, int) and value > 0 else None


def _systemd_jobs(allow: tuple[str, ...]) -> tuple[list[dict], int, str]:
    timers = _timers()
    if timers is None:
        return [], 0, NO_DATA
    timers = [t for t in timers if t["unit"].endswith(".timer") and _UNIT.fullmatch(t["unit"])]
    for t in timers:
        service = t["activates"] if isinstance(t["activates"], str) else t["unit"][:-6] + ".service"
        t["service"] = service if _UNIT.fullmatch(service) else None
    props = _properties([u for t in timers for u in (t["unit"], t["service"]) if u])
    rows, hidden = [], 0
    for t in timers:
        service_props = props.get(t["service"] or "", {})
        word = _exec_command(service_props)
        if word not in allow:
            hidden += 1
            continue
        last = _micro(t["last"])
        status = service_props.get("ExecMainStatus", "")
        rows.append(
            {
                "name": t["unit"],
                "kind": "systemd",
                "command": word,
                "schedule": _schedule(props.get(t["unit"], {})),
                "next_run": _micro(t["next"]),
                "last_run": last,
                "last_exit": int(status) if last and status.isdigit() else None,
            }
        )
    return rows, hidden, OK


def list_jobs() -> dict:
    """`{jobs, hidden, sources}`: agent jobs, the count of other jobs left out, and ok / no data per kind."""
    allow = config.jobs_allow()
    timer_rows, timer_hidden, timer_state = _systemd_jobs(allow)
    cron_rows, cron_hidden, cron_state = _cron_jobs(allow)
    return {
        "jobs": [*timer_rows, *cron_rows],
        "hidden": timer_hidden + cron_hidden,
        "sources": {"systemd": timer_state, "cron": cron_state},
    }
