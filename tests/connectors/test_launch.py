import ast
import json
import subprocess
import sys
from pathlib import Path

import pytest

from easy_personal import launch
from easy_personal.connectors import keychain

SECRET = "xoxb-PLANTED-secret-é\nline2"
SRC = Path(launch.__file__)

# Child process: installs the in-memory backend, stores a connector, then runs the real launcher.
CHILD = """
import time
_t0 = time.perf_counter()
import sys, keyring
sys.path.insert(0, {tests!r})
from fake_backend import MemoryKeyring
keyring.set_keyring(MemoryKeyring())
from easy_personal.connectors import keychain
from easy_personal import launch
keychain.store("slack", {command!r}, env={{"SLACK_TOKEN": {secret!r}}}, headers={{"Authorization": {secret!r}}})
sys.exit(launch.main(sys.argv[1:]))
"""


def run_child(action, command, name="slack"):
    code = CHILD.format(tests=str(Path(__file__).parent), command=command, secret=SECRET)
    return subprocess.run([sys.executable, "-c", code, action, name],
                          check=False, capture_output=True, text=True)


def test_exec_puts_secret_in_env_not_argv():
    probe = ("import os;print(os.environ['SLACK_TOKEN'].startswith('xoxb-'));"
             "print(open('/proc/self/cmdline').read().replace(chr(0),' '))")
    res = run_child("exec", [sys.executable, "-c", probe])
    assert res.returncode == 0, res.stderr
    lines = res.stdout.split("\n", 1)
    assert lines[0] == "True"
    assert "PLANTED" not in lines[1]


def test_exec_uses_os_execvpe(mem_keyring, monkeypatch):
    keychain.store("slack", ["prog", "arg"], env={"A": "b"})
    calls = []
    monkeypatch.setattr(launch.os, "execvpe", lambda *a: calls.append(a))
    launch.run_exec("slack")
    file, argv, env = calls[0]
    assert (file, argv) == ("prog", ["prog", "arg"]) and env["A"] == "b"


def test_exec_refuses_without_backend(no_keyring, capsys):
    assert launch.main(["exec", "slack"]) == 1
    err = capsys.readouterr().err
    assert err.count("\n") == 1 and "plaintext" in err


def test_exec_unknown_connector(mem_keyring, capsys):
    assert launch.main(["exec", "nope"]) == 1
    assert "not configured" in capsys.readouterr().err


def test_exec_missing_command(mem_keyring, capsys):
    keychain.store("slack", ["definitely-not-on-path-xyz"], env={"T": SECRET})
    assert launch.main(["exec", "slack"]) == 1
    err = capsys.readouterr().err
    assert "cannot start" in err and SECRET not in err


def test_headers_prints_json_only(mem_keyring, capsys):
    keychain.store("slack", ["x"], headers={"Authorization": SECRET})
    assert launch.main(["headers", "slack"]) == 0
    out = capsys.readouterr()
    assert json.loads(out.out) == {"Authorization": SECRET}
    assert out.out.count("\n") == 1 and out.err == ""


def test_headers_refuses_without_backend(no_keyring, capsys):
    assert launch.main(["headers", "slack"]) == 1
    assert capsys.readouterr().out == ""


def test_bad_usage(capsys):
    assert launch.main(["exec"]) == 2


def test_imports_only_stdlib_and_keyring():
    allowed = set(sys.stdlib_module_names) | {"keyring"}
    tree = ast.parse(SRC.read_text())
    mods = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            mods |= {a.name.split(".")[0] for a in node.names}
        elif isinstance(node, ast.ImportFrom):
            assert node.level == 0
            mods.add(node.module.split(".")[0])
    assert mods <= allowed, mods - allowed


@pytest.mark.parametrize("action", ["exec", "headers"])
def test_start_overhead_under_200ms(action):
    """Whole process (interpreter start + imports + keychain read + exec) with a fake backend."""
    command = [sys.executable, "-c", "pass"]
    best = min(_timed(action, command) for _ in range(3))
    assert best < 0.2, best


def _timed(action, command):
    """Seconds from child start (before importing keyring) to the moment of exec/print."""
    code = CHILD.format(tests=str(Path(__file__).parent), command=command, secret=SECRET)
    code = code.replace(
        "sys.exit(launch.main(sys.argv[1:]))",
        "import os\n"
        "def _stop(*a):\n"
        "    sys.stderr.write(str(time.perf_counter() - _t0)); sys.exit(0)\n"
        "os.execvpe = _stop\n"
        "launch.main(sys.argv[1:])\n"
        "sys.stderr.write(str(time.perf_counter() - _t0))\n")
    res = subprocess.run([sys.executable, "-c", code, action, "slack"],
                         check=False, capture_output=True, text=True)
    return float(res.stderr.strip().splitlines()[-1])
