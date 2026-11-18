"""Shared helpers for the T102 spike probes (throwaway). Python 3 stdlib only.

Every probe starts the owner's `claude` in a scratch git repo under /tmp with an argument list (never a shell),
with the parent Claude Code session markers removed from the environment and CLAUDE_CONFIG_DIR kept (the logged-in
account). Hooks and MCP config reach `claude` only through --settings <tmp file> / --mcp-config <tmp file>
--strict-mcp-config; nothing here writes a real Claude settings file.
"""
import json, os, queue, shutil, signal, subprocess, sys, tempfile, threading, time

DROP = ["CLAUDECODE", "AI_AGENT", "CLAUDE_CODE_ENTRYPOINT", "CLAUDE_CODE_MESSAGING_SOCKET", "CLAUDE_CODE_MESSAGING_TOKEN",
        "CLAUDE_CODE_EXECPATH", "CLAUDE_CODE_CHILD_SESSION", "CLAUDE_CODE_SESSION_ID", "CLAUDE_CODE_SESSION_ATTENDED",
        "CLAUDE_PID", "CLAUDE_EFFORT"]
BASE = "/tmp/t102"


def clean_env(extra=None):
    env = {k: v for k, v in os.environ.items() if k not in DROP}
    if extra:
        env.update(extra)
    return env


def scratch_repo(name):
    """A fresh git repo under /tmp/t102/<name>."""
    path = os.path.join(BASE, name)
    shutil.rmtree(path, ignore_errors=True)
    os.makedirs(path)
    for cmd in (["git", "init", "-q"], ["git", "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "init"]):
        subprocess.run(cmd, cwd=path, check=True)
    return path


def redact(v, key=None):
    """Event shape: every string value that is not a short fixed word becomes <text>."""
    keep = {"type", "subtype", "role", "name", "behavior", "status", "stop_reason", "hookEventName", "permissionDecision",
            "permission_mode", "terminal_reason", "event", "tool_name", "hook_event", "outcome", "decision", "model_name"}
    if isinstance(v, dict):
        return {k: redact(x, k) for k, x in v.items()}
    if isinstance(v, list):
        return [redact(x, key) for x in v[:6]]
    if isinstance(v, str):
        return v if key in keep or len(v) < 14 and key in ("tool_use_id", "id") else "<text>"
    return v


class Session:
    """A `claude -p --input-format stream-json --output-format stream-json --verbose` child with a reader thread."""

    def __init__(self, cwd, extra_args=(), extra_env=None, base_args=None):
        args = ["claude", "-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
                "--setting-sources", "project", "--model", "haiku"] if base_args is None else list(base_args)
        self.args = args + list(extra_args)
        self.p = subprocess.Popen(self.args, cwd=cwd, env=clean_env(extra_env), stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
        self.q = queue.Queue()
        self.events = []
        self.stderr = []
        threading.Thread(target=self._rd, daemon=True).start()
        threading.Thread(target=self._er, daemon=True).start()

    def _rd(self):
        for line in self.p.stdout:
            try:
                e = json.loads(line)
            except Exception:
                e = {"type": "_unparsed", "raw_len": len(line)}
            e["_t"] = time.time()
            self.events.append(e)
            self.q.put(e)
        self.q.put(None)

    def _er(self):
        for line in self.p.stderr:
            self.stderr.append(line.decode(errors="replace").rstrip())

    def send(self, text=None, raw=None):
        line = raw if raw is not None else json.dumps({"type": "user", "message": {"role": "user", "content": text}})
        self.p.stdin.write((line + "\n").encode())
        self.p.stdin.flush()
        return line

    def until(self, pred, timeout=90):
        """Read events until pred(event) is true; returns the event or None (timeout / end of output)."""
        end = time.time() + timeout
        while time.time() < end:
            try:
                e = self.q.get(timeout=max(0.05, end - time.time()))
            except queue.Empty:
                return None
            if e is None:
                return None
            if pred(e):
                return e
        return None

    def result(self, timeout=90):
        return self.until(lambda e: e.get("type") == "result", timeout)

    def alive(self):
        return self.p.poll() is None

    def close(self, grace=3):
        try:
            self.p.stdin.close()
        except Exception:
            pass
        try:
            self.p.wait(grace)
        except subprocess.TimeoutExpired:
            os.killpg(self.p.pid, signal.SIGTERM)
            try:
                self.p.wait(3)
            except subprocess.TimeoutExpired:
                os.killpg(self.p.pid, signal.SIGKILL)

    def kinds(self):
        return [(e.get("type"), e.get("subtype")) for e in self.events]


def dump(title, obj):
    print(f"--- {title}")
    print(json.dumps(obj, indent=1, ensure_ascii=False)[:4000])
