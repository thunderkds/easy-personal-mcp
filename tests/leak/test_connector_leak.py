"""A secret typed into `connector add` never reaches stdout, stderr, logs, `claude` argv, the store or disk.

HUN-65: the flow is add-in-workspace -> account use -> sync (per account) -> doctor -> remove <id>.

The keychain is an in-memory fake and `claude` is a fake script on PATH; the real ones are never used.
"""
import builtins
import getpass
import json
import re
import sys

import keyring
import pytest

from connectors.fake_backend import MemoryKeyring
from easy_personal.connectors import writer
from easy_personal.surfaces import cli
from leak import harness
from leak.canaries import Canaries

FAKE_CLAUDE = """#!{python}
import json, os, sys
open(os.environ["FAKE_CLAUDE_LOG"], "a").write(json.dumps(sys.argv[1:]) + "\\n")
if sys.argv[2:3] == ["get"]:
    sys.exit(1)
sys.stderr.write("claude: could not write")
sys.exit(int(os.environ.get("FAKE_CLAUDE_RC", "0")))
"""


@pytest.fixture
def world(tmp_path, monkeypatch):
    bindir = tmp_path / "bin"
    bindir.mkdir()
    exe = bindir / "claude"
    exe.write_text(FAKE_CLAUDE.format(python=sys.executable))
    exe.chmod(0o755)
    home = tmp_path / "home"
    home.mkdir()
    agents = tmp_path / "agents"
    (agents / ".claude").mkdir(parents=True)  # the default account: CLAUDE_CONFIG_DIR unset
    (agents / ".claude-work" / "projects").mkdir(parents=True)
    monkeypatch.setenv("EASY_PERSONAL_AGENT_ROOT", str(agents))
    monkeypatch.setenv("PATH", str(bindir))
    monkeypatch.setenv("FAKE_CLAUDE_LOG", str(tmp_path / "argv.log"))
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(home))
    monkeypatch.setattr(writer, "launcher_path", lambda: "/opt/fake/easy-personal-launch")
    old = keyring.get_keyring()
    keyring.set_keyring(MemoryKeyring())
    yield tmp_path, home
    keyring.set_keyring(old)


def _run(capsys, monkeypatch, canaries, rc_claude, argv, confirm="y"):
    monkeypatch.setenv("FAKE_CLAUDE_RC", str(rc_claude))
    monkeypatch.setattr(getpass, "getpass", lambda prompt="": canaries.values["api_key"])
    monkeypatch.setattr(
        builtins, "input", lambda prompt="": confirm if "[y/N]" in prompt else canaries.values["email"]
    )
    code = cli.main(argv)
    out = capsys.readouterr()
    return code, {"stdout": out.out, "stderr": out.err}


def test_secret_typed_into_connector_add_never_leaks(world, capsys, monkeypatch):
    tmp_path, _ = world
    canaries = Canaries.new()
    blobs = {}

    def step(rc_claude, argv):
        code, shown = _run(capsys, monkeypatch, canaries, rc_claude, argv)
        blobs[f"{argv} rc={rc_claude} -> {code}"] = shown
        return code, shown

    with harness.LogCapture() as logs:
        step(0, ["workspace", "add", "company"])
        step(0, ["account", "confirm", str(tmp_path / "agents" / ".claude-work"), "--label", "Claude work"])
        _, shown = step(0, ["connector", "add", "slack", "--workspace", "company"])  # success
        connector_id = re.search(r"\bc[0-9a-f]{8}\b", shown["stdout"])[0]
        step(0, ["connector", "add", "slack", "--workspace", "company", "--yes"])  # one per provider: refused
        step(0, ["connector", "add", "gmail", "--workspace", "company", "--alias", "mail", "--yes"])
        step(0, ["account", "use", "1", "company"])
        step(0, ["account", "use", "2", "company"])
        step(1, ["connector", "sync", "--yes"])  # claude fails -> stops, nothing recorded
        step(0, ["connector", "sync"])
        step(0, ["doctor"])
        step(0, ["connector", "list"])
        step(0, ["connector", "alias", connector_id, "team-chat", "--yes"])  # HUN-66 operations
        step(0, ["connector", "alias", connector_id, "--default", "--yes"])
        step(0, ["workspace", "edit", "company", "--rename", "firm", "--yes"])
        step(0, ["workspace", "delete", "firm", "--yes"])  # connectors go to orphan
        step(0, ["workspace", "add", "side"])
        step(0, ["connector", "move", connector_id, "side", "--yes"])
        step(0, ["connector", "move", connector_id, canaries.values["api_key"], "--yes"])  # refused, no echo
        step(0, ["connector", "remove", connector_id, "--yes"])
        step(0, ["connector", "remove", "slack", "--yes"])  # pre-workspace path
    # the argv the fake claude saw, and every file written under the home or the temp tree (store included)
    blobs["claude argv"] = (tmp_path / "argv.log").read_text()
    blobs["logs"] = logs.text()
    for path in sorted(p for p in tmp_path.rglob("*") if p.is_file() and p.name != "claude"):
        blobs[f"file {path.relative_to(tmp_path)}"] = path.read_bytes()
    for surface, blob in blobs.items():
        canaries.assert_clean(surface, blob)
    assert any(k.startswith("file home/store.sqlite") for k in blobs)  # the store was scanned too
    calls = [json.loads(x) for x in (tmp_path / "argv.log").read_text().splitlines()]
    assert ["mcp", "add-json"] in [c[:2] for c in calls] and ["mcp", "remove"] in [c[:2] for c in calls]


def test_scan_would_catch_a_leaking_argv(world, capsys, monkeypatch):
    canaries = Canaries.new()
    leaked = json.dumps(["mcp", "add-json", "slack", canaries.values["api_key"]])
    assert canaries.find(leaked)
