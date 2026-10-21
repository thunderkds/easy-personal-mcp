import asyncio
import json

import pytest

from easy_personal.surfaces import cli
from easy_personal.surfaces import mcp as mcp_surface


def _cli_json(capsys, argv):
    assert cli.main(argv) == 0
    return json.loads(capsys.readouterr().out)


@pytest.fixture
def ingested(tmp_path, monkeypatch, capsys, fixtures_root):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    monkeypatch.setenv("EASY_PERSONAL_CLAUDE_PROJECTS", str(fixtures_root))
    _cli_json(capsys, ["ingest"])


@pytest.mark.parametrize("by", ["day", "model", "project", "tool"])
def test_mcp_and_cli_breakdown_identical(ingested, capsys, by):
    app = mcp_surface.create_app()
    names = {t.name for t in asyncio.run(app.list_tools())}
    assert {"usage_breakdown", "session_outliers"} <= names
    cli_out = _cli_json(capsys, ["usage", "--by", by])
    _, mcp_out = asyncio.run(app.call_tool("usage_breakdown", {"by": by}))
    assert cli_out == mcp_out
    assert cli_out["total"]["input_tokens"] == 15
    assert cli_out["total"]["api_equivalent_cost"]["price_table_date"]


def test_fixture_model_is_unpriced_and_project_is_folder(ingested, capsys):
    models = _cli_json(capsys, ["usage", "--by", "model"])["groups"]
    assert models[0]["key"] == "claude-fixture-model-1"
    assert models[0]["api_equivalent_cost"]["usd"] is None
    projects = _cli_json(capsys, ["usage", "--by", "project"])["groups"]
    assert [g["key"] for g in projects] == ["demo-app"]


def test_mcp_and_cli_outliers_identical(ingested, capsys):
    app = mcp_surface.create_app()
    cli_out = _cli_json(capsys, ["outliers"])
    _, mcp_out = asyncio.run(app.call_tool("session_outliers", {}))
    assert cli_out == mcp_out
    assert cli_out["sessions"] == []


def test_cli_rejects_unknown_by(ingested, capsys):
    with pytest.raises(SystemExit) as exc:
        cli.main(["usage", "--by", "branch"])
    assert exc.value.code == 2
