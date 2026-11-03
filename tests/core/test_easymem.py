"""easy-memory store reader (HUN-184, FR-051..053): metadata only, read-only, tolerant of bad files."""

import hashlib
import json
import os
import time
from datetime import date

import pytest

from easy_personal.core import easymem

TODAY = date(2026, 11, 2)
PROMPT, ANSWER, SESSION = "INVENTED_prompt_text", "INVENTED_answer_text", "INVENTED_session_id"


def sha(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def key_of(identity) -> str:
    return hashlib.sha256(os.path.realpath(identity).encode()).hexdigest()[:16]


def note_body(time_text="2026-11-01T10:00:00Z", files=None, **extra):
    return {"v": 1, "id": "x", "time": time_text, "prompt": PROMPT, "answer": ANSWER, "session_id": SESSION,
            "files": files if files is not None else {}, **extra}  # fmt: skip


class Fixture:
    """A temp easy-memory home, one repo and its store folder."""

    def __init__(self, tmp_path, monkeypatch):
        self.home = tmp_path / "em-home"
        self.home.mkdir()
        monkeypatch.setenv("EASY_MEMORY_HOME", str(self.home))
        self.repo = tmp_path / "work" / "alpha"
        (self.repo / ".git").mkdir(parents=True)
        self.folder = self.home / key_of(self.repo / ".git")
        (self.folder / "notes").mkdir(parents=True)
        self.count = 0

    def meta(self, path=None):
        (self.folder / "meta.json").write_text(json.dumps({"v": 1, "path": str(path or self.repo / ".git")}))

    def note(self, body=None, raw=None, **kw):
        self.count += 1
        target = self.folder / "notes" / f"n{self.count:04d}.json"
        if raw is not None:
            target.write_bytes(raw)
        else:
            target.write_text(json.dumps(body if body is not None else note_body(**kw)))
        return target

    def log(self, *lines):
        (self.folder / "prompts.log").write_text("".join(line if isinstance(line, str) else json.dumps(line) + "\n" for line in lines))

    def find(self):
        return easymem.find(easymem.load(), str(self.repo))

    def summary(self):
        return easymem.summary(self.folder, str(self.repo), today=TODAY)


@pytest.fixture
def fx(tmp_path, monkeypatch):
    return Fixture(tmp_path, monkeypatch)


# --- AC1: where the store is, and a missing one --------------------------------------------------

def test_store_home_env_empty_and_unset(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_MEMORY_HOME", str(tmp_path / "x"))
    assert easymem.store_home() == tmp_path / "x"
    monkeypatch.setenv("EASY_MEMORY_HOME", "")
    monkeypatch.setenv("HOME", str(tmp_path / "h"))
    assert easymem.store_home() == tmp_path / "h" / ".easy-memory"
    monkeypatch.delenv("EASY_MEMORY_HOME")
    assert easymem.store_home() == tmp_path / "h" / ".easy-memory"


def test_missing_store_folder_finds_nothing(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_MEMORY_HOME", str(tmp_path / "nope"))
    assert easymem.find(easymem.load(), str(tmp_path)) is None


# --- AC2: which folder belongs to a project -------------------------------------------------------

def test_meta_path_ties_the_folder_to_the_repo(fx):
    fx.meta()
    assert fx.find() == fx.folder


def test_meta_path_through_a_symlink(fx, tmp_path):
    other = fx.home / "ffffffffffffffff"
    (other / "notes").mkdir(parents=True)
    link = tmp_path / "link-to-repo"
    link.symlink_to(fx.repo, target_is_directory=True)
    (other / "meta.json").write_text(json.dumps({"v": 1, "path": str(link / ".git")}))
    assert easymem.find(easymem.load(), str(fx.repo)) == other


def test_meta_path_of_a_non_git_folder(tmp_path, monkeypatch):
    fx = Fixture(tmp_path, monkeypatch)
    plain = tmp_path / "plain"
    plain.mkdir()
    folder = fx.home / key_of(plain)
    folder.mkdir()
    (folder / "meta.json").write_text(json.dumps({"v": 1, "path": str(plain)}))
    assert easymem.find(easymem.load(), str(plain)) == folder


def test_folder_without_meta_is_found_by_the_computed_key(fx):
    assert not (fx.folder / "meta.json").exists()
    assert fx.find() == fx.folder


def test_worktree_project_uses_the_common_dir_key(fx, tmp_path):
    """A worktree's `.git` file is followed to the main repo's common dir; no git subprocess runs."""
    wt = tmp_path / "work" / "alpha-wt"
    gitdir = fx.repo / ".git" / "worktrees" / "wt"
    gitdir.mkdir(parents=True)
    (gitdir / "commondir").write_text("../..\n")
    wt.mkdir()
    (wt / ".git").write_text(f"gitdir: {gitdir}\n")
    assert easymem.find(easymem.load(), str(wt)) == fx.folder


def test_worktree_without_commondir_file_uses_its_gitdir(fx, tmp_path):
    wt = tmp_path / "wt2"
    gitdir = tmp_path / "gitdirs" / "wt2"
    gitdir.mkdir(parents=True)
    wt.mkdir()
    (wt / ".git").write_text("gitdir: ../gitdirs/wt2\n")
    folder = fx.home / key_of(gitdir)
    folder.mkdir()
    assert easymem.find(easymem.load(), str(wt)) == folder


def test_bad_meta_json_falls_back_to_the_key(fx):
    (fx.folder / "meta.json").write_text("{not json")
    assert fx.find() == fx.folder


def test_folder_with_another_meta_path_is_not_matched_by_name(fx, tmp_path):
    fx.meta(path=tmp_path / "elsewhere" / ".git")
    assert fx.find() is None


def test_trash_is_never_listed_or_counted(fx):
    fx.note(files={"a.py": "x"})
    (fx.folder / "trash").mkdir()
    (fx.folder / "trash" / "t.json").write_text(json.dumps(note_body(prompt="INVENTED_TRASH")))
    panel, _ = fx.summary()
    assert panel["notes"] == 1
    assert "INVENTED_TRASH" not in json.dumps(panel)


# --- AC3: abstained notes -------------------------------------------------------------------------

@pytest.mark.parametrize(
    "raw",
    [
        b"[]", b'"text"', b"{bad json", b"\xff\xfe\x00 not utf8",
        json.dumps({"time": "2026-11-01T10:00:00Z", "files": {}}).encode(),  # no v
        json.dumps(note_body(v=True)).encode(),
        json.dumps(note_body(v=2)).encode(),
        json.dumps(note_body(v="1")).encode(),
        json.dumps(note_body(files=[])).encode(),
        json.dumps(note_body(files="x")).encode(),
        b"[" * 100000,  # deep nesting must not raise
    ],
)  # fmt: skip
def test_bad_notes_are_abstained_never_counted(fx, raw):
    fx.note(raw=raw)
    fx.note(files={"a.py": "x"})
    panel, cited = fx.summary()
    assert panel["notes"] == 1 and panel["abstained"] == 1
    assert [c["path"] for c in cited] == ["a.py"]


def test_oversized_note_is_abstained(fx):
    fx.note(raw=json.dumps(note_body(prompt="x" * (1024 * 1024 + 1))).encode())
    panel, _ = fx.summary()
    assert panel["notes"] == 0 and panel["abstained"] == 1


def test_unreadable_notes_folder_gives_zero(fx):
    (fx.folder / "notes").rmdir()
    panel, cited = fx.summary()
    assert panel["notes"] == 0 and panel["abstained"] == 0 and cited == []


# --- AC4: top 30, ties by path ---------------------------------------------------------------------

def test_top_thirty_by_citation_ties_by_path(fx):
    for n in range(35):
        (fx.repo / f"f{n:02d}.txt").write_text("x")
    # f00..f34 cited once; f34 cited twice -> first, then the 29 lowest paths
    fx.note(files={f"f{n:02d}.txt": sha("x") for n in range(35)})
    fx.note(files={"f34.txt": sha("x")})
    panel, cited = fx.summary()
    paths = [c["path"] for c in cited]
    assert paths[0] == "f34.txt" and cited[0]["cited"] == 2
    assert paths[1:] == [f"f{n:02d}.txt" for n in range(29)]
    assert len(cited) == 30
    assert panel["files_cited"] == 35 and panel["files_not_drawn"] == 5 and panel["outside"] == 0


# --- AC5: stale ------------------------------------------------------------------------------------

def test_stale_states(fx):
    (fx.repo / "fresh.txt").write_text("same")
    (fx.repo / "changed.txt").write_text("new")
    (fx.repo / "adir").mkdir()
    (fx.repo / "big.bin").write_bytes(b"0" * (10 * 1024 * 1024 + 1))
    (fx.repo / "noperm.txt").write_text("secret")
    (fx.repo / "noperm.txt").chmod(0)
    fx.note(files={"fresh.txt": sha("same"), "changed.txt": sha("old"), "gone.txt": sha("x"), "adir": "h", "big.bin": "h", "noperm.txt": "h"})
    try:
        _, cited = fx.summary()
    finally:
        (fx.repo / "noperm.txt").chmod(0o600)
    state = {c["path"]: c["stale"] for c in cited}
    assert state["fresh.txt"] == "fresh" and state["changed.txt"] == "stale" and state["gone.txt"] == "gone"
    assert state["adir"] == "unchecked" and state["big.bin"] == "unchecked"
    if os.geteuid() != 0:
        assert state["noperm.txt"] == "unchecked"


def test_stale_compares_the_newest_citing_note(fx):
    (fx.repo / "a.txt").write_text("v2")
    fx.note(time_text="2026-10-01T10:00:00Z", files={"a.txt": sha("v1")})
    fx.note(time_text="2026-10-20T10:00:00Z", files={"a.txt": sha("v2")})
    fx.note(time_text="2026-10-10T10:00:00Z", files={"a.txt": sha("v1")})  # saved later, but older by `time`
    _, cited = fx.summary()
    assert cited[0]["stale"] == "fresh" and cited[0]["cited"] == 3
    assert cited[0]["newest"] == "2026-10-20"


# --- AC6: paths that leave the project are never opened ---------------------------------------------

def test_outside_paths_are_never_opened(fx, tmp_path, monkeypatch):
    outside = tmp_path / "outside.txt"
    outside.write_text("x")
    (fx.repo / "escape").symlink_to(outside)
    (fx.repo / "dirlink").symlink_to(tmp_path, target_is_directory=True)
    fx.note(files={str(outside): "h", "../outside.txt": "h", "a/../../outside.txt": "h", "escape": "h", "dirlink/outside.txt": "h", "ok.txt": "h"})
    (fx.repo / "ok.txt").write_text("ok")
    opened = []
    real_open = os.open
    real_builtin_open = open

    def spy_open(path, *a, **k):
        opened.append(str(path))
        return real_open(path, *a, **k)

    def spy_builtin(path, *a, **k):
        opened.append(str(path))
        return real_builtin_open(path, *a, **k)

    monkeypatch.setattr(os, "open", spy_open)
    monkeypatch.setattr("builtins.open", spy_builtin)
    panel, cited = fx.summary()
    monkeypatch.undo()
    assert not [p for p in opened if p.endswith(("outside.txt", "escape"))]
    assert [c["path"] for c in cited] == ["ok.txt"]
    assert panel["outside"] == 5 and panel["files_cited"] == 6 and panel["files_not_drawn"] == 5


# --- AC8: days -------------------------------------------------------------------------------------

def test_thirty_days_oldest_first_and_counts(fx):
    fx.note(time_text="2026-11-02T10:00:00Z")
    fx.note(time_text="2026-11-02T10:05:00Z")
    fx.note(time_text="2026-10-04T10:00:00Z")  # day 30 (oldest in the window)
    fx.note(time_text="2026-10-03T10:00:00Z")  # one day too old: notes only
    fx.note(time_text="not a time")  # counted in notes, in no day
    fx.note(body=note_body(time_text=None))
    panel, _ = fx.summary()
    days = panel["days"]
    assert len(days) == 30 and days[0]["date"] == "2026-10-04" and days[-1]["date"] == "2026-11-02"
    assert days[0]["count"] == 1 and days[-1]["count"] == 2
    assert sum(d["count"] for d in days) == 3 and panel["notes"] == 6


def test_days_are_local_and_dst_aware(fx, monkeypatch):
    # New York: 2026-11-01 is the fall-back day (EDT -> EST). 02:30Z is Oct 31 22:30 local; 05:30Z is Nov 1 00:30 local.
    monkeypatch.setenv("TZ", "America/New_York")
    time.tzset()
    try:
        fx.note(time_text="2026-11-01T02:30:00Z")
        fx.note(time_text="2026-11-01T05:30:00Z")
        fx.note(time_text="2026-11-02T04:59:00Z")  # Nov 1 23:59 local (EST)
        fx.note(time_text="2026-11-02T05:00:00Z")  # Nov 2 00:00 local
        panel, _ = fx.summary()
    finally:
        monkeypatch.undo()
        time.tzset()
    by_date = {d["date"]: d["count"] for d in panel["days"]}
    assert len(panel["days"]) == 30
    assert by_date["2026-10-31"] == 1 and by_date["2026-11-01"] == 2 and by_date["2026-11-02"] == 1


def test_default_today_is_the_local_date(fx):
    panel, _ = easymem.summary(fx.folder, str(fx.repo))
    assert panel["days"][-1]["date"] == time.strftime("%Y-%m-%d")


# --- AC9: hit rate ---------------------------------------------------------------------------------

def test_hit_rate_counts(fx):
    def line(t, effective, injected):
        return {"time": t, "session_id": SESSION, "configured": "auto", "effective": effective, "injected": injected}

    fx.log(
        line("2026-11-01T10:00:00Z", "on", True),
        line("2026-11-01T10:01:00Z", "on", False),
        line("2026-11-01T10:02:00Z", "on", "yes"),  # injected must be exactly true
        line("2026-11-01T10:03:00Z", "off", True),  # memory off: not a prompt with memory on
        line("2026-10-01T10:00:00Z", "on", True),  # outside the 30 days
        "this is not json\n",
        "[1, 2]\n",
        json.dumps({"time": "bad", "effective": "on", "injected": True}) + "\n",
        b"".decode(),
    )
    panel, _ = fx.summary()
    assert panel["prompts_on"] == 3 and panel["injected"] == 1


def test_no_prompts_log_or_no_prompts(fx):
    panel, _ = fx.summary()
    assert panel["prompts_on"] == 0 and panel["injected"] == 0
    fx.log()
    panel, _ = fx.summary()
    assert panel["prompts_on"] == 0


def test_long_and_binary_log_lines_are_skipped(fx):
    (fx.folder / "prompts.log").write_bytes(
        b"x" * 200_000 + b"\n" + b"\xff\xfe\n" + json.dumps({"time": "2026-11-01T10:00:00Z", "effective": "on", "injected": True}).encode() + b"\n"
    )
    panel, _ = fx.summary()
    assert panel["prompts_on"] == 1 and panel["injected"] == 1


# --- counting helpers ------------------------------------------------------------------------------

def test_note_count_and_has_notes(fx):
    assert not easymem.has_notes(fx.folder)
    fx.note(raw=b"{bad")
    assert easymem.has_notes(fx.folder) and easymem.note_count(fx.folder) == 0
    fx.note()
    assert easymem.note_count(fx.folder) == 1


# --- AC11 / AC12: no content out, nothing written ----------------------------------------------------

def test_returned_values_hold_no_note_text(fx):
    (fx.repo / "a.txt").write_text("x")
    fx.note(files={"a.txt": sha("x")})
    fx.log({"time": "2026-11-01T10:00:00Z", "session_id": SESSION, "effective": "on", "injected": True})
    text = json.dumps(fx.summary())
    for secret in (PROMPT, ANSWER, SESSION):
        assert secret not in text


def test_nothing_is_written(fx):
    fx.meta()
    (fx.repo / "a.txt").write_text("x")
    fx.note(files={"a.txt": sha("x")})
    fx.log({"time": "2026-11-01T10:00:00Z", "effective": "on", "injected": True})
    (fx.folder / "trash").mkdir()

    def snapshot():
        out = {}
        for root in (fx.home, fx.repo):
            for here, dirs, files in os.walk(root):
                for name in dirs + files:
                    path = os.path.join(here, name)
                    out[path] = os.lstat(path).st_mtime_ns
        return out

    before = snapshot()
    easymem.find(easymem.load(), str(fx.repo))
    fx.summary()
    easymem.note_count(fx.folder)
    assert snapshot() == before


# --- HUN-198 (FR-065): report.json, the on-vs-off numbers easy-memory writes --------------------

REAL_REPORT = {
    "v": 1, "generated": "2026-11-03T10:06:31Z", "min_sessions": 10, "sessions_logged": 9, "no_data": 0,
    "no_claude_log": 1, "forced": 0,
    "groups": {"on": {"sessions": 3, "tokens_all_median": 18658261, "tokens_io_median": 129194, "turns_median": 177},
               "off": {"sessions": 5, "tokens_all_median": 16553412, "tokens_io_median": 100808, "turns_median": 169}},
    "note_opened": {"opened": 0, "indexed": 3}, "too_few": True, "on_vs_off_pct": None,
    "store": {"notes": 139, "bytes": 275959},
}  # fmt: skip


def put_report(fx, report=None, raw=None):
    target = fx.folder / "report.json"
    target.write_bytes(raw if raw is not None else json.dumps(REAL_REPORT if report is None else report).encode())
    return target


def edited(**groups_or_top):
    """A deep copy of the real report with top-level keys replaced; `on` / `off` replace that group."""
    report = json.loads(json.dumps(REAL_REPORT))
    for key, value in groups_or_top.items():
        if key in ("on", "off"):
            report["groups"][key] = value
        else:
            report[key] = value
    return report


def test_report_returns_exactly_the_panel_fields(fx):
    put_report(fx)
    assert easymem.report(fx.folder) == {
        "state": "ok", "generated": "2026-11-03T10:06:31Z", "min_sessions": 10, "too_few": True, "on_vs_off_pct": None,
        "on": {"sessions": 3, "tokens_all_median": 18658261}, "off": {"sessions": 5, "tokens_all_median": 16553412},
    }  # fmt: skip


def test_report_keeps_no_other_key_of_the_file(fx):
    put_report(fx, {**REAL_REPORT, "prompt": PROMPT, "answer": ANSWER, "session_id": SESSION})
    assert PROMPT not in json.dumps(easymem.report(fx.folder))
    assert set(easymem.report(fx.folder)) == {"state", "generated", "min_sessions", "too_few", "on_vs_off_pct", "on", "off"}


def test_report_with_a_saving_and_an_empty_group(fx):
    put_report(fx, edited(too_few=False, on_vs_off_pct=-12, off={"sessions": 0, "tokens_all_median": None}))
    got = easymem.report(fx.folder)
    assert got["state"] == "ok" and got["on_vs_off_pct"] == -12 and got["off"] == {"sessions": 0, "tokens_all_median": None}


@pytest.mark.parametrize("raw", [b"", b"[]", b"{", b'"text"', b"null", b"\xff\xfe\x00", b"{" * 100000], ids=["empty", "list", "cut", "string", "null", "binary", "deep"])
def test_report_not_an_object_is_none(fx, raw):
    put_report(fx, raw=raw)
    assert easymem.report(fx.folder) == {"state": "none"}


def test_report_missing_file_or_folder_is_none(fx, tmp_path):
    assert easymem.report(fx.folder) == {"state": "none"}
    assert easymem.report(tmp_path / "nowhere") == {"state": "none"}


def test_report_directory_instead_of_file_is_none(fx):
    (fx.folder / "report.json").mkdir()
    assert easymem.report(fx.folder) == {"state": "none"}


def test_report_over_64_kb_is_none(fx):
    padded = {**REAL_REPORT, "pad": "x" * (65 * 1024)}
    put_report(fx, padded)
    assert easymem.report(fx.folder) == {"state": "none"}


@pytest.mark.parametrize("version", [2, 99, 1.5])
def test_report_newer_version_is_unsupported(fx, version):
    put_report(fx, {"v": version})
    assert easymem.report(fx.folder) == {"state": "unsupported"}


def test_report_unsupported_carries_no_numbers(fx):
    put_report(fx, {**REAL_REPORT, "v": 2})
    assert easymem.report(fx.folder) == {"state": "unsupported"}


@pytest.mark.parametrize(
    "report",
    [
        {**REAL_REPORT, "v": 0}, {**REAL_REPORT, "v": True}, {**REAL_REPORT, "v": "1"},
        {k: v for k, v in REAL_REPORT.items() if k != "v"},
        edited(on={"sessions": "3", "tokens_all_median": 1}),
        edited(on={"sessions": True, "tokens_all_median": 1}),
        edited(on={"sessions": -1, "tokens_all_median": 1}),
        edited(on={"sessions": 2.5, "tokens_all_median": 1}),
        edited(on={"sessions": 3, "tokens_all_median": True}),
        edited(on={"sessions": 3, "tokens_all_median": "1"}),
        edited(on={"sessions": 3, "tokens_all_median": -5}),
        edited(on={"sessions": 3}),
        edited(on={"sessions": 3, "tokens_all_median": None}),
        edited(generated="x" * 65),
        edited(off=[1, 2]),
        edited(min_sessions=True), edited(min_sessions="10"), edited(min_sessions=0), edited(min_sessions=-3),
        edited(too_few="true"), edited(too_few=1), edited(too_few=None),
        edited(on_vs_off_pct="8"), edited(on_vs_off_pct=True), edited(on_vs_off_pct=float("nan")),
        edited(generated=5), edited(generated=None),
        {**REAL_REPORT, "groups": None}, {**REAL_REPORT, "groups": {"on": REAL_REPORT["groups"]["on"]}},
        {k: v for k, v in REAL_REPORT.items() if k != "too_few"},
        {k: v for k, v in REAL_REPORT.items() if k != "on_vs_off_pct"},
    ],
)  # fmt: skip
def test_report_a_wrong_field_is_none(fx, report):
    put_report(fx, report)
    assert easymem.report(fx.folder) == {"state": "none"}


def test_report_unparseable_generated_is_kept_as_text(fx):
    put_report(fx, edited(generated="not a time"))
    assert easymem.report(fx.folder)["generated"] == "not a time"  # the page shows no age; the block still shows


def test_report_symlink_inside_the_folder_is_read(fx):
    real = fx.folder / "elsewhere.json"
    real.write_text(json.dumps(REAL_REPORT))
    (fx.folder / "report.json").symlink_to(real)
    assert easymem.report(fx.folder)["state"] == "ok"


def test_report_symlink_outside_the_folder_is_none(fx, tmp_path):
    outside = tmp_path / "outside.json"
    outside.write_text(json.dumps(REAL_REPORT))
    (fx.folder / "report.json").symlink_to(outside)
    assert easymem.report(fx.folder) == {"state": "none"}


@pytest.mark.skipif(os.geteuid() == 0, reason="root can read a mode 0 file")
def test_report_never_raises_on_an_unreadable_file(fx):
    put_report(fx).chmod(0)
    assert easymem.report(fx.folder) == {"state": "none"}


def test_report_writes_nothing(fx):
    put_report(fx)
    before = sorted((p.name, p.stat().st_mtime_ns) for p in fx.folder.rglob("*"))
    easymem.report(fx.folder)
    assert sorted((p.name, p.stat().st_mtime_ns) for p in fx.folder.rglob("*")) == before
