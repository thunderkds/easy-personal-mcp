"""Shared helpers for the T110 spike probes (throwaway). Python 3 stdlib only.
claude is started with an argument list (never a shell) in a scratch git repo under /tmp/t110, with the parent Claude Code
markers dropped from the environment. CLAUDE_CONFIG_DIR is kept (the logged-in account, as in T102). Hooks reach claude only
through --settings <tmp file>; nothing here writes a real Claude settings file."""
import json, os, shutil, subprocess, time

DROP = ["CLAUDECODE", "AI_AGENT", "CLAUDE_CODE_ENTRYPOINT", "CLAUDE_CODE_MESSAGING_SOCKET", "CLAUDE_CODE_MESSAGING_TOKEN",
        "CLAUDE_CODE_EXECPATH", "CLAUDE_CODE_CHILD_SESSION", "CLAUDE_CODE_SESSION_ID", "CLAUDE_CODE_SESSION_ATTENDED",
        "CLAUDE_PID", "CLAUDE_EFFORT"]
BASE = "/tmp/t110"
HERE = os.path.dirname(os.path.abspath(__file__))
HOOK = f"python3 {HERE}/stop_hook.py"


def clean_env(extra=None):
    env = {k: v for k, v in os.environ.items() if k not in DROP}
    env.update(extra or {})
    return env


def scratch_repo(name):
    path = os.path.join(BASE, name)
    shutil.rmtree(path, ignore_errors=True)
    os.makedirs(path)
    for cmd in (["git", "init", "-q"], ["git", "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "init"]):
        subprocess.run(cmd, cwd=path, check=True)
    return path


def write_settings(work, timeout=None, with_hook=True):
    """--settings file with one Stop hook (timeout in seconds, omitted = Claude Code default)."""
    os.makedirs(work, exist_ok=True)
    h = {"type": "command", "command": HOOK}
    if timeout is not None:
        h["timeout"] = timeout
    s = {"hooks": {"Stop": [{"hooks": [h]}]}} if with_hook else {}
    p = f"{work}/settings.json"
    json.dump(s, open(p, "w"))
    return p


def hook_env(work, modes, reason=None):
    """Env for a probe: HOOK_LOG (jsonl), HOOK_MODES (a JSON list, one mode per Stop call; past the end = empty), HOOK_REASON file."""
    log, mf = f"{work}/hook.log", f"{work}/modes.json"
    for f in (log, mf + ".n", f"{work}/reason.txt"):
        if os.path.exists(f):
            os.remove(f)
    json.dump(modes, open(mf, "w"))
    if reason is not None:
        open(f"{work}/reason.txt", "w").write(reason)
    return {"HOOK_LOG": log, "HOOK_MODES": mf, "HOOK_REASON": f"{work}/reason.txt"}


def read_log(work):
    p = f"{work}/hook.log"
    return [json.loads(l) for l in open(p)] if os.path.exists(p) else []


def run_p(repo, work, prompt, settings, env_extra, timeout=120, extra_args=()):
    """Headless run: claude -p <prompt> --output-format stream-json --verbose. Returns (events, wall_s, returncode, stderr)."""
    args = ["claude", "-p", prompt, "--output-format", "stream-json", "--verbose", "--setting-sources", "project",
            "--settings", settings, "--model", "haiku", *extra_args]
    t0 = time.time()
    r = subprocess.run(args, cwd=repo, env=clean_env(env_extra), capture_output=True, text=True, timeout=timeout)
    ev = []
    for l in r.stdout.splitlines():
        try:
            e = json.loads(l)
        except Exception:
            continue
        ev.append(e)
    return ev, time.time() - t0, r.returncode, r.stderr


def transcript(session_id):
    """The session's transcript file under the logged-in config dir (read only)."""
    root = os.environ.get("CLAUDE_CONFIG_DIR") or os.path.expanduser("~/.claude")
    for d, _, fs in os.walk(os.path.join(root, "projects")):
        if f"{session_id}.jsonl" in fs:
            return os.path.join(d, f"{session_id}.jsonl")


def shape(o, key=None):
    """Record shape: every string that is not a short fixed word becomes <text>."""
    keep = {"type", "subtype", "role", "name", "hookEvent", "hookName", "level", "stop_reason", "attachmentType", "decision", "hook_event"}
    if isinstance(o, dict):
        return {k: shape(v, k) for k, v in o.items() if k not in ("uuid", "parentUuid", "sessionId", "timestamp", "cwd", "gitBranch", "version", "requestId", "id", "signature")}
    if isinstance(o, list):
        return [shape(x, key) for x in o[:4]]
    if isinstance(o, str):
        return o if key in keep else "<text>"
    return o
