"""README drift test: documented commands, MCP tools and install steps match the code."""

import argparse
import asyncio
import re
import shlex
from pathlib import Path

import pytest

from easy_personal import launch
from easy_personal.core import config
from easy_personal.sources import context
from easy_personal.surfaces import cli, mcp

ROOT = Path(__file__).resolve().parent.parent
README = (ROOT / "README.md").read_text(encoding="utf-8")
RUNBOOK = (ROOT / "RUNBOOK.md").read_text(encoding="utf-8")

SECTIONS = [
    "What it reads",
    "Install",
    "Quick start",
    "Schedule",
    "Connectors",
    "Configuration",
    "Upgrade and rollback",
    "Development",
    "Easy Kit",
]


def code_blocks(text: str) -> list[str]:
    return re.findall(r"```[a-z]*\n(.*?)```", text, flags=re.DOTALL)


def command_lines(prefix: str) -> list[list[str]]:
    found = []
    for block in code_blocks(README):
        for line in block.splitlines():
            if line.startswith(prefix + " "):
                found.append(shlex.split(line, comments=True))
    return found


def subparsers_of(parser: argparse.ArgumentParser):
    for action in parser._actions:
        if isinstance(action, argparse._SubParsersAction):
            return action
    return None


def check_cli_line(tokens: list[str]) -> None:
    parser = cli.build_parser()
    for token in tokens[1:]:
        subs = subparsers_of(parser)
        if token.startswith("-"):
            flag = token.split("=")[0]
            assert any(flag in a.option_strings for a in parser._actions), f"{flag} not in parser"
        elif subs is not None:
            assert token in subs.choices, f"{token!r} is not a subcommand of {parser.prog}"
            parser = subs.choices[token]
        else:
            break  # positional or flag value


def test_sections_exist_in_order():
    headings = re.findall(r"^#{1,3} (.+)$", README, flags=re.MULTILINE)
    positions = [headings.index(name) for name in SECTIONS]
    assert positions == sorted(positions)


def test_status_names_v0_1_0_not_planning():
    assert "Status: planning" not in README
    assert "v0.1.0" in README
    assert README.split("\n\n")[1].strip(), "first paragraph after the title is the pitch"


def test_documented_easy_personal_commands_exist():
    lines = command_lines("easy-personal")
    assert len(lines) >= 12
    for tokens in lines:
        check_cli_line(tokens)


def test_every_top_level_subcommand_is_documented():
    names = set(subparsers_of(cli.build_parser()).choices)
    used = {t[1] for t in command_lines("easy-personal") if len(t) > 1}
    assert names - used - {"exec", "headers"} == set(), "documented command missing"


def test_documented_launcher_commands_exist(capsys):
    assert launch.main([]) != 0
    usage = capsys.readouterr().err
    valid = set(re.search(r"(\w+(?:\|\w+)+)", usage).group(1).split("|"))
    lines = command_lines("easy-personal-launch")
    assert lines
    for tokens in lines:
        assert tokens[1] in valid


def test_mcp_tools_listed_match_server():
    registered = {t.name for t in asyncio.run(mcp.create_app().list_tools())}
    section = README.split("MCP tools", 1)[1].split("\n## ", 1)[0].split("\n### ", 1)[0]
    listed = set(re.findall(r"`([a-z_]+)`", section))
    assert listed == registered


def test_install_and_upgrade_match_runbook():
    for command in (
        "uv tool install --reinstall --no-cache .",
        "npm --prefix web ci && npm --prefix web run build",
    ):
        assert command in RUNBOOK
        assert command in README
    build = README.index("npm --prefix web run build")
    assert build < README.index("uv tool install --reinstall --no-cache .")
    assert "RUNBOOK.md" in README


def test_env_vars_and_config_keys_exist_in_code():
    for var in (
        config.HOME_ENV,
        config.CLAUDE_PROJECTS_ENV,
        config.UI_REPORTS_ENV,
        config.VERIFIER_REPORTS_ENV,
        context.URL_ENV,
    ):
        assert f"`{var}`" in README
    for key in ("reports_dir", "thresholds"):
        assert key in README


def test_no_host_paths_emails_or_tokens():
    assert not re.search(r"/home/|/Users/|/tmp/", README)
    assert not re.search(r"[\w.+-]+@[\w-]+\.[\w.]+", README)
    assert not re.search(r"xox[bp]-[\w-]{6,}|sk-[\w-]{12,}|ghp_\w{10,}|AKIA\w{12,}", README)


def test_cost_is_api_equivalent():
    assert "API-equivalent" in README


def test_easy_kit_links_kept():
    for repo in ("personal-agentic-claude", "easy-ui-mcp", "easy-verifier-mcp", "easy-context-mcp"):
        assert f"https://github.com/thunderkds/{repo}" in README


@pytest.mark.parametrize(
    "tokens", [["easy-personal", "nope"], ["easy-personal", "connector", "nope"]]
)
def test_checker_rejects_unknown_command(tokens):
    with pytest.raises(AssertionError):
        check_cli_line(tokens)
