import json
import os
import subprocess
from datetime import datetime

import pytest

from easy_personal.core import kit
from easy_personal.sources import git_local

CANARY = "LEAKCANARY-commit-message-0123456789"
# Local noon on three dates: the day bucket is the OS local day, so noon is safe in any timezone.
D1, D2 = datetime(2026, 3, 10, 12, 0), datetime(2026, 3, 11, 12, 0)


def _git(repo, *args, when=None):
    env = {**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_SYSTEM": "/dev/null"}
    if when:
        stamp = f"{int(when.timestamp())} +0000"
        env |= {"GIT_AUTHOR_DATE": stamp, "GIT_COMMITTER_DATE": stamp}
    cmd = ["git", "-c", "user.email=t@example.invalid", "-c", "user.name=t", "-C", str(repo), *args]
    return subprocess.run(cmd, check=True, capture_output=True, text=True, env=env)


def _commit(repo, when, msg=CANARY):
    _git(repo, "commit", "-q", "--allow-empty", "-m", msg, when=when)


@pytest.fixture
def repo(tmp_path):
    path = tmp_path / "repo"
    path.mkdir()
    _git(path, "init", "-q", "-b", "main")
    return path


def _day(d):
    return d.astimezone().strftime("%Y-%m-%d")


def test_commits_per_local_day_match_git_log(repo):
    _commit(repo, D1)
    _commit(repo, D1.replace(hour=13))
    _commit(repo, D2)
    got = git_local.commits_by_day(str(repo))
    assert got == {_day(D1): 2, _day(D2): 1}
    log = _git(repo, "log", "--all", "--format=%ct").stdout.split()
    assert sum(got.values()) == len(log)


def test_all_branches_and_worktree_counted_once(repo, tmp_path):
    _commit(repo, D1)
    _git(repo, "checkout", "-q", "-b", "feature")
    _commit(repo, D2)
    _git(repo, "checkout", "-q", "main")
    wt = tmp_path / "wt"
    _git(repo, "worktree", "add", "-q", str(wt), "-b", "wtb")
    _commit(wt, D2, msg="on worktree branch")  # distinct message, else the hash equals feature's
    assert git_local.commits_by_day(str(repo)) == {_day(D1): 1, _day(D2): 2}


def test_shallow_clone_counts_what_it_has(repo, tmp_path):
    for d in (D1, D2):
        _commit(repo, d)
    shallow = tmp_path / "shallow"
    subprocess.run(
        ["git", "clone", "-q", "--depth", "1", f"file://{repo}", str(shallow)],
        check=True, capture_output=True,
    )  # fmt: skip
    assert git_local.commits_by_day(str(shallow)) == {_day(D2): 1}


def test_not_a_repo_empty_repo_missing_folder_are_no_data(repo, tmp_path):
    assert git_local.commits_by_day(str(repo)) is None  # no commits
    plain = tmp_path / "plain"
    plain.mkdir()
    assert git_local.commits_by_day(str(plain)) is None
    assert git_local.commits_by_day(str(tmp_path / "gone")) is None


def test_only_local_read_commands_with_hardening(repo, monkeypatch):
    _commit(repo, D1)
    calls = []
    real = subprocess.run

    def spy(cmd, **kw):
        calls.append((cmd, kw))
        return real(cmd, **kw)

    monkeypatch.setattr(subprocess, "run", spy)
    git_local.commits_by_day(str(repo))
    assert calls
    for cmd, kw in calls:
        assert cmd[cmd.index("-C") + 2] in {"log", "rev-list"}
        assert not {"fetch", "pull", "push", "ls-remote", "clone", "remote"} & set(cmd)
        assert "core.fsmonitor=false" in cmd
        assert kw["env"]["GIT_OPTIONAL_LOCKS"] == "0"
        assert kw["timeout"]


def test_timeout_abstains(repo, monkeypatch):
    _commit(repo, D1)

    def boom(cmd, **kw):
        raise subprocess.TimeoutExpired(cmd, kw["timeout"])

    monkeypatch.setattr(subprocess, "run", boom)
    assert git_local.git_part(str(repo)) == {"status": "abstained"}


def test_kit_summary_git_part(conn, repo):
    conn.execute(
        "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, "
        "record_type, project_path) VALUES ('m', 'claude_code', 'f', 1, 0, 1, 'x', ?)",
        (str(repo),),
    )
    parts = kit.kit_summary(conn)["projects"][0]["parts"]
    assert parts["git"] == {"status": "no data"}
    _commit(repo, D1)
    _commit(repo, D2)
    part = kit.kit_summary(conn)["projects"][0]["parts"]["git"]
    assert part == {"status": "ok", "commits": 2, "commits_by_day": {_day(D1): 1, _day(D2): 1}}
    assert CANARY not in json.dumps(kit.kit_summary(conn))
