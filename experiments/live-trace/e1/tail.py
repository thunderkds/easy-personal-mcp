"""E1 (HUN-166): live tail of Claude Code transcripts, metadata events only.

Polls the account's `projects/` tree (main files `projects/<proj>/<session>.jsonl` and subagent
files `projects/<proj>/<session>/subagents/*.jsonl`) with `os.scandir` + size/inode checks, reads
new complete lines with the repo's reader (`easy_personal.sources.claude_code.read_complete_lines`;
a half-written last line is never yielded) and maps each record with `parse_record` (read-only
import; `src/` is not changed). It emits one JSON line per tool_use block, per tool_result block
and per assistant record that has `usage`. A record `parse_record` abstains on is counted
(`stats["abstained"]`, per record type in `stats["abstained_by_type"]`) and emits nothing.

Event fields (names, ids, numbers, timestamps only; never text, tool input or tool output):
  kind, session_id, project, file_kind (main|sub), is_sidechain, record_ts, recv_utc, recv_ms,
  tool, tool_use_id, is_error (tool_result), model, message_id, tokens_in, tokens_out,
  cache_creation, cache_read, cost_usd (always null: transcripts hold no cost number),
  mtime_ms (file mtime read right after the line was read; diagnostic only).

Usage:
  python tail.py [--projects DIR] [--poll-ms 100] [--out FILE] [--state FILE]
                 [--from-start] [--duration SEC]
Library (HUN-170 combined run):
  h = start_tail(projects_dir, out_path, poll_ms=100)   # background thread, appends events to out_path
  ... run the session ...
  stats = h.stop()                                       # final poll, joins, returns stats
  score(read_events(out_path), reference)                # capture n/N + latency median / p95
"""
import argparse
import json
import os
import statistics
import sys
import threading
import time
from datetime import UTC, datetime
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "src"))
from easy_personal.sources.claude_code import parse_record, read_complete_lines  # noqa: E402

DEFAULT_PROJECTS = Path(
    os.environ.get("CLAUDE_CONFIG_DIR", "/home/hungnguyenhuu/.claude-hung2")
) / "projects"
EVENT_KINDS = ("tool_use", "tool_result", "usage")


def scan(projects: Path) -> dict[str, tuple[int, int, str]]:
    """{path: (inode, size, file_kind)} for every main and subagent *.jsonl under `projects`."""
    found: dict[str, tuple[int, int, str]] = {}

    def add(entry, kind):
        try:
            st = entry.stat()
        except OSError:
            return  # file vanished between listing and stat
        found[entry.path] = (st.st_ino, st.st_size, kind)

    try:
        proj_dirs = [e for e in os.scandir(projects) if e.is_dir()]
    except OSError:
        return found
    for pd in proj_dirs:
        try:
            entries = list(os.scandir(pd.path))
        except OSError:
            continue
        for e in entries:
            if e.name.endswith(".jsonl") and e.is_file():
                add(e, "main")
            elif e.is_dir():
                sub = os.path.join(e.path, "subagents")
                if os.path.isdir(sub):
                    try:
                        for s in os.scandir(sub):
                            if s.name.endswith(".jsonl") and s.is_file():
                                add(s, "sub")
                    except OSError:
                        continue
    return found


def _int(value):
    return value if type(value) is int else None


def events_from_record(record, parsed, file_kind, recv_ms, recv_utc) -> list[dict]:
    """Normalized metadata events for one decoded record. `parsed` is parse_record's event."""
    base = {
        "session_id": parsed["session_id"],
        "project": parsed["project_path"],
        "file_kind": file_kind,
        "is_sidechain": bool(parsed["is_sidechain"]),
        "record_ts": record.get("timestamp") if isinstance(record.get("timestamp"), str) else None,
        "recv_utc": recv_utc,
        "recv_ms": recv_ms,
    }
    out: list[dict] = []
    message = record.get("message")
    content = message.get("content") if isinstance(message, dict) else None
    for block in content if isinstance(content, list) else []:
        if not isinstance(block, dict):
            continue
        if record.get("type") == "assistant" and block.get("type") == "tool_use":
            tid, name = block.get("id"), block.get("name")
            if isinstance(tid, str) and isinstance(name, str):
                out.append({**base, "kind": "tool_use", "tool": name, "tool_use_id": tid})
        elif record.get("type") == "user" and block.get("type") == "tool_result":
            tid = block.get("tool_use_id")
            if isinstance(tid, str):
                err = block.get("is_error")
                out.append({**base, "kind": "tool_result", "tool_use_id": tid,
                            "is_error": err if isinstance(err, bool) else None})
    if parsed["message_id"] and parsed["input_tokens"] is not None:
        out.append({**base, "kind": "usage", "model": parsed["model"],
                    "message_id": parsed["message_id"], "tokens_in": parsed["input_tokens"],
                    "tokens_out": parsed["output_tokens"],
                    "cache_creation": parsed["cache_creation_input_tokens"],
                    "cache_read": parsed["cache_read_input_tokens"], "cost_usd": None})
    return out


class Tailer:
    """Follows transcripts under `projects`; calls `sink(event)` for each event.

    A file present at the first poll starts at its end (tail -f) unless `from_start`; a file that
    appears later starts at 0. `offsets` ({path: byte offset}) resumes a previous run; it is
    saved to `state_path` (offsets and inodes only) by `save_state()`.
    """

    def __init__(self, projects, sink, from_start=False, state_path=None):
        self.projects = Path(projects)
        self.sink = sink
        self.from_start = from_start
        self.state_path = Path(state_path) if state_path else None
        self.offsets: dict[str, int] = {}
        self.inodes: dict[str, int] = {}
        self._first = True
        self.stats = {"polls": 0, "records": 0, "events": 0, "abstained": 0,
                      "abstained_by_type": {}, "bad_json": 0, "files_seen": 0,
                      "subagent_files_seen": 0, "scan_ms_max": 0.0, "scan_ms_sum": 0.0,
                      "final_poll_events": 0}
        self._sub_seen: set[str] = set()
        if self.state_path and self.state_path.exists():
            saved = json.loads(self.state_path.read_text())
            self.offsets = {k: int(v[1]) for k, v in saved.items()}
            self.inodes = {k: int(v[0]) for k, v in saved.items()}
            self._first = False  # resumed: unknown files start at 0

    def save_state(self):
        if self.state_path:
            data = {p: [self.inodes[p], off] for p, off in self.offsets.items()}
            self.state_path.write_text(json.dumps(data))

    def poll_once(self) -> int:
        """One scan + read pass; returns the number of events emitted."""
        t0 = time.monotonic()
        files = scan(self.projects)
        scan_ms = (time.monotonic() - t0) * 1000
        self.stats["scan_ms_max"] = max(self.stats["scan_ms_max"], scan_ms)
        self.stats["scan_ms_sum"] += scan_ms
        self.stats["polls"] += 1
        emitted = 0
        for path, (inode, size, kind) in files.items():
            if kind == "sub" and path not in self._sub_seen:
                self._sub_seen.add(path)
                if not self._first:
                    self.stats["subagent_files_seen"] += 1  # appeared while tailing
            known = path in self.offsets
            if known and self.inodes[path] != inode:
                known = False  # file replaced: read the new one from 0
            if not known:
                self.stats["files_seen"] += 1
                self.inodes[path] = inode
                self.offsets[path] = size if (self._first and not self.from_start) else 0
            elif size < self.offsets[path]:
                self.offsets[path] = 0  # truncated
            if size <= self.offsets[path]:
                continue
            emitted += self._read(Path(path), kind)
        self._first = False
        return emitted

    def _read(self, path: Path, kind: str) -> int:
        emitted = 0
        try:
            for _off, line, nxt in read_complete_lines(path, self.offsets[str(path)]):
                recv_ms = time.time() * 1000.0
                try:
                    mtime_ms = os.stat(path).st_mtime * 1000.0  # diagnostic: when the file last changed
                except OSError:
                    mtime_ms = None
                self.offsets[str(path)] = nxt
                self.stats["records"] += 1
                try:
                    record = json.loads(line)
                except ValueError:
                    self.stats["bad_json"] += 1
                    continue
                parsed = parse_record(record, lambda cwd: cwd)
                if parsed["abstained"]:
                    self.stats["abstained"] += 1
                    rt = parsed["record_type"] or "?"
                    by = self.stats["abstained_by_type"]
                    by[rt] = by.get(rt, 0) + 1
                    continue
                recv_utc = datetime.fromtimestamp(recv_ms / 1000.0, tz=UTC).isoformat(timespec="milliseconds")
                for ev in events_from_record(record, parsed, kind, recv_ms, recv_utc):
                    ev["mtime_ms"] = mtime_ms
                    self.sink(ev)
                    emitted += 1
                del record, parsed
        except OSError:
            pass  # file deleted mid-read: next poll drops it
        self.stats["events"] += emitted
        return emitted

    def run(self, stop: threading.Event, poll_ms: float = 100.0):
        interval = poll_ms / 1000.0
        while not stop.is_set():
            t0 = time.monotonic()
            self.poll_once()
            stop.wait(max(0.0, interval - (time.monotonic() - t0)))


class TailHandle:
    def __init__(self, tailer, stop, thread, fh):
        self.tailer, self._stop, self._thread, self._fh = tailer, stop, thread, fh

    def stop(self) -> dict:
        """Stop the thread, do one final poll, close the log, return stats."""
        self._stop.set()
        self._thread.join()
        before = self.tailer.stats["kept_events"]
        self.tailer.poll_once()
        self.tailer.stats["final_poll_events"] = self.tailer.stats["kept_events"] - before
        self.tailer.save_state()
        self._fh.close()
        return self.tailer.stats


def start_tail(projects=DEFAULT_PROJECTS, out_path=None, poll_ms=100.0, from_start=False,
               state_path=None, keep=None) -> TailHandle:
    """Start tailing in a background thread; events are appended to `out_path` as JSON lines.

    The first scan (baseline of existing files) runs before this returns, so a session started
    right after is never missed. `keep(event) -> bool` filters what is written (default: all).
    """
    fh = open(out_path, "a", buffering=1) if out_path else open(os.devnull, "w")
    def sink(ev):
        if keep is None or keep(ev):
            fh.write(json.dumps(ev) + "\n")
            tailer.stats["kept_events"] += 1

    tailer = Tailer(projects, sink, from_start, state_path)
    tailer.stats["kept_events"] = 0
    tailer.poll_once()
    stop = threading.Event()
    thread = threading.Thread(target=tailer.run, args=(stop, poll_ms), daemon=True)
    thread.start()
    return TailHandle(tailer, stop, thread, fh)


def read_events(path) -> list[dict]:
    return [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]


def _p95(values):
    s = sorted(values)
    return s[min(len(s) - 1, round(0.95 * (len(s) - 1)))] if s else None


def score(events, reference, to_epoch_ms=None) -> dict:
    """Capture n/N and latency vs the transcript reference.

    reference: {tool_use_id: (tool_name, iso_ts)} (harness `read_reference`). Latency per tool_use
    = first recv_ms for that id minus the reference timestamp. Event ids not in the reference
    are counted in `extra`.
    """
    if to_epoch_ms is None:
        to_epoch_ms = lambda ts: datetime.fromisoformat(ts).timestamp() * 1000.0  # noqa: E731
    first: dict[str, float] = {}
    for ev in events:
        if ev["kind"] == "tool_use":
            first.setdefault(ev["tool_use_id"], ev["recv_ms"])
    lat = {tid: first[tid] - to_epoch_ms(ts) for tid, (_n, ts) in reference.items() if tid in first}
    vals = list(lat.values())
    return {
        "captured": len(lat), "reference_total": len(reference),
        "missed": sorted(set(reference) - set(first)), "extra": len(set(first) - set(reference)),
        "dup_tool_use_events": sum(1 for e in events if e["kind"] == "tool_use") - len(first),
        "latency_ms": {tid: round(v, 1) for tid, v in lat.items()},
        "median_ms": round(statistics.median(vals), 1) if vals else None,
        "p95_ms": round(_p95(vals), 1) if vals else None,
        "min_ms": round(min(vals), 1) if vals else None,
        "max_ms": round(max(vals), 1) if vals else None,
    }


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--projects", default=str(DEFAULT_PROJECTS))
    ap.add_argument("--poll-ms", type=float, default=100.0)
    ap.add_argument("--out", help="append events here (default: stdout)")
    ap.add_argument("--state", help="offsets file: resume from it and save to it on exit")
    ap.add_argument("--from-start", action="store_true", help="read existing files from byte 0")
    ap.add_argument("--duration", type=float, help="stop after this many seconds (default: Ctrl-C)")
    args = ap.parse_args()
    fh = open(args.out, "a", buffering=1) if args.out else sys.stdout
    tailer = Tailer(args.projects, lambda ev: (fh.write(json.dumps(ev) + "\n"), fh.flush()),
                    args.from_start, args.state)
    stop = threading.Event()
    if args.duration:
        threading.Timer(args.duration, stop.set).start()
    try:
        tailer.run(stop, args.poll_ms)
    except KeyboardInterrupt:
        pass
    finally:
        tailer.poll_once()
        tailer.save_state()
        print(json.dumps(tailer.stats), file=sys.stderr)


if __name__ == "__main__":
    main()
