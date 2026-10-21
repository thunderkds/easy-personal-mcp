import pytest

from leak import harness


def test_fixture_plants_every_canary(canaries, tmp_path):
    import json

    canaries.render_fixture(tmp_path / "p")
    raw = b"".join(p.read_bytes() for p in (tmp_path / "p").rglob("*.jsonl"))
    for name, value in canaries.values.items():
        escaped = json.dumps(value)[1:-1].encode()  # how a JSON line holds it
        assert escaped in raw or value.encode() in raw, name


def test_canaries_are_unique_per_run():
    from leak.canaries import Canaries

    a, b = Canaries.new(), Canaries.new()
    assert len(set(a.values.values())) == len(a.values)
    assert not set(a.values.values()) & set(b.values.values())


def test_ingest_actually_ran(leak_run):
    # guards against a vacuous pass: the store must hold the planted session's metadata
    dump = leak_run.outputs["store .dump"]
    assert 'INSERT INTO "events"' in dump
    assert "/work/leak-demo" in dump


def test_every_mcp_tool_is_registered():
    assert harness.unregistered_tools(harness.build_app()) == set()


def test_every_cli_command_is_registered():
    assert harness.unregistered_commands() == set()


def test_no_canary_in_any_output_log_or_store(leak_run, canaries):
    leaks = {}
    for label, blob in leak_run.outputs.items():
        hits = canaries.find(blob)
        if hits:
            leaks[label] = hits
    assert not leaks, f"LEAKS: {leaks}"


def test_surfaces_were_scanned(leak_run):
    labels = " | ".join(leak_run.outputs)
    for expected in ("cli ping", "cli ingest", "cli usage", "mcp ping", "mcp usage_summary",
                     "mcp-stdio usage_summary", "log records", "store .dump",
                     "store file store.sqlite"):  # fmt: skip
        assert expected in labels, expected


# --- self-tests: the suite must FAIL when something leaks -----------------------------------


def _leaky_app(canaries, tool_returns):
    app = harness.build_app()

    @app.tool()
    def leaky() -> str:
        """Deliberately leaking test tool."""
        return tool_returns

    return app


def test_unregistered_new_tool_fails_meta_test(canaries):
    app = _leaky_app(canaries, "nothing")
    assert harness.unregistered_tools(app) == {"leaky"}


@pytest.mark.parametrize(
    "form",
    ["raw", "json", "unicode-escape", "url", "base64", "hex"],
)
def test_scan_detects_every_encoding(canaries, form):
    from leak.canaries import forms_of

    value = canaries.values["message_text"]
    leaked = f"prefix-{forms_of(value)[form][0]}-suffix"
    assert (("message_text", form)) in canaries.find(leaked)
    with pytest.raises(AssertionError, match="LEAK"):
        canaries.assert_clean("selftest", leaked)


def test_scan_detects_base64_at_any_alignment(canaries):
    import base64

    value = canaries.values["api_key"]
    for pad in range(3):
        blob = base64.b64encode(b"y" * pad + value.encode()).decode()
        assert ("api_key", "base64") in canaries.find(f"x:{blob}:y"), pad


def test_leaking_tool_fails_the_suite(canaries, monkeypatch):
    leak = canaries.values["tool_output"]
    app = _leaky_app(canaries, leak)
    monkeypatch.setitem(harness.MCP_CALLS, "leaky", [{}])
    assert harness.unregistered_tools(app) == set()
    out = harness.Collected()
    with harness.LogCapture() as logs:
        harness.collect_mcp(app, out, logs)
    leaks = {k: canaries.find(v) for k, v in out.outputs.items() if canaries.find(v)}
    assert any(label.startswith("mcp leaky") for label in leaks), leaks


def test_leak_via_log_and_stderr_is_detected(canaries):
    import logging

    with harness.LogCapture() as logs:
        logging.getLogger("selftest").warning("bad record %s", canaries.values["api_key"])
    assert canaries.find(logs.text())
    assert canaries.find(b"\x00" + canaries.values["email"].encode() + b"\xff")


def test_leak_in_store_is_detected(canaries, tmp_path):
    import sqlite3

    db = tmp_path / "store.sqlite"
    conn = sqlite3.connect(db)
    conn.execute("CREATE TABLE events (note TEXT)")
    conn.execute("INSERT INTO events VALUES (?)", (canaries.values["file_content"],))
    conn.commit()
    conn.close()
    blobs = harness.store_blobs(tmp_path)
    assert all(canaries.find(b) for b in blobs.values())
