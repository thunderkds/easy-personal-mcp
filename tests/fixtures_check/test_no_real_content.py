"""HUN-32: fixtures hold only invented values (no real paths, ids or content)."""
import getpass
import json
import os
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "fixtures" / "claude_code"
FILES = sorted(ROOT.rglob("*.jsonl"))
SYNTH_SID = re.compile(r"^00000000-0000-4000-8000-\d{12}$")
UUID = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
KNOWN = {"user", "assistant", "attachment", "system", "agent-name", "custom-title",
         "last-prompt", "atis-latch", "queue-operation", "cost-state",
         "file-history-snapshot", "artifact-comment-monitor", "frame-link"}


def _records(path):
    """Yield (parsed_or_None, is_last_line_without_newline)."""
    raw = path.read_text()
    lines = raw.split("\n")
    for i, ln in enumerate(lines):
        if not ln:
            continue
        last_partial = i == len(lines) - 1 and not raw.endswith("\n")
        try:
            yield json.loads(ln), last_partial
        except json.JSONDecodeError:
            yield None, last_partial


def test_fixtures_exist_per_version():
    versions = {p.relative_to(ROOT).parts[0] for p in FILES}
    assert {"2.1.293", "2.1.295", "2.1.302"} <= versions


def test_no_home_path_or_username():
    needles = [str(Path.home()), "/home/", getpass.getuser(), os.path.expanduser("~")]
    for p in FILES:
        text = p.read_text()
        for n in needles:
            assert n not in text, f"{p}: contains {n!r}"


def test_session_ids_are_synthetic_and_not_in_scan_output():
    real = set()
    ids_file = os.environ.get("EPM_SCAN_IDS")  # optional uncommitted scan output
    if ids_file and Path(ids_file).is_file():
        real = set(Path(ids_file).read_text().split())
    for p in FILES:
        for rec, _ in _records(p):
            if rec and "sessionId" in rec:
                assert SYNTH_SID.match(rec["sessionId"]), f"{p}: non-synthetic sessionId"
                assert rec["sessionId"] not in real
        for u in UUID.findall(p.read_text()):
            assert u not in real


def test_cwd_is_invented():
    for p in FILES:
        for rec, _ in _records(p):
            if rec and "cwd" in rec:
                assert rec["cwd"].startswith("/work/")


def test_each_version_has_unknown_type_and_truncated_last_line():
    for vdir in ("2.1.293", "2.1.295", "2.1.302"):
        recs, truncated = [], 0
        for p in (ROOT / vdir).rglob("*.jsonl"):
            for rec, partial in _records(p):
                if rec is None:
                    assert partial, f"{p}: unparseable line that is not the last partial line"
                    truncated += 1
                else:
                    recs.append(rec)
        assert any(r.get("type") not in KNOWN for r in recs), vdir
        assert truncated >= 1, vdir

