"""Stdlib OTLP/HTTP JSON receiver for the live-trace spike (HUN-168, E3).

Takes `POST /v1/logs`, `/v1/metrics` and `/v1/traces` (protocol `http/json`, optional gzip) on
127.0.0.1 and keeps METADATA ONLY (PRD FR-002 / NFR-002):

* per event: event name, receive time, record time, ALL attribute names, and the VALUES of an
  allow-listed set of attributes (ids, names, token counts, cost, model, ...);
* per metric data point: metric name, unit, temporality, attribute names, allow-listed attribute
  values, numeric value, time;
* per span: span name, ids, kind, start/end, attribute names, allow-listed values.

Everything else is dropped before anything is stored: record bodies (unless they are a plain
`claude_code.<name>` event name), non-allow-listed attribute values (prompt text, emails, paths,
tool input), resource attribute values. Raw request bodies are never written or logged.

Interface (HUN-170 calls this):
    from receiver import OtlpReceiver
    with OtlpReceiver(port=4318) as rx:          # port=0 picks a free port; rx.port tells which
        ...run the Claude Code session with the OTEL env...
        rx.wait_quiet(quiet_s=8, max_s=30)       # let the last export arrive
        rx.events, rx.metrics, rx.spans, rx.posts   # lists of plain dicts (sanitized)
    rx.dump(path)                                 # write the sanitized records as JSON

CLI: python receiver.py [--port 4318] [--out FILE]   (runs until Ctrl-C, writes FILE on exit)
"""
import argparse
import gzip
import json
import re
import sys
import threading
import time
from datetime import UTC, datetime
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

DEFAULT_PORT = 4318
MAX_BODY_BYTES = 8 * 1024 * 1024

# Attribute VALUES that may be stored. Anything not on these lists keeps its NAME only.
STR_KEYS = frozenset({
    "event.name", "event.timestamp", "session.id", "prompt.id", "tool_use_id", "tool_name",
    "model", "query_source", "type", "agent_id", "parent_agent_id", "agent.name", "skill.name",
    "request_id", "client_request_id", "success", "error_type", "decision_type",
    "decision_source", "speed", "effort", "app.version", "app.entrypoint", "terminal.type",
    "stop_reason", "subagent_type", "mcp_server_scope", "language", "interaction.sequence",
    "span.type", "llm_request.context",
})
NUM_KEYS = frozenset({
    "event.sequence", "input_tokens", "output_tokens", "cache_read_tokens", "cache_creation_tokens",
    "cache_creation_5m_tokens", "cache_creation_1h_tokens", "cost_usd", "cost_usd_micros",
    "duration_ms", "tool_input_size_bytes", "tool_result_size_bytes", "ttft_ms", "attempt",
    "status_code", "input_tokens_total", "output_tokens_total", "num_tools", "tool_count",
})
# A stored string value must look like an id or a name: no spaces, short. Sentences never match.
SAFE_STR = re.compile(r"^[A-Za-z0-9_.:+\-]{1,128}$")
EVENT_NAME = re.compile(r"^(claude_code\.)?[a-z][a-z0-9_]{0,63}$")
SPAN_NAME = re.compile(r"^[A-Za-z][A-Za-z0-9_.:\-]{0,63}$")
METRIC_NAME = re.compile(r"^[A-Za-z][A-Za-z0-9_.:\-]{0,95}$")
ATTR_NAME = re.compile(r"^[A-Za-z_][A-Za-z0-9_.:\-]{0,95}$")


def _any_value(v):
    """Decode an OTLP JSON AnyValue to a Python scalar, or None for arrays/maps/bytes."""
    if not isinstance(v, dict):
        return None
    if "stringValue" in v:
        return v["stringValue"] if isinstance(v["stringValue"], str) else None
    if "boolValue" in v:
        return v["boolValue"] if isinstance(v["boolValue"], bool) else None
    if "intValue" in v:  # int64 is a JSON string
        try:
            return int(v["intValue"])
        except (TypeError, ValueError):
            return None
    if "doubleValue" in v:
        return v["doubleValue"] if isinstance(v["doubleValue"], (int, float)) else None
    return None


def _num(x):
    if isinstance(x, bool):
        return None
    if isinstance(x, (int, float)):
        return x
    if isinstance(x, str):
        try:
            return float(x) if "." in x or "e" in x.lower() else int(x)
        except ValueError:
            return None
    return None


def sanitize_attrs(attrs) -> tuple[list[str], dict]:
    """(sorted attribute NAMES, {allow-listed name: value}). Other values are dropped here."""
    names: set[str] = set()
    values: dict = {}
    if not isinstance(attrs, list):
        return [], {}
    for a in attrs:
        if not isinstance(a, dict):
            continue
        key = a.get("key")
        if not isinstance(key, str) or not ATTR_NAME.match(key):
            continue
        names.add(key)
        val = _any_value(a.get("value"))
        if key in STR_KEYS:
            if isinstance(val, bool):
                values[key] = val
            elif isinstance(val, str) and SAFE_STR.match(val):
                values[key] = val
            elif isinstance(val, (int, float)):
                values[key] = val
        elif key in NUM_KEYS:
            n = _num(val)
            if n is not None:
                values[key] = n
    return sorted(names), values


def _nano_to_ms(x):
    n = _num(x)
    return None if n is None or n == 0 else int(n) / 1e6


def _event_name(rec_body, values) -> str | None:
    """Event name from the allow-listed event.name attribute, else a plain-name body, else None."""
    name = values.get("event.name")
    if isinstance(name, str):
        return name
    body = _any_value(rec_body) if isinstance(rec_body, dict) else None
    if isinstance(body, str) and EVENT_NAME.match(body):
        return body
    return None


def parse_logs(payload, recv_ms: float) -> list[dict]:
    out = []
    for rl in (payload.get("resourceLogs") or []) if isinstance(payload, dict) else []:
        res_names, _ = sanitize_attrs((rl.get("resource") or {}).get("attributes")) if isinstance(rl, dict) else ([], {})
        for sl in (rl.get("scopeLogs") or []) if isinstance(rl, dict) else []:
            for rec in (sl.get("logRecords") or []) if isinstance(sl, dict) else []:
                if not isinstance(rec, dict):
                    continue
                names, values = sanitize_attrs(rec.get("attributes"))
                out.append({
                    "kind": "log",
                    "event_name": _event_name(rec.get("body"), values),
                    "recv_ms": recv_ms,
                    "record_ms": _nano_to_ms(rec.get("timeUnixNano")),
                    "observed_ms": _nano_to_ms(rec.get("observedTimeUnixNano")),
                    "attr_names": names,
                    "resource_attr_names": res_names,
                    "attrs": values,
                })
    return out


def parse_metrics(payload, recv_ms: float) -> list[dict]:
    out = []
    for rm in (payload.get("resourceMetrics") or []) if isinstance(payload, dict) else []:
        for sm in (rm.get("scopeMetrics") or []) if isinstance(rm, dict) else []:
            for m in (sm.get("metrics") or []) if isinstance(sm, dict) else []:
                if not isinstance(m, dict):
                    continue
                name = m.get("name")
                if not isinstance(name, str) or not METRIC_NAME.match(name):
                    continue
                unit = m.get("unit") if isinstance(m.get("unit"), str) and len(m.get("unit")) < 16 else None
                body = m.get("sum") or m.get("gauge") or m.get("histogram") or {}
                temporality = body.get("aggregationTemporality") if isinstance(body, dict) else None
                for dp in (body.get("dataPoints") or []) if isinstance(body, dict) else []:
                    if not isinstance(dp, dict):
                        continue
                    names, values = sanitize_attrs(dp.get("attributes"))
                    val = _num(dp.get("asDouble", dp.get("asInt", dp.get("sum"))))
                    out.append({
                        "kind": "metric", "metric": name, "unit": unit,
                        "temporality": temporality, "recv_ms": recv_ms,
                        "start_ms": _nano_to_ms(dp.get("startTimeUnixNano")),
                        "time_ms": _nano_to_ms(dp.get("timeUnixNano")),
                        "value": val, "attr_names": names, "attrs": values,
                    })
    return out


def parse_traces(payload, recv_ms: float) -> list[dict]:
    out = []
    for rs in (payload.get("resourceSpans") or []) if isinstance(payload, dict) else []:
        for ss in (rs.get("scopeSpans") or []) if isinstance(rs, dict) else []:
            for sp in (ss.get("spans") or []) if isinstance(ss, dict) else []:
                if not isinstance(sp, dict):
                    continue
                name = sp.get("name")
                names, values = sanitize_attrs(sp.get("attributes"))
                ids = {k: sp.get(k) for k in ("traceId", "spanId", "parentSpanId")}
                out.append({
                    "kind": "span",
                    "span_name": name if isinstance(name, str) and SPAN_NAME.match(name) else None,
                    "trace_id": ids["traceId"] if isinstance(ids["traceId"], str) and re.fullmatch(r"[0-9a-f]{32}", ids["traceId"]) else None,
                    "span_id": ids["spanId"] if isinstance(ids["spanId"], str) and re.fullmatch(r"[0-9a-f]{16}", ids["spanId"]) else None,
                    "parent_span_id": ids["parentSpanId"] if isinstance(ids["parentSpanId"], str) and re.fullmatch(r"[0-9a-f]{16}", ids["parentSpanId"]) else None,
                    "span_kind": sp.get("kind") if isinstance(sp.get("kind"), int) else None,
                    "start_ms": _nano_to_ms(sp.get("startTimeUnixNano")),
                    "end_ms": _nano_to_ms(sp.get("endTimeUnixNano")),
                    "recv_ms": recv_ms, "attr_names": names, "attrs": values,
                })
    return out


class _Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "live-trace-e3"

    def log_message(self, *args):  # silence: no request line is ever printed
        pass

    def _reply(self, code: int, body: bytes = b"{}"):
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):  # noqa: N802
        recv_ms = time.time() * 1000.0
        rx: OtlpReceiver = self.server.rx  # type: ignore[attr-defined]
        path = self.path.split("?", 1)[0]
        parsers = {"/v1/logs": ("events", parse_logs), "/v1/metrics": ("metrics", parse_metrics),
                   "/v1/traces": ("spans", parse_traces)}
        if path not in parsers:
            self.rfile.read(min(int(self.headers.get("Content-Length") or 0), MAX_BODY_BYTES))
            return self._reply(404)
        try:
            length = int(self.headers.get("Content-Length") or 0)
            if length > MAX_BODY_BYTES:
                return self._reply(413)
            raw = self.rfile.read(length)
            if (self.headers.get("Content-Encoding") or "").lower() == "gzip":
                raw = gzip.decompress(raw)
            payload = json.loads(raw)
        except (ValueError, OSError, EOFError):
            return self._reply(400)
        attr, parse = parsers[path]
        rows = parse(payload, recv_ms)
        del raw, payload  # the raw body is dropped here
        rx._add(attr, path, recv_ms, rows)
        self._reply(200)


class OtlpReceiver:
    """Threaded receiver; all stored rows are sanitized dicts (see module doc)."""

    def __init__(self, host: str = "127.0.0.1", port: int = DEFAULT_PORT):
        if host != "127.0.0.1":
            raise ValueError("the receiver binds 127.0.0.1 only")
        self._srv = ThreadingHTTPServer((host, port), _Handler)
        self._srv.daemon_threads = True
        self._srv.rx = self  # type: ignore[attr-defined]
        self.host, self.port = host, self._srv.server_address[1]
        self.events: list[dict] = []
        self.metrics: list[dict] = []
        self.spans: list[dict] = []
        self.posts: list[dict] = []
        self._lock = threading.Lock()
        self._last_post = time.monotonic()
        self._thread = None

    def _add(self, attr: str, path: str, recv_ms: float, rows: list[dict]):
        with self._lock:
            getattr(self, attr).extend(rows)
            self.posts.append({"path": path, "recv_ms": recv_ms, "records": len(rows)})
            self._last_post = time.monotonic()

    def start(self):
        self._thread = threading.Thread(target=self._srv.serve_forever, daemon=True)
        self._thread.start()
        return self

    def stop(self):
        self._srv.shutdown()
        self._srv.server_close()
        if self._thread:
            self._thread.join(timeout=5)

    def __enter__(self):
        return self.start()

    def __exit__(self, *exc):
        self.stop()

    def wait_quiet(self, quiet_s: float = 8.0, max_s: float = 30.0) -> float:
        """Block until no POST arrived for quiet_s, or max_s passed. Returns seconds waited."""
        t0 = time.monotonic()
        while time.monotonic() - t0 < max_s:
            with self._lock:
                idle = time.monotonic() - self._last_post
            if idle >= quiet_s:
                break
            time.sleep(0.25)
        return time.monotonic() - t0

    def snapshot(self) -> dict:
        with self._lock:
            return {"events": list(self.events), "metrics": list(self.metrics),
                    "spans": list(self.spans), "posts": list(self.posts)}

    def dump(self, path) -> Path:
        p = Path(path)
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(self.snapshot(), indent=1) + "\n")
        return p


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--port", type=int, default=DEFAULT_PORT)
    ap.add_argument("--out", default=None, help="write sanitized records here on exit")
    args = ap.parse_args()
    rx = OtlpReceiver(port=args.port).start()
    print(f"listening on 127.0.0.1:{rx.port}", file=sys.stderr)
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        pass
    finally:
        rx.stop()
        if args.out:
            rx.dump(args.out)
            print(datetime.now(UTC).isoformat(timespec="seconds"), "wrote", args.out, file=sys.stderr)


if __name__ == "__main__":
    main()
