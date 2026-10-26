import builtins
import getpass
import json
import sys

import pytest

from easy_personal.connectors import writer
from easy_personal.surfaces import cli

SECRET = "xoxb-PLANTED-secret-123"
EXPECTED_JSON = {"type": "stdio", "command": "/opt/fake/bin/easy-personal-launch", "args": ["exec", "slack"]}


@pytest.fixture
def answers(monkeypatch):
    """Inject prompt answers: secret fields via getpass, plain fields and confirm via input."""
    state = {"secret": SECRET, "confirm": "y", "asked": []}

    def fake_getpass(prompt=""):
        state["asked"].append(("secret", prompt))
        return state["secret"]

    def fake_input(prompt=""):
        kind = "confirm" if "[y/N]" in prompt else "plain"
        state["asked"].append((kind, prompt))
        return state[kind]

    monkeypatch.setattr(getpass, "getpass", fake_getpass)
    monkeypatch.setattr(builtins, "input", fake_input)
    return state


def run(capsys, *argv):
    rc = cli.main(list(argv))
    out = capsys.readouterr()
    return rc, out.out, out.err


def test_add_json_has_only_launcher_never_env():
    built = json.loads(writer.add_json("slack", "/x/easy-personal-launch"))
    assert built == {"type": "stdio", "command": "/x/easy-personal-launch", "args": ["exec", "slack"]}


def test_launcher_path_prefers_sibling_of_interpreter(monkeypatch, tmp_path):
    exe = tmp_path / "easy-personal-launch"
    exe.write_text("#!/bin/sh\n")
    exe.chmod(0o755)
    monkeypatch.setattr(sys, "executable", str(tmp_path / "python"))
    assert writer.launcher_path() == str(exe)


def test_remove_deletes_keychain_and_claude_entry(mem_keyring, fake_claude, launcher, answers, capsys, monkeypatch):
    good_get(monkeypatch)
    mem_keyring.items[("easy-personal/slack", "connector")] = "{}"
    rc, _, _ = run(capsys, "connector", "remove", "slack")
    assert rc == 0 and mem_keyring.items == {}
    assert fake_claude.writes() == [["mcp", "remove", "slack", "--scope", "user"]]


def test_remove_abort_deletes_nothing(mem_keyring, fake_claude, launcher, answers, capsys):
    mem_keyring.items[("easy-personal/slack", "connector")] = "{}"
    answers["confirm"] = "n"
    rc, *_ = run(capsys, "connector", "remove", "slack")
    assert rc == 1 and len(mem_keyring.items) == 1 and fake_claude.writes() == []


def test_remove_claude_missing_prints_snippet_and_keeps_keychain(mem_keyring, no_claude, answers, capsys):
    mem_keyring.items[("easy-personal/slack", "connector")] = "{}"
    rc, out, _ = run(capsys, "connector", "remove", "slack")
    assert rc == 0 and "claude mcp remove slack --scope user" in out
    assert len(mem_keyring.items) == 1


def test_remove_claude_failure_keeps_keychain(mem_keyring, fake_claude, answers, capsys, monkeypatch):
    good_get(monkeypatch)
    mem_keyring.items[("easy-personal/slack", "connector")] = "{}"
    monkeypatch.setenv("FAKE_CLAUDE_RC", "1")
    rc, *_ = run(capsys, "connector", "remove", "slack", "--yes")
    assert rc == 1 and len(mem_keyring.items) == 1


def good_get(monkeypatch, command="/opt/fake/bin/easy-personal-launch"):
    monkeypatch.setenv("FAKE_CLAUDE_GET_RC", "0")
    monkeypatch.setenv("FAKE_CLAUDE_GET_ONLY", "slack")
    monkeypatch.setenv(
        "FAKE_CLAUDE_GET_OUT",
        f"slack:\n  Scope: User config\n  Type: stdio\n  Command: {command}\n  Args: exec slack\n",
    )


def test_doctor_all_ok(mem_keyring, fake_claude, launcher, capsys, monkeypatch):
    mem_keyring.items[("easy-personal/slack", "connector")] = "{}"
    good_get(monkeypatch)
    rc, out, _ = run(capsys, "doctor")
    assert rc == 0 and "FAIL" not in out
    for word in ("keychain", "claude", "launcher", "slack"):
        assert word in out


def test_doctor_entry_not_pointing_at_launcher(mem_keyring, fake_claude, launcher, capsys, monkeypatch):
    good_get(monkeypatch, command="npx")
    rc, out, _ = run(capsys, "doctor")
    assert rc == 1 and "FAIL" in out and "slack" in out


def test_doctor_claude_missing(mem_keyring, no_claude, launcher, capsys):
    rc, out, _ = run(capsys, "doctor")
    assert rc == 1 and "FAIL claude" in out


def test_doctor_no_keychain_backend(no_keyring, fake_claude, launcher, capsys):
    rc, out, _ = run(capsys, "doctor")
    assert rc == 1 and "FAIL keychain" in out


def test_doctor_keychain_without_claude_entry(mem_keyring, fake_claude, launcher, capsys):
    mem_keyring.items[("easy-personal/slack", "connector")] = "{}"
    rc, out, _ = run(capsys, "doctor")  # get returns rc 1 -> not registered
    assert rc == 1 and "FAIL slack" in out
