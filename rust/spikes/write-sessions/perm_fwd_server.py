"""AC5 (b): a stdio MCP child (what `<binary> mcp-permission` would be) that forwards each permission prompt over a unix
socket to the "app" (p5b_stdio_socket.py plays the app) and returns the app's answer. Session identity comes from argv
(set per session in the temp --mcp-config). The socket check the real thing needs: mode 0600 + peer uid (SO_PEERCRED).
env: PERM_SOCK (socket path), PERM_SESSION."""
import json, os, socket, sys

SOCK, SESSION = os.environ["PERM_SOCK"], os.environ.get("PERM_SESSION", "")


def send(msg):
    sys.stdout.write(json.dumps(msg) + "\n"); sys.stdout.flush()


def ask_app(args):
    c = socket.socket(socket.AF_UNIX); c.connect(SOCK)
    c.sendall((json.dumps({"session": SESSION, "args": args}) + "\n").encode())
    data = b""
    while not data.endswith(b"\n"):
        chunk = c.recv(65536)
        if not chunk:
            break
        data += chunk
    return data.decode().strip()


for line in sys.stdin:
    m = json.loads(line); method, mid = m.get("method"), m.get("id")
    if method == "initialize":
        send({"jsonrpc": "2.0", "id": mid, "result": {"protocolVersion": m["params"].get("protocolVersion", "2025-06-18"), "capabilities": {"tools": {}}, "serverInfo": {"name": "perm", "version": "0"}}})
    elif method == "tools/list":
        send({"jsonrpc": "2.0", "id": mid, "result": {"tools": [{"name": "permission_prompt", "description": "spike", "inputSchema": {"type": "object", "properties": {"tool_name": {"type": "string"}, "input": {"type": "object"}, "tool_use_id": {"type": "string"}}}}]}})
    elif method == "tools/call":
        send({"jsonrpc": "2.0", "id": mid, "result": {"content": [{"type": "text", "text": ask_app(m["params"].get("arguments", {}))}]}})
    elif mid is not None:
        send({"jsonrpc": "2.0", "id": mid, "result": {}})
