"""Share the result of a costly store computation between calls while the store has not changed (NFR-013).

A result is keyed by the function, every argument and the store's `PRAGMA data_version`. SQLite bumps
that number on every commit made by another connection (any process), so one long-lived watcher
connection per store sees every write and a result is never served after a change. The watcher is used
for nothing else. Several threads asking for one key at once share one computation (single flight).

Returned values are shared, so a decorated function must return immutable values (tuples, frozensets,
read-only mappings). The cache is small and fixed: `MAX_ENTRIES` results, `MAX_STORES` watchers. When
the watcher cannot be opened or read, the store is in memory or the connection has an open transaction
(its own uncommitted writes are invisible to the watcher), the function runs without any caching.
"""

import contextlib
import functools
import inspect
import os
import sqlite3
import threading

MAX_ENTRIES = 32
MAX_STORES = 8

_lock = threading.Lock()
_watchers: dict[tuple, sqlite3.Connection] = {}  # store id -> its watcher connection, oldest first
_versions: dict[tuple, int] = {}  # store id -> last data version seen
_entries: dict[tuple, object] = {}  # (store id, version, function, args) -> result, oldest first
_flights: dict[tuple, "_Flight"] = {}


class _Flight:
    """One computation in progress; threads that ask for the same key wait on `done`."""

    __slots__ = ("done", "error", "value")

    def __init__(self) -> None:
        self.done = threading.Event()
        self.value = None
        self.error: BaseException | None = None


def _store_id(conn: sqlite3.Connection) -> tuple | None:
    """(real path, device, inode) of the store file, or None for a connection that must not be cached."""
    if conn.in_transaction:
        return None
    file = conn.execute("PRAGMA database_list").fetchone()[2]
    if not file:
        return None
    stat = os.stat(file)
    return os.path.realpath(file), stat.st_dev, stat.st_ino


def _drop(sid: tuple) -> None:
    """Forget a store: close its watcher and its entries. Caller holds `_lock`."""
    watcher = _watchers.pop(sid, None)
    _versions.pop(sid, None)
    for key in [k for k in _entries if k[0] == sid]:
        del _entries[key]
    if watcher is not None:
        with contextlib.suppress(Exception):  # a broken watcher is being thrown away anyway
            watcher.close()


def _version(sid: tuple) -> int | None:
    """The store's data version, or None when it cannot be read. Caller holds `_lock`."""
    try:
        watcher = _watchers.get(sid)
        if watcher is None:
            while len(_watchers) >= MAX_STORES:
                _drop(next(iter(_watchers)))
            watcher = _watchers[sid] = sqlite3.connect(sid[0], check_same_thread=False)
        version = watcher.execute("PRAGMA data_version").fetchone()[0]
    except Exception:  # noqa: BLE001 - never an error: the caller computes without caching
        _drop(sid)
        return None
    if _versions.get(sid) != version:
        for key in [k for k in _entries if k[0] == sid]:  # entries of an older version
            del _entries[key]
        _versions[sid] = version
    return version


def memoized(fn):
    """Cache `fn(conn, *args)` per store, arguments and data version, with single flight."""
    signature, name = inspect.signature(fn), f"{fn.__module__}.{fn.__qualname__}"

    @functools.wraps(fn)
    def wrapper(conn, *args, **kwargs):
        try:
            bound = signature.bind(conn, *args, **kwargs)
            bound.apply_defaults()
            arguments = tuple(bound.arguments.values())[1:]
            hash(arguments)
            sid = _store_id(conn)
        except Exception:  # noqa: BLE001 - not cacheable; `fn` raises its own error if the call is wrong
            return fn(conn, *args, **kwargs)
        if sid is None:
            return fn(conn, *args, **kwargs)
        with _lock:
            version = _version(sid)
            if version is None:
                key = None
            else:
                key = (sid, version, name, arguments)
                if key in _entries:
                    return _entries[key]
                flight = _flights.get(key)
                leader = flight is None
                if leader:
                    flight = _flights[key] = _Flight()
        if key is None:
            return fn(conn, *args, **kwargs)
        if not leader:
            flight.done.wait()
            if flight.error is not None:
                raise flight.error
            return flight.value
        try:
            flight.value = fn(conn, *args, **kwargs)
        except BaseException as exc:
            flight.error = exc
            raise
        else:
            with _lock:
                if _versions.get(sid) == version:  # nothing newer was seen while computing
                    _entries[key] = flight.value
                    while len(_entries) > MAX_ENTRIES:
                        del _entries[next(iter(_entries))]
            return flight.value
        finally:
            with _lock:
                _flights.pop(key, None)
            flight.done.set()

    return wrapper


def reset() -> None:
    """Close every watcher and forget every entry (tests)."""
    with _lock:
        for sid in list(_watchers):
            _drop(sid)
        _entries.clear()
        _versions.clear()


def size() -> int:
    return len(_entries)


def watchers() -> int:
    return len(_watchers)
