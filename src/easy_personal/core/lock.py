"""Single-writer lock for ingest: an flock on a file in the data dir.

The OS drops the lock when the holder exits or crashes, so a leftover lock file never blocks.
"""

import fcntl
import time
from collections.abc import Iterator
from contextlib import contextmanager
from pathlib import Path

POLL_SECONDS = 0.05
DEFAULT_TIMEOUT = 300.0


class LockBusy(Exception):
    """Another ingest still holds the lock after the wait timeout."""


@contextmanager
def ingest_lock(path: Path, timeout: float = DEFAULT_TIMEOUT) -> Iterator[None]:
    """Hold an exclusive lock on `path`; wait up to `timeout` seconds, else raise LockBusy."""
    with open(path, "a") as handle:
        deadline = time.monotonic() + timeout
        while True:
            try:
                fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    raise LockBusy(str(path)) from None
                time.sleep(POLL_SECONDS)
        try:
            yield
        finally:
            fcntl.flock(handle, fcntl.LOCK_UN)
