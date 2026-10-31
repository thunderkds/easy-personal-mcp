"""E2 hook collector (HUN-167): tiny local HTTP server for Claude Code `http` hooks.

POST /hook with the hook's JSON body. Keeps ONLY: hook_event_name, session_id, tool_use_id,
tool_name, agent_id, and the local receive time (recv_ms, epoch ms). Everything else in the body
(tool_input, tool_response, prompt, last_assistant_message, ...) is dropped before anything is
written. The NAMES of the top-level body fields are kept per event in `<out>.fields.json`
(names only, never values). Replies 200 with an empty body (the docs call a non-empty,
non-JSON 2xx body a hook failure).

Usage: python collector.py [--port 4390] [--out FILE]   (stdlib only; no auth, no retries)
Python:  with Collector(out_path, port=4390) as c: ...   (c.port is the bound port; port=0 = free port)
"""
import argparse
import json
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

DEFAULT_PORT = 4390
KEEP_KEYS = ("hook_event_name", "session_id", "tool_use_id", "tool_name", "agent_id")
MAX_BODY = 256 * 1024 * 1024  # refuse absurd bodies; a >1 MB tool_response must still be accepted


def reduce_event(body, recv_ms: float):
    """Allowed keys only. Returns None when the body is not a JSON object."""
    if not isinstance(body, dict):
        return None
    rec = {k: body[k] for k in KEEP_KEYS if isinstance(body.get(k), str)}
    rec["recv_ms"] = round(recv_ms, 3)
    return rec


class _State:
    def __init__(self, out_path):
        self.out_path = Path(out_path)
        self.lock = threading.Lock()
        self.fields: dict[str, set] = {}
        self.out_path.parent.mkdir(parents=True, exist_ok=True)
        self.fh = open(self.out_path, "a", buffering=1, encoding="utf-8")

    def add(self, rec, names, event):
        with self.lock:
            self.fh.write(json.dumps(rec) + "\n")
            self.fields.setdefault(event or "?", set()).update(names)

    def dump_fields(self):
        with self.lock:
            data = {e: sorted(v) for e, v in sorted(self.fields.items())}
        Path(str(self.out_path) + ".fields.json").write_text(json.dumps(data, indent=1) + "\n")


def make_handler(state: _State, delay_ms: float = 0.0):
    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def log_message(self, *a):  # silence: nothing but metadata is ever written
            pass

        def _reply(self, code):
            self.send_response(code)
            self.send_header("Content-Length", "0")
            self.end_headers()

        def do_POST(self):
            recv_ms = time.time() * 1000.0
            if self.path != "/hook":
                self._reply(404)
                return
            try:
                n = int(self.headers.get("Content-Length") or 0)
            except ValueError:
                n = -1
            if n < 0 or n > MAX_BODY:
                self._reply(413)
                return
            raw = self.rfile.read(n)
            try:
                body = json.loads(raw)
            except ValueError:
                body = None
            del raw
            rec = reduce_event(body, recv_ms)
            if rec is None:
                self._reply(400)
                return
            state.add(rec, [k for k in body if isinstance(k, str)], rec.get("hook_event_name"))
            del body
            if delay_ms:
                time.sleep(delay_ms / 1000.0)  # test knob: a slow collector
            self._reply(200)

    return Handler


class Collector:
    """Run the collector in a background thread (tests) or use `python collector.py` (sessions)."""

    def __init__(self, out_path, port=DEFAULT_PORT, host="127.0.0.1", delay_ms=0.0):
        self.state = _State(out_path)
        self.server = ThreadingHTTPServer((host, port), make_handler(self.state, delay_ms))
        self.server.daemon_threads = True
        self.port = self.server.server_address[1]
        self._t = threading.Thread(target=self.server.serve_forever, daemon=True)

    def start(self):
        self._t.start()
        return self

    def stop(self):
        self.server.shutdown()
        self.server.server_close()
        self.state.dump_fields()
        self.state.fh.close()

    def __enter__(self):
        return self.start()

    def __exit__(self, *a):
        self.stop()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--port", type=int, default=DEFAULT_PORT)
    ap.add_argument("--out", required=True, help="JSONL file for the reduced events")
    ap.add_argument("--delay-ms", type=float, default=0.0, help="reply delay (slow-collector test)")
    args = ap.parse_args()
    c = Collector(args.out, args.port, delay_ms=args.delay_ms).start()
    print(f"collector listening on 127.0.0.1:{c.port} -> {args.out}", flush=True)
    try:
        while True:
            time.sleep(0.5)
            c.state.dump_fields()
    except KeyboardInterrupt:
        pass
    finally:
        c.stop()


if __name__ == "__main__":
    main()
