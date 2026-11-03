"""easy-memory store reader (FR-051..053): a project's saved notes, read live as files, never stored.

easy-memory keeps `<store>/<key>/notes/*.json`, `meta.json` and `prompts.log` (`$EASY_MEMORY_HOME`, else
`~/.easy-memory`). Only a note's `time` and `files` map and a log line's `time`, `effective` and `injected`
are kept; `prompt`, `answer` and `session_id` are parsed away and never returned, logged or put in an
exception. `report.json` (FR-065, written by easy-memory's `report --json`) is read for its on-vs-off numbers only. `trash/` is never opened. A cited file is opened only to hash it, only inside the project folder.
Read-only: nothing is written anywhere. Core code: stdlib only, it must not import connectors.
"""

import hashlib
import json
import math
import os
import stat
from datetime import UTC, date, datetime, timedelta
from pathlib import Path

FORMAT_VERSION = 1  # the newest note format this reader understands
MAX_NOTE_BYTES = 1024 * 1024  # a bigger note is abstained, never read
MAX_META_BYTES = 64 * 1024
MAX_REPORT_BYTES = 64 * 1024
MAX_GENERATED_CHARS = 64
REPORT_VERSION = 1  # the newest report.json format this reader understands
MAX_LOG_LINE = 64 * 1024  # a longer prompts.log line is skipped
MAX_HASH_BYTES = 10 * 1024 * 1024  # a bigger cited file is "unchecked"
MAX_NODES = 30
DAYS = 30
_OLDEST = datetime.min.replace(tzinfo=UTC)


def store_home() -> Path:
    """`$EASY_MEMORY_HOME` when set and not empty, else `~/.easy-memory` (as easy-memory itself does)."""
    override = os.environ.get("EASY_MEMORY_HOME")
    return Path(override) if override else Path.home() / ".easy-memory"


def _meta_path(folder: Path) -> str | None:
    """The identity path from `meta.json`, or None when the file is missing, too big or not usable."""
    try:
        with open(folder / "meta.json", "rb") as handle:
            raw = handle.read(MAX_META_BYTES + 1)
        data = json.loads(raw) if len(raw) <= MAX_META_BYTES else None
    except (OSError, ValueError, RecursionError):
        return None
    path = data.get("path") if isinstance(data, dict) else None
    return path if isinstance(path, str) and path and "\0" not in path else None


def load(home: Path | None = None) -> dict:
    """Index the store once: `meta` = real identity path -> folder, `keyed` = folder name -> folder (no usable meta)."""
    meta: dict[str, Path] = {}
    keyed: dict[str, Path] = {}
    try:
        folders = sorted(p for p in (home or store_home()).iterdir() if p.is_dir())
    except OSError:
        return {"meta": meta, "keyed": keyed}
    for folder in folders:
        path = _meta_path(folder)
        if path is None:
            keyed[folder.name] = folder
            continue
        real = os.path.realpath(path)
        meta.setdefault(real, folder)
        if real.endswith(os.sep + ".git"):  # the repo itself, so a repo without its `.git` still matches
            meta.setdefault(real[: -len(os.sep + ".git")], folder)
    return {"meta": meta, "keyed": keyed}


def _identity(project: str) -> str:
    """easy-memory's identity path of a folder without a `git` call: its git common dir, else its real path."""
    real = os.path.realpath(project)
    dot_git = os.path.join(real, ".git")
    if os.path.isdir(dot_git):
        return os.path.realpath(dot_git)
    try:
        with open(dot_git, encoding="utf-8") as handle:  # a worktree: `.git` is a file `gitdir: <path>`
            first = handle.readline(4096)
    except (OSError, UnicodeDecodeError):
        return real
    if not first.startswith("gitdir:"):
        return real
    gitdir = os.path.realpath(os.path.join(real, first[len("gitdir:"):].strip()))
    try:
        with open(os.path.join(gitdir, "commondir"), encoding="utf-8") as handle:
            return os.path.realpath(os.path.join(gitdir, handle.readline(4096).strip()))
    except (OSError, UnicodeDecodeError):
        return gitdir


def find(store: dict, project: str) -> Path | None:
    """The store folder of a project path, or None: by `meta.json` path first, else by the computed key."""
    try:
        identity = _identity(project)
        folder = store["meta"].get(identity) or store["meta"].get(os.path.realpath(project))
        return folder or store["keyed"].get(hashlib.sha256(identity.encode()).hexdigest()[:16])
    except (OSError, ValueError):
        return None


def has_notes(folder: Path) -> bool:
    """True when the folder's `notes/` holds at least one `.json` file (no file is opened)."""
    try:
        with os.scandir(folder / "notes") as entries:
            return any(e.name.endswith(".json") for e in entries)
    except OSError:
        return False


def _when(text) -> tuple[datetime, date] | None:
    """`(UTC datetime, local calendar day)` of an ISO time, or None when it does not parse."""
    if not isinstance(text, str):
        return None
    try:
        moment = datetime.fromisoformat(text.strip())
        if moment.tzinfo is None:
            moment = moment.replace(tzinfo=UTC)
        return moment, moment.astimezone().date()
    except (ValueError, OverflowError, OSError):
        return None


def _load_note(path: Path) -> tuple[tuple[datetime, date] | None, dict[str, str]] | None:
    """`(time, {cited path: hash})` of one note, or None when it is abstained. Nothing else is kept."""
    try:
        with open(path, "rb") as handle:
            raw = handle.read(MAX_NOTE_BYTES + 1)
        data = json.loads(raw.decode("utf-8")) if len(raw) <= MAX_NOTE_BYTES else None
    except (OSError, ValueError, RecursionError):  # UnicodeDecodeError is a ValueError
        return None
    if not isinstance(data, dict):
        return None
    version, files = data.get("v"), data.get("files")
    if isinstance(version, bool) or not isinstance(version, int) or not 1 <= version <= FORMAT_VERSION:
        return None
    if not isinstance(files, dict):
        return None
    cited = {p: (h if isinstance(h, str) else "") for p, h in files.items() if isinstance(p, str) and p}
    return _when(data.get("time")), cited


def _read_notes(folder: Path) -> tuple[list, int]:
    """The usable notes (oldest file name first) and the number of abstained ones."""
    try:
        names = sorted(n for n in os.listdir(folder / "notes") if n.endswith(".json"))
    except OSError:
        return [], 0
    notes = []
    abstained = 0
    for name in names:
        note = _load_note(folder / "notes" / name)
        if note is None:
            abstained += 1
        else:
            notes.append(note)
    return notes, abstained


def note_count(folder: Path) -> int:
    """Usable notes in a store folder (abstained ones are not counted)."""
    return len(_read_notes(folder)[0])


def _inside(root: str, path: str) -> str | None:
    """Real path of a cited file when it stays inside the project folder, else None (never opened)."""
    if os.path.isabs(path) or ".." in path.split("/") or "\0" in path:
        return None
    try:
        real = os.path.realpath(os.path.join(root, path))
    except (OSError, ValueError):
        return None
    return real if real == root or real.startswith(root + os.sep) else None


def _state(real: str, newest_hash: str) -> str:
    """`fresh`, `stale`, `gone` or `unchecked` for a cited file against the newest citing note's hash."""
    try:
        info = os.stat(real)
    except (FileNotFoundError, NotADirectoryError):
        return "gone"
    except OSError:
        return "unchecked"
    if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_HASH_BYTES:
        return "unchecked"
    digest = hashlib.sha256()
    size = 0
    try:
        with open(real, "rb") as handle:
            while chunk := handle.read(65536):
                size += len(chunk)
                if size > MAX_HASH_BYTES:
                    return "unchecked"
                digest.update(chunk)
    except OSError:
        return "unchecked"
    return "fresh" if digest.hexdigest() == newest_hash.lower() else "stale"


def _prompts(folder: Path, start: date, today: date) -> tuple[int, int]:
    """`(prompts with memory on, of those injected)` over the window, from `prompts.log` line by line."""
    on = injected = 0
    try:
        handle = open(folder / "prompts.log", "rb")  # noqa: SIM115 - closed below
    except OSError:
        return 0, 0
    with handle:
        while line := handle.readline(MAX_LOG_LINE + 1):
            if len(line) > MAX_LOG_LINE and not line.endswith(b"\n"):
                while (rest := handle.readline(MAX_LOG_LINE)) and not rest.endswith(b"\n"):
                    pass  # skip the rest of a too long line
                continue
            try:
                record = json.loads(line)
            except (ValueError, RecursionError):
                continue
            if not isinstance(record, dict):
                continue
            when = _when(record.get("time"))
            if when is None or not start <= when[1] <= today or record.get("effective") != "on":
                continue
            on += 1
            injected += record.get("injected") is True
    return on, injected


def summary(folder: Path, project: str, today: date | None = None) -> tuple[dict, list[dict]]:
    """`(panel, cited)` of one project: the FR-053 panel numbers and its top cited files with their stale state.

    `cited` = at most 30 `{path, cited, newest, stale}`, most cited first, ties by path. `today` is a local date.
    """
    today = today or datetime.now().astimezone().date()
    start = today - timedelta(days=DAYS - 1)
    days = {start + timedelta(days=i): 0 for i in range(DAYS)}
    notes, abstained = _read_notes(folder)
    cites: dict[str, list] = {}  # path -> [count, newest sort key, newest hash, newest local day]
    for when, files in notes:
        if when and when[1] in days:
            days[when[1]] += 1
        key, day = (when[0], when[1]) if when else (_OLDEST, None)
        for path, hash_ in files.items():
            entry = cites.get(path)
            if entry is None:
                cites[path] = [1, key, hash_, day]
                continue
            entry[0] += 1
            if key >= entry[1]:  # notes run oldest first, so a tie goes to the later file
                entry[1:] = [key, hash_, day]
    root = os.path.realpath(project)
    inside = []
    for path, entry in cites.items():
        real = _inside(root, path)
        if real is not None:
            inside.append((-entry[0], path, real, entry))
    inside.sort(key=lambda t: (t[0], t[1]))
    cited = [
        {"path": path, "cited": entry[0], "newest": entry[3].isoformat() if entry[3] else "", "stale": _state(real, entry[2])}
        for _, path, real, entry in inside[:MAX_NODES]
    ]
    on, injected = _prompts(folder, start, today)
    panel = {
        "notes": len(notes),
        "abstained": abstained,
        "days": [{"date": d.isoformat(), "count": n} for d, n in days.items()],
        "prompts_on": on,
        "injected": injected,
        "files_cited": len(cites),
        "files_not_drawn": len(cites) - len(cited),
        "outside": len(cites) - len(inside),
    }
    return panel, cited


def _count(value) -> bool:
    """A whole number of at least 0; a bool is not a number here."""
    return isinstance(value, int) and not isinstance(value, bool) and value >= 0


def _number(value) -> bool:
    """A finite int or float; a bool is not a number here."""
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def _group(raw) -> dict | None:
    """`{sessions, tokens_all_median}` of one report group, or None when a field has the wrong type.

    The median is null for a group with no sessions (easy-memory writes it so); anywhere else it must be a number.
    """
    if not isinstance(raw, dict):
        return None
    sessions, median = raw.get("sessions"), raw.get("tokens_all_median")
    if not _count(sessions):
        return None
    if median is None and sessions == 0:
        return {"sessions": 0, "tokens_all_median": None}
    return {"sessions": sessions, "tokens_all_median": median} if _number(median) and median >= 0 else None


def report(folder: Path) -> dict:
    """The on-vs-off comparison of a store folder's `report.json` (FR-065). Never raises.

    `{"state": "ok", generated, min_sessions, too_few, on_vs_off_pct, on, off}` with only those fields;
    `{"state": "unsupported"}` for a format newer than this reader; `{"state": "none"}` for a missing, unreadable,
    too big (64 KB), malformed or wrongly typed file, and for a symlink that leaves the store folder.
    """
    none = {"state": "none"}
    try:
        root = os.path.realpath(folder)
        real = os.path.realpath(os.path.join(root, "report.json"))
        if os.path.dirname(real) != root and not real.startswith(root + os.sep):
            return none
        with open(real, "rb") as handle:
            if not stat.S_ISREG(os.fstat(handle.fileno()).st_mode):
                return none
            raw = handle.read(MAX_REPORT_BYTES + 1)
        data = json.loads(raw.decode("utf-8")) if len(raw) <= MAX_REPORT_BYTES else None
    except (OSError, ValueError, RecursionError):  # UnicodeDecodeError is a ValueError
        return none
    if not isinstance(data, dict):
        return none
    version = data.get("v")
    if not _number(version):
        return none
    if version > REPORT_VERSION:
        return {"state": "unsupported"}
    groups = data.get("groups")
    on, off = (_group(groups.get(name)) for name in ("on", "off")) if isinstance(groups, dict) else (None, None)
    generated, minimum, too_few = data.get("generated"), data.get("min_sessions"), data.get("too_few")
    percent = data.get("on_vs_off_pct", "missing")
    if version != REPORT_VERSION or not isinstance(version, int) or on is None or off is None:
        return none
    if not isinstance(generated, str) or len(generated) > MAX_GENERATED_CHARS:
        return none
    if not _count(minimum) or minimum < 1 or not isinstance(too_few, bool):
        return none
    if percent is not None and not _number(percent):
        return none
    return {"state": "ok", "generated": generated, "min_sessions": minimum, "too_few": too_few, "on_vs_off_pct": percent, "on": on, "off": off}
