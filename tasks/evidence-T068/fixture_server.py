"""Fixture dashboard for T068 screenshots: serves the built page and canned JSON. Usage: fixture_server.py PORT STATIC_DIR [variant]"""
import json, sys, time, datetime as dt
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
port, static = int(sys.argv[1]), sys.argv[2]
variant = sys.argv[3] if len(sys.argv) > 3 else "full"
now = dt.datetime.now(dt.timezone.utc)
iso = lambda d: d.strftime("%Y-%m-%dT%H:%M:%SZ")
def acct(i, label, five):
    sl = {"email": None, "plan": None, "organization": None, "last_model": None, "last_active_utc": None, "tokens_today": 0, "tokens_7d": 0,
          "limits": None if five is None else {"five_hour": {"used_percentage": five, "resets_at": iso(now + dt.timedelta(hours=1)), "stale": False}, "seven_day": None, "written_at": iso(now), "stale": False}}
    return {"id": i, "provider": "claude", "label": label, "folder": f"/home/o/.claude{i}", "status": "confirmed", "config_dir_env": None, "workspaces": [], "status_line": sl}
five = {"full": 42, "amber": 72, "red": 91, "nodata": None}.get(variant, 42)
timer_done = variant != "notimer"
data = {
 "/api/accounts": {"by": "account", "accounts": [acct(1, "Claude work", five)]},
 "/api/workspaces": {"workspaces": []},
 "/api/first-run": {"steps": [{"id": "first_ingest", "done": True, "hint": "x"}, {"id": "ingest_timer", "done": timer_done, "hint": "easy-personal schedule install"}, {"id": "accounts_confirmed", "done": True, "hint": "x"}, {"id": "first_workspace", "done": True, "hint": "x"}]},
 "/api/store/health": {"sources": {"claude_code": {"last_ingest_utc": iso(now - dt.timedelta(minutes=4)), "abstained": 0}, "kit_trace": {"last_ingest_utc": None, "abstained": 0}}},
}
per = {"since_utc": None, "until_utc": None}
cost = {"usd": 12.5, "price_table_date": "2026-10-01", "abstained_models": []}
zero = {"period": per, "sessions": 3, "messages": 20, "input_tokens": 1200, "output_tokens": 3400, "cache_creation_input_tokens": 5600, "cache_read_input_tokens": 78900, "cache_hit_ratio": 0.9184,
        "split": {"typed": 50600, "agent": 38500, "unknown": 0}, "entrypoints": {"cli": 50600}, "abstained": {"total": 0, "by_version": {}}}
tot = {"messages": 20, "input_tokens": 1200, "output_tokens": 3400, "cache_creation_input_tokens": 5600, "cache_read_input_tokens": 78900, "api_equivalent_cost": cost, "split": {"typed": 1, "agent": 1, "unknown": 0}}
def snap():
    sess = [] if variant == "idle" else [
      {"session_id": "aaaaaaaa1", "account": "Claude work", "project": "alpha", "last_ts": iso(now), "tool_calls": 3, "tokens_in": 100, "tokens_out": 50, "cost_usd": 0.1, "attention": []},
      {"session_id": "bbbbbbbb2", "account": "Claude work", "project": "beta", "last_ts": iso(now), "tool_calls": 1, "tokens_in": 10, "tokens_out": 5, "cost_usd": 0.01,
       "attention": [{"state": "waiting", "tool": "AskUserQuestion", "count": 1, "is_sidechain": False}]}]
    return {"active_window_s": 300, "last_id": 0, "sessions": sess}
class H(SimpleHTTPRequestHandler):
    def __init__(self, *a, **k): super().__init__(*a, directory=static, **k)
    def log_message(self, *a): pass
    def do_GET(self):
        p = self.path.split("?")[0]
        if p == "/api/live":
            self.send_response(200); self.send_header("Content-Type", "text/event-stream"); self.send_header("Cache-Control", "no-store"); self.end_headers()
            self.wfile.write(f"event: snapshot\ndata: {json.dumps(snap())}\n\n".encode()); self.wfile.flush()
            try:
                while True: time.sleep(5); self.wfile.write(b": ping\n\n"); self.wfile.flush()
            except Exception: pass
            return
        if p.startswith("/api/"):
            body = data.get(p)
            if body is None and p == "/api/usage/summary": body = zero
            if body is None and p.startswith("/api/usage/breakdown"): body = {"by": "model", "period": per, "groups": [], "total": tot}
            if body is None and p == "/api/usage/sessions": body = {"period": per, "gap_minutes": 30, "sessions": []}
            if body is None and p == "/api/usage/outliers": body = {"period": per, "thresholds": {"tool_calls": 100, "context_tokens": 150000}, "sessions": []}
            if body is None: body = {"hits": [], "sessions": [], "gaps": [], "groups": []}
            b = json.dumps(body).encode(); self.send_response(200); self.send_header("Content-Type", "application/json"); self.send_header("Content-Length", str(len(b))); self.end_headers(); self.wfile.write(b); return
        super().do_GET()
ThreadingHTTPServer(("127.0.0.1", port), H).serve_forever()
