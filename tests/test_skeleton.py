import asyncio
import stat
import subprocess
import sys
from importlib.metadata import version

import pytest

from easy_personal import core
from easy_personal.core import config
from easy_personal.surfaces import cli
from easy_personal.surfaces import mcp as mcp_surface


def test_version_flag_prints_package_version(capsys):
    with pytest.raises(SystemExit) as exc:
        cli.main(["--version"])
    assert exc.value.code == 0
    assert version("easy-personal-mcp") in capsys.readouterr().out


def test_cli_and_mcp_return_same_core_value(capsys):
    assert cli.main(["ping"]) == 0
    cli_out = capsys.readouterr().out.strip()

    app = mcp_surface.create_app()
    tools = asyncio.run(app.list_tools())
    assert [t.name for t in tools] == [
        "ping",
        "usage_summary",
        "usage_breakdown",
        "session_outliers",
        "warnings",
        "improvement_plan",
        "kit_summary",
    ]
    _, structured = asyncio.run(app.call_tool("ping", {}))
    assert cli_out == core.ping() == structured["result"]


def test_mcp_stdio_serves_without_stdout_noise():
    proc = subprocess.run(
        [sys.executable, "-m", "easy_personal.surfaces.cli", "mcp"],
        check=False,
        input="",
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert proc.stdout == ""


def test_http_host_0000_refused(capsys):
    with pytest.raises(SystemExit) as exc:
        cli.main(["mcp", "--transport", "http", "--host", "0.0.0.0"])
    assert exc.value.code != 0
    assert "127.0.0.1" in capsys.readouterr().err


def test_http_default_host_is_loopback():
    args = cli.build_parser().parse_args(["mcp", "--transport", "http"])
    assert args.host == "127.0.0.1"
    assert mcp_surface.validate_host(args.host) == "127.0.0.1"


def test_data_dir_created_0700(tmp_path, monkeypatch):
    target = tmp_path / "ep"
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(target))
    assert config.data_dir() == target
    assert stat.S_IMODE(target.stat().st_mode) == 0o700


def test_data_dir_leaves_existing_dir_mode_alone(tmp_path, monkeypatch):
    target = tmp_path / "existing"
    target.mkdir(mode=0o755)
    target.chmod(0o755)
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(target))
    assert config.data_dir() == target
    assert stat.S_IMODE(target.stat().st_mode) == 0o755


def test_mcp_server_reports_package_version():
    app = mcp_surface.create_app()
    options = app._mcp_server.create_initialization_options()
    assert options.server_version == version("easy-personal-mcp")
