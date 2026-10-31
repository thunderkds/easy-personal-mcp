"""Tests for the E3 OTLP receiver and analysis (HUN-168). Run: pytest experiments/live-trace/tests -k e3"""
import gzip
import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path

import pytest

LT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(LT / "e3"))
sys.path.insert(0, str(LT / "harness"))

import analyze  # noqa: E402
import receiver  # noqa: E402
import run_e3  # noqa: E402

SECRET = "sk-ant-FAKE-SECRET-123"
SENTENCE = "Please summarise the quarterly numbers for the Zebra account"
EMAIL = "someone@example.invalid"


def s(k, v):
    return {"key": k, "value": {"stringValue": v}}


def i(k, v):
    return {"key": k, "value": {"intValue": str(v)}}


def d(k, v):
    return {"key": k, "value": {"doubleValue": v}}


def nano(ms):
    return str(int(ms * 1_000_000))


T0 = 1_800_000_000_000  # an epoch ms


def logs_payload():
    res = {"attributes": [s("user.email", EMAIL), s("service.name", "claude-code"), s("host.arch", SECRET)]}
    recs = [
        {"timeUnixNano": nano(T0 + 5000), "body": {"stringValue": "claude_code.tool_result"},
         "attributes": [s("event.name", "tool_result"), s("session.id", "S1"), s("prompt.id", "P1"),
                        s("tool_use_id", "toolu_A"), s("tool_name", "Read"), s("success", "true"),
                        i("duration_ms", 12), s("event.timestamp", "2027-01-15T08:00:05.000Z"),
                        s("tool_parameters", SECRET), s("prompt", SENTENCE + SECRET),
                        s("user.email", EMAIL)]},
        {"timeUnixNano": nano(T0 + 6000), "body": {"stringValue": SENTENCE + " " + SECRET},
         "attributes": [s("event.name", "api_request"), s("session.id", "S1"), s("model", "claude-haiku-4-5"),
                        i("input_tokens", 10), i("output_tokens", 20), i("cache_read_tokens", 300),
                        i("cache_creation_tokens", 40), d("cost_usd", 0.002), s("query_source", "repl_main_thread"),
                        s("request_id", "req_1"), s("response", SENTENCE)]},
        # body is prompt text and no event.name attribute: name must be dropped, not stored
        {"timeUnixNano": nano(T0 + 7000), "body": {"stringValue": SENTENCE},
         "attributes": [s("session.id", "S1")]},
    ]
    return {"resourceLogs": [{"resource": res, "scopeLogs": [{"logRecords": recs}]}]}


def metrics_payload():
    def dp(val, **attrs):
        return {"attributes": [s(k, v) for k, v in attrs.items()], "asDouble": val,
                "startTimeUnixNano": nano(T0), "timeUnixNano": nano(T0 + 1000)}
    return {"resourceMetrics": [{"resource": {"attributes": [s("user.email", EMAIL)]}, "scopeMetrics": [{"metrics": [
        {"name": "claude_code.cost.usage", "unit": "USD", "sum": {"aggregationTemporality": 1, "isMonotonic": True,
         "dataPoints": [dp(0.002, model="claude-haiku-4-5", note=SENTENCE), dp(0.003, model="claude-haiku-4-5")]}},
        {"name": "claude_code.token.usage", "unit": "tokens", "sum": {"aggregationTemporality": 1,
         "dataPoints": [dp(10, type="input"), dp(20, type="output"), dp(300, type="cacheRead"),
                        dp(40, type="cacheCreation")]}},
    ]}]}]}


def post(rx, path, payload, gz=False, raw=None):
    body = raw if raw is not None else json.dumps(payload).encode()
    headers = {"Content-Type": "application/json"}
    if gz:
        body, headers["Content-Encoding"] = gzip.compress(body), "gzip"
    req = urllib.request.Request(f"http://127.0.0.1:{rx.port}{path}", data=body, headers=headers, method="POST")
    with urllib.request.urlopen(req, timeout=5) as r:
        return r.status


@pytest.fixture
def rx():
    with receiver.OtlpReceiver(port=0) as r:
        yield r


def test_e3_logs_keep_ids_tokens_cost_and_all_attribute_names(rx):
    assert post(rx, "/v1/logs", logs_payload()) == 200
    tool = next(e for e in rx.events if e["event_name"] == "tool_result")
    assert tool["attrs"]["session.id"] == "S1" and tool["attrs"]["prompt.id"] == "P1"
    assert tool["attrs"]["tool_use_id"] == "toolu_A" and tool["attrs"]["tool_name"] == "Read"
    assert tool["attrs"]["duration_ms"] == 12
    assert {"prompt", "tool_parameters", "user.email", "tool_use_id"} <= set(tool["attr_names"])
    api = next(e for e in rx.events if e["event_name"] == "api_request")
    a = api["attrs"]
    assert (a["input_tokens"], a["output_tokens"], a["cache_read_tokens"], a["cache_creation_tokens"]) == (10, 20, 300, 40)
    assert a["cost_usd"] == pytest.approx(0.002) and a["model"] == "claude-haiku-4-5"
    assert tool["record_ms"] == pytest.approx(T0 + 5000) and tool["recv_ms"] > 1_700_000_000_000
    assert "user.email" in tool["resource_attr_names"] and "service.name" in tool["resource_attr_names"]


def test_e3_metrics_keep_name_attribute_names_and_values(rx):
    assert post(rx, "/v1/metrics", metrics_payload(), gz=True) == 200
    names = {m["metric"] for m in rx.metrics}
    assert names == {"claude_code.cost.usage", "claude_code.token.usage"}
    cost = [m for m in rx.metrics if m["metric"] == "claude_code.cost.usage"]
    assert sorted(m["value"] for m in cost) == [0.002, 0.003] and cost[0]["unit"] == "USD"
    assert "note" in cost[0]["attr_names"] and "note" not in cost[0]["attrs"]  # name kept, value dropped
    assert cost[0]["temporality"] == 1


def test_e3_allow_list_drops_prompt_like_text_secret_email_and_body(rx):
    post(rx, "/v1/logs", logs_payload())
    post(rx, "/v1/metrics", metrics_payload())
    dumped = json.dumps(rx.snapshot())
    assert SECRET not in dumped and SENTENCE not in dumped and EMAIL not in dumped and "Zebra" not in dumped
    unnamed = [e for e in rx.events if e["event_name"] is None]
    assert len(unnamed) == 1  # the prompt-text body never became an event name


def test_e3_allow_listed_key_with_sentence_value_is_dropped(rx):
    payload = {"resourceLogs": [{"scopeLogs": [{"logRecords": [{"attributes": [
        s("event.name", "api_request"), s("model", SENTENCE), s("session.id", "S9"), s("tool_name", SECRET + " x y")]}]}]}]}
    post(rx, "/v1/logs", payload)
    ev = rx.events[0]
    assert "model" in ev["attr_names"] and "model" not in ev["attrs"] and "tool_name" not in ev["attrs"]
    assert ev["attrs"]["session.id"] == "S9"


def test_e3_bad_requests(rx):
    for path, kw in (("/v1/logs", {"raw": b"not json " + SECRET.encode()}), ("/nope", {"payload": {}})):
        with pytest.raises(urllib.error.HTTPError) as exc:
            post(rx, path, kw.get("payload"), raw=kw.get("raw"))
        assert exc.value.code in (400, 404)
    assert rx.events == [] and rx.posts == []


def test_e3_binds_loopback_only():
    with pytest.raises(ValueError):
        receiver.OtlpReceiver(host="0.0.0.0", port=0)


def test_e3_traces_keep_span_names_and_attribute_names_only(rx):
    payload = {"resourceSpans": [{"scopeSpans": [{"spans": [
        {"name": "claude_code.interaction", "traceId": "a" * 32, "spanId": "b" * 16, "kind": 1,
         "startTimeUnixNano": nano(T0), "endTimeUnixNano": nano(T0 + 50),
         "attributes": [s("session.id", "S1"), s("user_prompt", SENTENCE), s("tool_name", "Read")]},
        {"name": SENTENCE, "traceId": "a" * 32, "spanId": "c" * 16, "parentSpanId": "b" * 16, "attributes": []},
    ]}]}]}
    assert post(rx, "/v1/traces", payload) == 200
    assert [x["span_name"] for x in rx.spans] == ["claude_code.interaction", None]
    assert rx.spans[1]["parent_span_id"] == "b" * 16
    assert "user_prompt" in rx.spans[0]["attr_names"] and SENTENCE not in json.dumps(rx.snapshot())


def test_e3_wait_quiet_returns_after_last_post(rx):
    post(rx, "/v1/logs", logs_payload())
    waited = rx.wait_quiet(quiet_s=0.5, max_s=5)
    assert 0.4 <= waited < 3


def _ev(name, **attrs):
    return {"event_name": name, "recv_ms": attrs.pop("recv_ms", 0), "record_ms": attrs.pop("record_ms", None),
            "attrs": attrs, "attr_names": sorted(attrs)}


def test_e3_latency_matches_by_tool_use_id_and_reports_median_p95():
    ref = {f"t{k}": ("Read", analyze_ts(T0)) for k in range(1, 6)}
    events = [_ev("tool_result", tool_use_id=f"t{k}", recv_ms=T0 + 1000 * k, **{"event.timestamp": analyze_ts(T0 + 1000 * k - 200)})
              for k in range(1, 6)] + [_ev("tool_result", tool_use_id="orphan", recv_ms=T0 + 1)]
    out = analyze.tool_result_latency(events, ref)
    assert out["latency"]["n"] == 5 and out["unmatched"] == 1
    assert out["latency"]["median_ms"] == pytest.approx(3000, abs=1) and out["latency"]["p95_ms"] == pytest.approx(5000, abs=1)
    assert out["export_lag"]["median_ms"] == pytest.approx(200, abs=1)


def analyze_ts(ms):
    from datetime import UTC, datetime
    return datetime.fromtimestamp(ms / 1000, tz=UTC).isoformat(timespec="milliseconds").replace("+00:00", "Z")


def test_e3_duplicate_tool_use_ids_are_counted():
    evs = [_ev("tool_result", tool_use_id="a"), _ev("tool_result", tool_use_id="a"), _ev("tool_result", tool_use_id="b")]
    assert analyze.duplicate_tool_ids(evs) == 1


def test_e3_token_and_cost_comparison_with_transcript(tmp_path):
    sid = "SESS"
    proj = tmp_path / "projects" / "p"
    (proj / sid / "subagents").mkdir(parents=True)

    def line(mid, usage):
        return json.dumps({"requestId": "r" + mid, "message": {"id": mid, "usage": usage,
                           "content": [{"type": "text", "text": SENTENCE + SECRET}]}}) + "\n"
    u1 = {"input_tokens": 10, "output_tokens": 20, "cache_read_input_tokens": 300, "cache_creation_input_tokens": 40}
    # same message written twice (one record per content block): counted once; later partial output kept as max
    (proj / f"{sid}.jsonl").write_text(line("m1", u1) + line("m1", {**u1, "output_tokens": 5}) + "{partial " + SECRET + "\n")
    (proj / sid / "subagents" / "agent-x.jsonl").write_text(line("m2", {"input_tokens": 1, "output_tokens": 2}))
    tr = analyze.transcript_usage(sid, tmp_path)
    assert tr["messages"] == 2
    assert tr["total"] == {"input": 11, "output": 22, "cacheRead": 300, "cacheCreation": 40}
    assert tr["sub"]["input"] == 1 and tr["main"]["input"] == 10
    events = [_ev("api_request", input_tokens=10, output_tokens=20, cache_read_tokens=300, cache_creation_tokens=40,
                  cost_usd=0.002, request_id="q1", query_source="repl_main_thread"),
              _ev("api_request", input_tokens=1, output_tokens=2, cost_usd=0.001, request_id="q2", query_source="subagent"),
              _ev("api_request", input_tokens=1, output_tokens=2, cost_usd=0.001, request_id="q2", query_source="subagent")]
    api = analyze.otel_api_totals(events)
    assert api["total"]["requests"] == 2 and api["total"]["duplicate_request_ids"] == 1
    assert api["total"]["cost_usd"] == pytest.approx(0.003)
    cmp = analyze.compare_tokens(api["total"], tr["total"])
    assert cmp["all_types"]["pct_diff"] == 0 and cmp["input"]["pct_diff"] == 0
    assert analyze.pct_diff(101, 100) == 1.0 and analyze.pct_diff(1, 0) is None
    mt = analyze.metric_totals([
        {"metric": "claude_code.cost.usage", "value": 0.002, "attrs": {}, "temporality": 1},
        {"metric": "claude_code.cost.usage", "value": 0.001, "attrs": {}, "temporality": 1},
        {"metric": "claude_code.token.usage", "value": 7, "attrs": {"type": "input"}, "temporality": 1}])
    assert mt["cost_usd"] == pytest.approx(0.003) and mt["tokens"]["input"] == 7
    assert SECRET not in json.dumps(tr) and SENTENCE not in json.dumps(tr)


def test_e3_attribute_inventory_counts_names_per_event():
    rows = [_ev("tool_result", tool_use_id="a", tool_name="Read"), _ev("tool_result", tool_use_id="b"),
            _ev("api_request", model="m")]
    inv = analyze.attribute_inventory(rows, "event_name")
    assert inv["tool_result"]["records"] == 2 and inv["tool_result"]["attrs"] == {"tool_use_id": 2, "tool_name": 1}


def test_e3_runner_refuses_content_logging_vars_and_sets_none():
    for cfg, extra in run_e3.CONFIGS.items():
        env = {**run_e3.BASE_ENV, **extra}
        assert not [k for k in env if k.startswith("OTEL_LOG_")], cfg
        assert env["OTEL_EXPORTER_OTLP_PROTOCOL"] == "http/json" and "127.0.0.1" in env["OTEL_EXPORTER_OTLP_ENDPOINT"]
    assert run_e3.CONFIGS["lowered"] == {"OTEL_LOGS_EXPORT_INTERVAL": "1000", "OTEL_METRIC_EXPORT_INTERVAL": "5000"}
    assert run_e3.CONFIGS["default"] == {}


def test_e3_run_once_with_fake_claude_exports_to_receiver_and_holds_lock(tmp_path):
    """A fake `claude` posts one tool_result log to the receiver named in the child env."""
    fake = tmp_path / "claude"
    fake.write_text(f"""#!/usr/bin/env python3
import json, os, urllib.request
assert os.environ["CLAUDE_CODE_ENABLE_TELEMETRY"] == "1" and not any(k.startswith("OTEL_LOG_") for k in os.environ)
ep = os.environ["OTEL_EXPORTER_OTLP_ENDPOINT"]
body = {{"resourceLogs": [{{"scopeLogs": [{{"logRecords": [{{"attributes": [
  {{"key": "event.name", "value": {{"stringValue": "tool_result"}}}},
  {{"key": "session.id", "value": {{"stringValue": "FAKESESS"}}}},
  {{"key": "tool_use_id", "value": {{"stringValue": "toolu_1"}}}},
  {{"key": "prompt", "value": {{"stringValue": "{SENTENCE} {SECRET}"}}}}]}}]}}]}}]}}
urllib.request.urlopen(urllib.request.Request(ep + "/v1/logs", data=json.dumps(body).encode(), method="POST"), timeout=5)
print(json.dumps({{"type": "system", "subtype": "init", "session_id": "FAKESESS"}}))
print(json.dumps({{"type": "result", "subtype": "success", "is_error": False, "total_cost_usd": 0.01}}))
""")
    fake.chmod(0o755)
    lock = tmp_path / "lock"
    out = run_e3.run_once("lowered", grace_quiet=0.3, grace_max=3, lock_path=str(lock), config_dir=tmp_path / "cfg",
                          runs_dir=tmp_path / "runs", port=0 + _free_port(), claude_bin=str(fake))
    assert out["session"]["session_id"] == "FAKESESS" and out["session"]["exit_code"] == 0
    assert [e["event_name"] for e in out["receiver"]["events"]] == ["tool_result"]
    stored = Path(out["out_file"]).read_text() + "".join(p.read_text() for p in (tmp_path / "runs" / "sessions").glob("*.json"))
    assert SECRET not in stored and SENTENCE not in stored
    assert lock.exists()


def _free_port():
    import socket
    with socket.socket() as sk:
        sk.bind(("127.0.0.1", 0))
        return sk.getsockname()[1]


def test_e3_planted_secret_absent_from_stored_files():
    """Grep check: no stored artefact of this task holds the planted secret or the fake prompt sentence."""
    own = Path(__file__).resolve()
    roots = [LT / "e3"]
    for root in roots:
        for p in root.rglob("*"):
            if p.is_file() and p.suffix in {".json", ".md", ".py", ".txt", ".log", ".jsonl"}:
                text = p.read_text(errors="replace")
                assert SECRET not in text and SENTENCE not in text and EMAIL not in text, p
    assert own.exists()
    for name in ("receiver.py", "analyze.py", "run_e3.py"):
        assert SECRET not in (LT / "e3" / name).read_text()
