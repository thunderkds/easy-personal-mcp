"""AC5 (a): a stand-in for a permission tool served over loopback HTTP, with the same HTTP rules as mcp_http.rs:
POST /mcp takes one JSON-RPC message, answers 200 application/json (202, no body, for a notification), GET and DELETE
are 405 (no sessions, no SSE stream), Host must be 127.0.0.1:<port>. Logs the headers it got (to see that a per-session
header from --mcp-config arrives) and holds each tools/call for $PERM_DELAY seconds before answering (an owner thinking).
usage: perm_http_server.py <port-file> <log> ; env PERM_DELAY
"""
import json, os, sys, threading, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORTFILE, LOG = sys.argv[1], sys.argv[2]
DELAY = float(os.environ.get("PERM_DELAY", "0"))


def log(o):
    with open(LOG, "a") as f:
        f.write(json.dumps({"t": time.time(), **o}) + "\n")


class H(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *a):
        pass

    def _send(self, code, body=None):
        data = b"" if body is None else json.dumps(body).encode()
        self.send_response(code)
        if body is not None:
            self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        log({"http": "GET", "path": self.path})
        self._send(405)

    do_DELETE = do_GET

    def do_POST(self):
        host = self.headers.get("Host", "")
        n = int(self.headers.get("Content-Length", "0"))
        m = json.loads(self.rfile.read(n))
        hdr = {k: ("<redacted>" if k.lower() == "authorization" else v) for k, v in self.headers.items()
               if k.lower() in ("x-spike-session", "authorization", "accept", "content-type", "mcp-protocol-version", "mcp-session-id", "user-agent", "host", "origin")}
        log({"http": "POST", "path": self.path, "rpc": m.get("method"), "headers": hdr,
             "bearer_ok": self.headers.get("Authorization") == "Bearer spike-token"})
        if not host.startswith("127.0.0.1:"):
            return self._send(421, {"error": "bad host"})
        method, mid = m.get("method"), m.get("id")
        if mid is None:
            return self._send(202)
        if method == "initialize":
            return self._send(200, {"jsonrpc": "2.0", "id": mid, "result": {"protocolVersion": m["params"].get("protocolVersion", "2025-06-18"),
                              "capabilities": {"tools": {}}, "serverInfo": {"name": "perm", "version": "0"}}})
        if method == "tools/list":
            return self._send(200, {"jsonrpc": "2.0", "id": mid, "result": {"tools": [{"name": "permission_prompt", "description": "spike",
                              "inputSchema": {"type": "object", "properties": {"tool_name": {"type": "string"}, "input": {"type": "object"}, "tool_use_id": {"type": "string"}}}}]}})
        if method == "tools/call":
            args = m["params"].get("arguments", {})
            log({"call": args.get("tool_name"), "input_keys": sorted(args.get("input", {}).keys())})
            time.sleep(DELAY)
            log({"answering": True})
            return self._send(200, {"jsonrpc": "2.0", "id": mid, "result": {"content": [{"type": "text", "text": json.dumps({"behavior": "allow", "updatedInput": args.get("input")})}]}})
        return self._send(200, {"jsonrpc": "2.0", "id": mid, "result": {}})


srv = ThreadingHTTPServer(("127.0.0.1", 0), H)
srv.daemon_threads = True
open(PORTFILE, "w").write(str(srv.server_address[1]))
srv.serve_forever()
