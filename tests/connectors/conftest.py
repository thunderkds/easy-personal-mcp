import json
import sys

import keyring
import pytest
from fake_backend import MemoryKeyring
from keyring.backends import fail


@pytest.fixture
def mem_keyring():
    old = keyring.get_keyring()
    backend = MemoryKeyring()
    keyring.set_keyring(backend)
    yield backend
    keyring.set_keyring(old)


@pytest.fixture
def no_keyring():
    old = keyring.get_keyring()
    keyring.set_keyring(fail.Keyring())
    yield
    keyring.set_keyring(old)


FAKE_CLAUDE = """#!{python}
import json, os, sys
args = sys.argv[1:]
config_dir = os.environ.get("CLAUDE_CONFIG_DIR", "<unset>")
with open(os.environ["FAKE_CLAUDE_LOG"], "a") as log:
    log.write(json.dumps({{"argv": args, "config_dir": config_dir}}) + "\\n")
state_path = os.environ.get("FAKE_CLAUDE_STATE")
if state_path:  # per-account user scope: {{config_dir: {{name: entry}}}}, like separate ~/.claude.json files
    state = json.load(open(state_path)) if os.path.exists(state_path) else {{}}
    entries = state.setdefault(config_dir, {{}})
    name = args[2] if len(args) > 2 else ""
    if args[:2] == ["mcp", "get"]:
        if name not in entries:
            sys.exit(1)
        e = entries[name]
        sys.stdout.write(f"{{name}}:\\n  Scope: User config\\n  Type: stdio\\n  Command: {{e['command']}}\\n"
                         f"  Args: {{' '.join(e['args'])}}\\n")
        sys.exit(0)
    if os.environ.get("FAKE_CLAUDE_FAIL") == f"{{args[1]}} {{name}}":
        sys.stderr.write("boom")
        sys.exit(1)
    if args[1] == "add-json":
        if name in entries:
            sys.exit(1)
        entries[name] = json.loads(args[3])
    elif args[1] == "remove":
        if entries.pop(name, None) is None:
            sys.exit(1)
    json.dump(state, open(state_path, "w"))
    sys.exit(0)
if args[:2] == ["mcp", "get"]:
    if os.environ.get("FAKE_CLAUDE_GET_ONLY", args[2]) != args[2]:
        sys.exit(1)  # only this one connector name exists in Claude Code
    sys.stdout.write(os.environ.get("FAKE_CLAUDE_GET_OUT", ""))
    sys.exit(int(os.environ.get("FAKE_CLAUDE_GET_RC", "1")))
sys.stderr.write(os.environ.get("FAKE_CLAUDE_ERR", ""))
sys.exit(int(os.environ.get("FAKE_CLAUDE_RC", "0")))
"""
FAKE_LAUNCHER = "/opt/fake/bin/easy-personal-launch"


class FakeClaude:
    def __init__(self, log):
        self.log = log

    def records(self):
        """Every call: {"argv": [...], "config_dir": CLAUDE_CONFIG_DIR or "<unset>"}."""
        return [json.loads(x) for x in self.log.read_text().splitlines()] if self.log.exists() else []

    def calls(self):
        return [r["argv"] for r in self.records()]

    def write_records(self):
        return [(r["config_dir"], r["argv"]) for r in self.records() if r["argv"][:2] != ["mcp", "get"]]

    def state(self):
        """The fake per-account user scope (only with the `claude_accounts` fixture)."""
        path = self.log.parent / "claude-state.json"
        return json.loads(path.read_text()) if path.exists() else {}

    def writes(self):
        return [c for c in self.calls() if c[:2] != ["mcp", "get"]]


@pytest.fixture
def fake_claude(tmp_path, monkeypatch):
    """A fake `claude` first on PATH that records argv; the real CLI is never run."""
    bindir = tmp_path / "bin"
    bindir.mkdir()
    exe = bindir / "claude"
    exe.write_text(FAKE_CLAUDE.format(python=sys.executable))
    exe.chmod(0o755)
    log = tmp_path / "claude-argv.log"
    monkeypatch.setenv("PATH", str(bindir))  # nothing else on PATH: the real claude is unreachable
    monkeypatch.setenv("FAKE_CLAUDE_LOG", str(log))
    return FakeClaude(log)


@pytest.fixture
def no_claude(tmp_path, monkeypatch):
    empty = tmp_path / "empty"
    empty.mkdir()
    monkeypatch.setenv("PATH", str(empty))


@pytest.fixture
def launcher(monkeypatch):
    from easy_personal.connectors import writer

    monkeypatch.setattr(writer, "launcher_path", lambda: FAKE_LAUNCHER)
    return FAKE_LAUNCHER


@pytest.fixture
def claude_state(fake_claude, monkeypatch):
    """Make the fake `claude` keep a per-CLAUDE_CONFIG_DIR registry (get/add-json/remove behave)."""
    monkeypatch.setenv("FAKE_CLAUDE_STATE", str(fake_claude.log.parent / "claude-state.json"))
    return fake_claude
