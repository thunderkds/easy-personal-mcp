"""NFR-005 throughput: a synthetic multi-account transcript set (HUN-63). Run with `-m perf`."""

import json
import time

import pytest

from easy_personal.core import accounts, config, metrics
from easy_personal.sources import claude_code

TARGET_BYTES = 210 * 1024 * 1024
FILE_BYTES = 5 * 1024 * 1024
MIN_MB_PER_S = 8
CACHED_RERUN_S = 2.0
PAD = "invented filler text, never real content " * 30  # ~1.2 KB per record, like a real turn


def _records(n: int, tag: str):
    base = {"version": "2.1.295", "cwd": "/work/demo", "sessionId": f"s-{tag}", "isSidechain": False}
    for i in range(n):
        stamp = f"2026-01-{1 + i % 28:02d}T{i % 24:02d}:{i % 60:02d}:00.000Z"
        yield {**base, "type": "user", "timestamp": stamp, "message": {"role": "user", "content": PAD}}
        yield {
            **base, "type": "assistant", "timestamp": stamp,
            "message": {"id": f"msg_{tag}_{i}", "model": "m-1", "content": [{"type": "text", "text": PAD}],
                        "usage": {"input_tokens": 3, "output_tokens": 5,
                                  "cache_creation_input_tokens": 0, "cache_read_input_tokens": 7}},
        }  # fmt: skip


def _generate(projects, account: str) -> int:
    projects.mkdir(parents=True)
    total, index = 0, 0
    while total < TARGET_BYTES // 2:
        lines = [json.dumps(r) for r in _records(1500, f"{account}{index}")]
        data = ("\n".join(lines) + "\n").encode()
        (projects / f"{index}.jsonl").write_bytes(data)
        total += len(data)
        index += 1
    return total


@pytest.mark.perf
def test_two_accounts_over_200mb_ingest_at_8mb_per_s_and_rerun_under_2s(conn):
    root = config.agent_root().resolve()
    total = _generate(root / ".claude" / "projects", "d") + _generate(root / ".claude-work" / "projects", "w")
    accounts.detect(conn)
    accounts.confirm(conn, str(root / ".claude-work"), "Claude work")
    assert total >= 200 * 1024 * 1024

    start = time.perf_counter()
    result = claude_code.ingest_accounts(conn)
    cold = time.perf_counter() - start
    rate = total / 1024 / 1024 / cold
    start = time.perf_counter()
    again = claude_code.ingest_accounts(conn)
    rerun = time.perf_counter() - start
    print(f"perf ingest: {total / 1024 / 1024:.0f} MB, {result['files']} files, cold {cold:.1f}s = "
          f"{rate:.1f} MB/s; unchanged re-run {rerun:.2f}s")  # fmt: skip

    assert result["new_events"] > 0 and again["new_events"] == 0
    assert metrics.usage_summary(conn, account="Claude work")["messages"] > 0
    assert rate >= MIN_MB_PER_S, rate
    assert rerun < CACHED_RERUN_S, rerun
