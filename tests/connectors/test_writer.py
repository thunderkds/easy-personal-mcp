import builtins
import getpass
import json
import sys

import pytest

from easy_personal.connectors import writer
from easy_personal.surfaces import cli

SECRET = "xoxb-PLANTED-secret-123"
TEAM = "T-PLANTED-team"
EXPECTED_JSON = {"type": "stdio", "command": "/opt/fake/bin/easy-personal-launch", "args": ["exec", "slack"]}


@pytest.fixture
def answers(monkeypatch):
    """Inject prompt answers: secret fields via getpass, plain fields and confirm via input."""
    state = {"secret": SECRET, "plain": TEAM, "confirm": "y", "asked": []}

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


def test_add_stores_secrets_and_registers_launcher(mem_keyring, fake_claude, launcher, answers, capsys):
    rc, out, err = run(capsys, "connector", "add", "slack")
    assert rc == 0
    ((service, user), payload), = mem_keyring.items.items()
    assert (service, user) == ("easy-personal/slack", "connector")
    stored = json.loads(payload)
    assert stored["env"] == {"SLACK_BOT_TOKEN": SECRET, "SLACK_TEAM_ID": TEAM}
    assert stored["command"][0] == "npx"
    assert fake_claude.writes() == [
        ["mcp", "add-json", "slack", json.dumps(EXPECTED_JSON), "--scope", "user"]
    ]
    for text in (out, err, json.dumps(fake_claude.calls())):
        assert SECRET not in text and TEAM not in text
    assert "***" in out and "SLACK_BOT_TOKEN" in out  # the what-will-change summary
    assert ("secret", "Bot token: ") in answers["asked"]


def test_add_json_has_only_launcher_never_env():
    built = json.loads(writer.add_json("slack", "/x/easy-personal-launch"))
    assert built == {"type": "stdio", "command": "/x/easy-personal-launch", "args": ["exec", "slack"]}


def test_yes_flag_skips_confirm(mem_keyring, fake_claude, launcher, answers, capsys):
    rc, *_ = run(capsys, "connector", "add", "slack", "--yes")
    assert rc == 0 and not [a for a in answers["asked"] if a[0] == "confirm"]


def test_abort_at_confirm_writes_nothing(mem_keyring, fake_claude, launcher, answers, capsys):
    answers["confirm"] = "n"
    rc, out, _ = run(capsys, "connector", "add", "slack")
    assert rc == 1 and "aborted" in out
    assert mem_keyring.items == {} and fake_claude.writes() == []


def test_claude_missing_prints_snippet_and_writes_nothing(
    mem_keyring, no_claude, launcher, answers, capsys
):
    rc, out, _ = run(capsys, "connector", "add", "slack")
    assert rc == 0
    assert "claude mcp add-json slack" in out and "--scope user" in out
    assert json.dumps(EXPECTED_JSON) in out
    assert mem_keyring.items == {} and answers["asked"] == []


def test_existing_claude_entry_is_refused_and_both_are_shown(
    mem_keyring, fake_claude, launcher, answers, capsys, monkeypatch
):
    monkeypatch.setenv("FAKE_CLAUDE_GET_RC", "0")
    monkeypatch.setenv(
        "FAKE_CLAUDE_GET_OUT",
        "slack:\n  Scope: User config\n  Type: stdio\n  Command: npx\n"
        "  Args: --token PLANTED-ARG-SECRET\n  Environment:\n    TOKEN=PLANTED-ENV-SECRET\n",
    )
    rc, out, err = run(capsys, "connector", "add", "slack")
    assert rc == 1 and "already exists" in out
    assert "Command: npx" in out  # existing entry shown
    assert json.dumps(EXPECTED_JSON) in out  # proposed entry shown
    assert "PLANTED" not in out + err  # an existing entry's secrets are never echoed
    assert mem_keyring.items == {} and fake_claude.writes() == [] and answers["asked"] == []


def test_existing_keychain_item_is_refused(mem_keyring, fake_claude, launcher, answers, capsys):
    mem_keyring.items[("easy-personal/slack", "connector")] = "{}"
    rc, out, _ = run(capsys, "connector", "add", "slack")
    assert rc == 1 and "already exists" in out and "keychain" in out
    assert fake_claude.writes() == [] and answers["asked"] == []


def test_empty_secret_is_refused(mem_keyring, fake_claude, launcher, answers, capsys):
    answers["secret"] = ""
    rc, out, err = run(capsys, "connector", "add", "slack")
    assert rc == 2 and "empty" in (out + err)
    assert mem_keyring.items == {} and fake_claude.writes() == []


def test_claude_failure_rolls_back_keychain(
    mem_keyring, fake_claude, launcher, answers, capsys, monkeypatch
):
    monkeypatch.setenv("FAKE_CLAUDE_RC", "1")
    monkeypatch.setenv("FAKE_CLAUDE_ERR", "boom")
    rc, out, err = run(capsys, "connector", "add", "slack")
    assert rc == 1 and "boom" in (out + err)
    assert mem_keyring.items == {}
    assert len(fake_claude.writes()) == 1  # add-json was tried, nothing else
    assert SECRET not in out + err


def test_no_keychain_backend_refuses_before_claude(no_keyring, fake_claude, launcher, answers, capsys):
    rc, out, err = run(capsys, "connector", "add", "slack", "--yes")
    assert rc == 1 and "plaintext" in out + err
    assert fake_claude.writes() == [] and SECRET not in out + err


def test_launcher_not_found_is_refused(mem_keyring, fake_claude, answers, capsys, monkeypatch, tmp_path):
    monkeypatch.setattr(sys, "executable", str(tmp_path / "python"))  # no sibling launcher
    rc, out, err = run(capsys, "connector", "add", "slack")
    assert rc == 1 and "easy-personal-launch" in out + err
    assert mem_keyring.items == {} and fake_claude.writes() == []


def test_launcher_path_prefers_sibling_of_interpreter(monkeypatch, tmp_path):
    exe = tmp_path / "easy-personal-launch"
    exe.write_text("#!/bin/sh\n")
    exe.chmod(0o755)
    monkeypatch.setattr(sys, "executable", str(tmp_path / "python"))
    assert writer.launcher_path() == str(exe)


def test_unknown_provider(mem_keyring, fake_claude, launcher, answers, capsys):
    rc, out, err = run(capsys, "connector", "add", "nope")
    assert rc == 2 and "slack" in out + err and fake_claude.writes() == []


def test_remove_deletes_keychain_and_claude_entry(mem_keyring, fake_claude, launcher, answers, capsys):
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
    mem_keyring.items[("easy-personal/slack", "connector")] = "{}"
    monkeypatch.setenv("FAKE_CLAUDE_RC", "1")
    rc, *_ = run(capsys, "connector", "remove", "slack", "--yes")
    assert rc == 1 and len(mem_keyring.items) == 1


def good_get(monkeypatch, command="/opt/fake/bin/easy-personal-launch"):
    monkeypatch.setenv("FAKE_CLAUDE_GET_RC", "0")
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
