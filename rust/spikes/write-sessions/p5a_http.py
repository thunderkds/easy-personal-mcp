"""AC5 (a): claude -> permission tool over loopback HTTP (a stand-in with mcp_http.rs's HTTP rules) with a per-session
header and a 65 s wait; plus the real `easy-personal-rs mcp --transport http` connected in the same run to show claude
accepts that server's stateless JSON-only style.  usage: p5a_http.py [delay_seconds] [real-binary]"""
import json, os, subprocess, sys, time
from cc import *

delay = sys.argv[1] if len(sys.argv) > 1 else "65"
real = sys.argv[2] if len(sys.argv) > 2 else None
here = os.path.dirname(os.path.abspath(__file__))
repo = scratch_repo("p5a")
work = os.path.join(BASE, "p5a_cfg"); os.makedirs(work, exist_ok=True)
log = f"{work}/http.log"; open(log, "w").close()
pf = f"{work}/port"
if os.path.exists(pf): os.remove(pf)
srv = subprocess.Popen([sys.executable, f"{here}/perm_http_server.py", pf, log], env={**os.environ, "PERM_DELAY": delay})
while not os.path.exists(pf) or not open(pf).read(): time.sleep(0.1)
port = int(open(pf).read())
servers = {"perm": {"type": "http", "url": f"http://127.0.0.1:{port}/mcp", "headers": {"X-Spike-Session": "sess-123", "Authorization": "Bearer spike-token"}}}
real_p = None
if real:
    real_p = subprocess.Popen([real, "mcp", "--transport", "http", "--port", "0"], stderr=subprocess.PIPE, stdout=subprocess.DEVNULL)
    line = real_p.stderr.readline().decode()
    rport = [t for t in line.replace(":", " ").replace("/", " ").split() if t.isdigit()][-1]
    servers["ep"] = {"type": "http", "url": f"http://127.0.0.1:{rport}/mcp"}
json.dump({"mcpServers": servers}, open(f"{work}/mcp.json", "w"))
s = Session(repo, ["--mcp-config", f"{work}/mcp.json", "--strict-mcp-config", "--permission-prompt-tool", "mcp__perm__permission_prompt"])
s.send("Use the Bash tool to run exactly: touch spike-marker.txt . Then say done.")
t0 = time.time()
r = s.result(delay and int(float(delay)) + 120)
print(f"result after {time.time()-t0:.1f}s:", None if r is None else (r.get("subtype"), r.get("is_error"), r.get("num_turns")))
print("mcp_servers in init:", s.events[0].get("mcp_servers"))
print("marker created:", os.path.exists(f"{repo}/spike-marker.txt"))
print("--- http log")
for l in open(log):
    o = json.loads(l)
    print({k: v for k, v in o.items() if k != "t"})
s.close(); srv.terminate()
if real_p: real_p.terminate()
print("stderr:", s.stderr[:5])
