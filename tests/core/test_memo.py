"""core/memo.py (HUN-227, NFR-013): share a costly store computation while the store is unchanged."""

import sqlite3
import subprocess
import sys
import threading
import time

import pytest

from easy_personal.core import accounts, memo, metrics, owners, store, workspaces

BASE = 1_767_225_600  # 2026-01-01T00:00:00Z
INSERT = (
    "INSERT INTO events (machine_id, source, source_path, inode, byte_offset, abstained, record_type,"
    " version, ts_utc, session_id, project_path, is_sidechain, is_api_error, model, message_id,"
    " git_branch, input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens)"
    " VALUES ('m','claude_code','/p.jsonl',1,?,0,'assistant','2.1',?,'s1',?,0,0,'model-a',?,?,10,5,0,0)"
)


def _ts(n: int) -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(BASE + n)) + ".000000Z"


def add_event(conn, n: int, project: str = "/work/app0", branch: str | None = None) -> None:
    conn.execute(INSERT, (n, _ts(n), project, f"msg{n}", branch))
    conn.commit()


@pytest.fixture(autouse=True)
def _fresh_memo():
    memo.reset()
    yield
    memo.reset()


@pytest.fixture
def path(tmp_path):
    return tmp_path / "store.sqlite"


@pytest.fixture
def conn(path):
    connection = store.connect(path)
    yield connection
    connection.close()


@pytest.fixture
def other(path, conn):
    """A second connection to the same store: the writer the memo must notice."""
    connection = store.connect(path)
    yield connection
    connection.close()


@pytest.fixture
def scope_calls(monkeypatch):
    """Counts real computations of `metrics._messages` (its scope resolution runs once per compute)."""
    calls = []
    real = metrics._scope

    def counting(*args, **kwargs):
        calls.append(args[1:])
        return real(*args, **kwargs)

    monkeypatch.setattr(metrics, "_scope", counting)
    return calls


@memo.memoized
def costly(conn, a, b=None):
    costly.calls.append((a, b))
    return conn.execute("SELECT COUNT(*) FROM events").fetchone()[0]


costly.calls = []


@pytest.fixture(autouse=True)
def _reset_costly():
    costly.calls.clear()


# --- AC 1: compute once per key --------------------------------------------------------------


def test_same_arguments_compute_once_different_arguments_separately(conn, scope_calls):
    add_event(conn, 1)
    first = metrics._messages(conn, None, None)
    assert metrics._messages(conn, None, None) == first
    assert (
        metrics._messages(conn, None, None, None, None) == first
    )  # a default spelled out is the same key
    assert len(scope_calls) == 1
    metrics._messages(conn, _ts(0), None)
    metrics._messages(conn, None, _ts(9))
    assert len(scope_calls) == 3


def test_workspace_and_account_are_part_of_the_key(conn, scope_calls):
    add_event(conn, 1)
    workspaces.add(conn, "acme", repos=["/work/app0"])
    assert len(metrics._messages(conn, None, None, "acme")) == 1
    assert len(metrics._messages(conn, None, None)) == 1
    assert len(scope_calls) == 2
    metrics._messages(conn, None, None, "acme")
    assert len(scope_calls) == 2


def test_two_stores_never_share_entries(tmp_path):
    a, b = store.connect(tmp_path / "a.sqlite"), store.connect(tmp_path / "b.sqlite")
    add_event(a, 1)
    assert len(metrics._messages(a, None, None)) == 1
    assert len(metrics._messages(b, None, None)) == 0
    a.close()
    b.close()


def test_unknown_workspace_and_account_raise_as_before(conn):
    with pytest.raises(ValueError):
        metrics._messages(conn, None, None, "nope")
    with pytest.raises(ValueError):
        metrics._messages(conn, None, None, "nope")  # an error is never cached as a result
    with pytest.raises(accounts.AccountError):
        metrics._messages(conn, None, None, None, "nope")


# --- AC 2: single flight ---------------------------------------------------------------------


def test_eight_threads_on_one_key_compute_once(conn, path):
    add_event(conn, 1)
    gate, started = threading.Event(), threading.Event()

    @memo.memoized
    def slow(c):
        started.set()
        gate.wait(5)
        slow.calls += 1
        return (1, 2, 3)

    slow.calls = 0
    results, errors = [], []

    def ask():
        c = store.connect(path)
        try:
            results.append(slow(c))
        except Exception as exc:  # noqa: BLE001
            errors.append(exc)
        finally:
            c.close()

    threads = [threading.Thread(target=ask) for _ in range(8)]
    for t in threads:
        t.start()
    assert started.wait(5)
    time.sleep(0.3)  # the other seven are waiting on the leader
    gate.set()
    for t in threads:
        t.join(10)
    assert not errors
    assert slow.calls == 1
    assert results == [(1, 2, 3)] * 8


def test_a_failing_computation_reaches_the_waiters_and_caches_nothing(conn, path):
    gate = threading.Event()
    state = {"n": 0}

    @memo.memoized
    def flaky(c):
        state["n"] += 1
        if state["n"] == 1:
            gate.wait(5)
            raise RuntimeError("boom")
        return "ok"

    outcomes = []

    def ask():
        c = store.connect(path)
        try:
            outcomes.append(flaky(c))
        except RuntimeError as exc:
            outcomes.append(str(exc))
        finally:
            c.close()

    threads = [threading.Thread(target=ask) for _ in range(4)]
    for t in threads:
        t.start()
    time.sleep(0.3)
    gate.set()
    for t in threads:
        t.join(10)
    assert outcomes == ["boom"] * 4
    assert flaky(conn) == "ok"  # the failure was not stored


# --- AC 3: never stale -----------------------------------------------------------------------


def test_new_event_from_another_connection(conn, other, scope_calls):
    add_event(conn, 1)
    assert len(metrics._messages(conn, None, None)) == 1
    add_event(other, 2)
    assert len(metrics._messages(conn, None, None)) == 2


def test_upsert_of_an_existing_row(conn, other):
    add_event(conn, 1)
    assert metrics._messages(conn, None, None)[0][4] == 10
    sql = (
        INSERT
        + " ON CONFLICT (machine_id, source, source_path, inode, byte_offset) DO UPDATE SET input_tokens = 77"
    )
    other.execute(sql, (1, _ts(1), "/work/app0", "msg1", None))
    other.commit()
    assert metrics._messages(conn, None, None)[0][4] == 77


def test_update_events(conn, other):
    add_event(conn, 1)
    assert metrics._messages(conn, None, None)[0][2] == "/work/app0"
    other.execute("UPDATE events SET project_path = '/work/moved'")
    other.commit()
    assert metrics._messages(conn, None, None)[0][2] == "/work/moved"


def test_retention_delete(conn, other):
    add_event(conn, 1)
    add_event(conn, 2)
    assert len(metrics._messages(conn, None, None)) == 2
    assert store.prune(other, 1) == 2
    assert metrics._messages(conn, None, None) == ()


def test_workspace_repo_add_and_remove(conn, other):
    add_event(conn, 1, "/work/app0")
    add_event(conn, 2, "/work/app1")
    ws = workspaces.add(conn, "acme", repos=["/work/app0"])
    assert [r[2] for r in metrics._messages(conn, None, None, "acme")] == ["/work/app0"]
    workspaces.edit(other, ws["id"], {"repos": ["/work/app0", "/work/app1"]})
    assert len(metrics._messages(conn, None, None, "acme")) == 2
    workspaces.edit(other, ws["id"], {"repos": []})
    assert metrics._messages(conn, None, None, "acme") == ()


def test_account_workspace_change(conn, other, scope_calls, tmp_path):
    folder = tmp_path / ".claude-work"
    (folder / "projects").mkdir(parents=True)
    acc = accounts.add(conn, str(folder))
    accounts.confirm(conn, str(acc["id"]), "Claude work")
    ws = workspaces.add(conn, "acme")
    add_event(conn, 1)
    metrics._messages(conn, None, None, None, "Claude work")
    metrics._messages(conn, None, None, None, "Claude work")
    assert len(scope_calls) == 1
    accounts.use(other, str(acc["id"]), [ws["name"]])
    metrics._messages(conn, None, None, None, "Claude work")
    assert len(scope_calls) == 2


def test_write_through_the_readers_own_connection(conn):
    add_event(conn, 1)
    assert len(metrics._messages(conn, None, None)) == 1
    add_event(conn, 2)  # the watcher is a separate connection, so it sees this commit
    assert len(metrics._messages(conn, None, None)) == 2


def test_uncommitted_writes_of_the_readers_connection_are_never_cached(conn, other):
    add_event(conn, 1)
    conn.execute(INSERT, (2, _ts(2), "/work/app0", "msg2", None))  # open transaction, not committed
    assert len(metrics._messages(conn, None, None)) == 2
    conn.rollback()
    assert len(metrics._messages(conn, None, None)) == 1
    assert len(metrics._messages(other, None, None)) == 1


def test_write_from_a_separate_process(conn, path, scope_calls):
    add_event(conn, 1)
    assert len(metrics._messages(conn, None, None)) == 1
    code = (
        "import sys; from pathlib import Path; from easy_personal.core import store;"
        "c = store.connect(Path(sys.argv[1]));"
        "c.execute('DELETE FROM events'); c.commit(); c.close()"
    )
    subprocess.run([sys.executable, "-c", code, str(path)], check=True)
    assert metrics._messages(conn, None, None) == ()


def test_stored_branches_follow_the_store(conn, other):
    add_event(conn, 1, "/work/app0", "feat-a")
    assert owners._stored_branches(conn) == {"/work/app0": {"feat-a"}}
    add_event(other, 2, "/work/app0", "feat-b")
    assert owners._stored_branches(conn) == {"/work/app0": {"feat-a", "feat-b"}}
    other.execute("DELETE FROM events")
    other.commit()
    assert owners._stored_branches(conn) == {}


# --- AC 4: callers cannot change what is shared -----------------------------------------------


def test_what_a_caller_gets_cannot_be_changed(conn):
    add_event(conn, 1, branch="feat-a")
    rows = metrics._messages(conn, None, None)
    with pytest.raises(AttributeError):
        rows.append(("x",))  # a tuple
    with pytest.raises(TypeError):
        rows[0][0] = "x"
    assert metrics._messages(conn, None, None) == rows

    branches = owners._stored_branches(conn)
    with pytest.raises(TypeError):
        branches["/other"] = frozenset()
    with pytest.raises(AttributeError):
        branches["/work/app0"].add("x")
    assert owners._stored_branches(conn) == {"/work/app0": {"feat-a"}}


# --- AC 5: bounded, older versions dropped ----------------------------------------------------


def test_cache_never_holds_more_than_its_bound(conn):
    for i in range(memo.MAX_ENTRIES * 3):
        costly(conn, i)
    assert memo.size() == memo.MAX_ENTRIES
    costly.calls.clear()
    costly(conn, memo.MAX_ENTRIES * 3 - 1)  # the newest is kept
    assert costly.calls == []


def test_entries_of_an_older_data_version_are_dropped(conn, other):
    for i in range(5):
        costly(conn, i)
    assert memo.size() == 5
    add_event(other, 1)
    costly(conn, 0)
    assert memo.size() == 1


def test_watchers_are_bounded_and_closed(tmp_path):
    conns = [store.connect(tmp_path / f"s{i}.sqlite") for i in range(memo.MAX_STORES + 4)]
    for c in conns:
        costly(c, 1)
    assert memo.watchers() == memo.MAX_STORES
    for c in conns:
        c.close()


def test_an_evicted_store_is_not_served_stale(tmp_path):
    first = store.connect(tmp_path / "s0.sqlite")
    writer = store.connect(tmp_path / "s0.sqlite")
    assert costly(first, 1) == 0
    for i in range(1, memo.MAX_STORES + 2):  # push s0's watcher out
        other = store.connect(tmp_path / f"s{i}.sqlite")
        costly(other, 1)
        other.close()
    add_event(writer, 1)
    assert costly(first, 1) == 1
    first.close()
    writer.close()


def test_a_store_recreated_at_the_same_path_is_not_served_stale(tmp_path):
    p = tmp_path / "store.sqlite"
    c = store.connect(p)
    add_event(c, 1)
    assert costly(c, 1) == 1
    c.close()
    for f in tmp_path.glob("store.sqlite*"):
        f.unlink()
    c = store.connect(p)
    assert costly(c, 1) == 0
    c.close()


# --- AC 6: watcher failure -> no cache, no error -----------------------------------------------


def test_watcher_cannot_be_opened(conn, monkeypatch, scope_calls):
    add_event(conn, 1)
    real = sqlite3.connect

    def failing(database, *args, **kwargs):
        raise sqlite3.OperationalError("unable to open")

    monkeypatch.setattr(sqlite3, "connect", failing)
    assert len(metrics._messages(conn, None, None)) == 1
    assert len(metrics._messages(conn, None, None)) == 1
    assert len(scope_calls) == 2  # computed each time, no cache
    assert owners._stored_branches(conn) == {"/work/app0": {""}}
    assert memo.size() == 0
    monkeypatch.setattr(sqlite3, "connect", real)
    metrics._messages(conn, None, None)
    metrics._messages(conn, None, None)
    assert len(scope_calls) == 3  # the watcher opens again once it can


def test_watcher_cannot_be_read(conn, scope_calls):
    add_event(conn, 1)
    metrics._messages(conn, None, None)

    class Broken:
        def execute(self, *a):
            raise sqlite3.DatabaseError("disk I/O error")

        def close(self):
            pass

    memo._watchers[next(iter(memo._watchers))] = Broken()
    assert len(metrics._messages(conn, None, None)) == 1
    assert len(metrics._messages(conn, None, None)) == 1
    assert len(scope_calls) >= 2


def test_in_memory_store_is_computed_without_cache():
    mem = sqlite3.connect(":memory:")
    mem.execute("CREATE TABLE events (x)")
    assert costly(mem, 1) == 0
    assert costly(mem, 1) == 0
    assert len(costly.calls) == 2
    assert memo.size() == 0
    mem.close()
