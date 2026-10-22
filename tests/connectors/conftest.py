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
with open(os.environ["FAKE_CLAUDE_LOG"], "a") as log:
    log.write(json.dumps(sys.argv[1:]) + "\\n")
args = sys.argv[1:]
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

    def calls(self):
        return [json.loads(x) for x in self.log.read_text().splitlines()] if self.log.exists() else []

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
