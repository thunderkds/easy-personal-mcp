"""AC5 (b): claude -> stdio child (perm_fwd_server.py) -> unix socket -> "the app" (this script), which answers after argv[1] seconds (default 3)."""
import json, os, socket, threading, time
from cc import *

here = os.path.dirname(os.path.abspath(__file__))
repo = scratch_repo("p5b")
work = os.path.join(BASE, "p5b_cfg"); os.makedirs(work, exist_ok=True)
sock = f"{work}/app.sock"
if os.path.exists(sock): os.remove(sock)
got = []
import sys
DELAY = float(sys.argv[1]) if len(sys.argv) > 1 else 3
srv = socket.socket(socket.AF_UNIX); srv.bind(sock); os.chmod(sock, 0o600); srv.listen(4)


def app():
    while True:
        c, _ = srv.accept()
        data = b""
        while not data.endswith(b"\n"):
            data += c.recv(65536)
        req = json.loads(data)
        import struct
        uid = struct.unpack("3i", c.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, struct.calcsize("3i")))[1]
        got.append({"session": req["session"], "tool_name": req["args"].get("tool_name"), "input_keys": sorted(req["args"].get("input", {})), "peer_uid_is_me": uid == os.getuid()})
        time.sleep(DELAY)
        c.sendall((json.dumps({"behavior": "allow", "updatedInput": req["args"].get("input")}) + "\n").encode()); c.close()


threading.Thread(target=app, daemon=True).start()
json.dump({"mcpServers": {"perm": {"command": "python3", "args": [f"{here}/perm_fwd_server.py"], "env": {"PERM_SOCK": sock, "PERM_SESSION": "sess-456"}}}}, open(f"{work}/mcp.json", "w"))
s = Session(repo, ["--mcp-config", f"{work}/mcp.json", "--strict-mcp-config", "--permission-prompt-tool", "mcp__perm__permission_prompt"])
s.send("Use the Bash tool to run exactly: touch spike-marker.txt . Then say done.")
r = s.result(int(DELAY) + 120)
print("result:", None if r is None else (r.get("subtype"), r.get("is_error"), r.get("num_turns")))
print("marker created:", os.path.exists(f"{repo}/spike-marker.txt"))
print("app saw:", got)
pid = s.p.pid
s.close()
time.sleep(1)
print("stdio child gone after claude exit:", subprocess.run(["pgrep", "-f", "perm_fwd_server.py"], capture_output=True).stdout == b"")
print("stderr:", s.stderr[:5])
