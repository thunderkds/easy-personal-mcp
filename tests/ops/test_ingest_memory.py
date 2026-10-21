import json
import tracemalloc

from easy_personal.core import store
from easy_personal.sources import claude_code

PEAK_CAP_BYTES = 8 * 1024 * 1024
LINES = 40_000  # about 12 MB of transcript


def test_ingest_of_a_large_transcript_stays_under_a_memory_cap(tmp_path):
    root = tmp_path / "projects" / "p"
    root.mkdir(parents=True)
    line = json.dumps(
        {
            "type": "assistant",
            "sessionId": "s1",
            "timestamp": "2026-01-01T00:00:00Z",
            "cwd": str(tmp_path),
            "message": {
                "id": "m",
                "model": "claude-x",
                "content": [{"type": "text", "text": "x" * 200}],
                "usage": {"input_tokens": 1, "output_tokens": 1},
            },
        }
    )
    with open(root / "big.jsonl", "w") as handle:
        handle.writelines(line.replace('"id": "m"', f'"id": "m{i}"') + "\n" for i in range(LINES))
    assert (root / "big.jsonl").stat().st_size > 10 * 1024 * 1024
    with store.opened(tmp_path / "store.sqlite") as conn:
        tracemalloc.start()
        result = claude_code.ingest(conn, tmp_path / "projects")
        _, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
    assert result["new_events"] > 0
    assert peak < PEAK_CAP_BYTES, peak
