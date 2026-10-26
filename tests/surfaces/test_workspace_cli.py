"""CLI `workspace add|list|edit|delete` and `--workspace` on usage/outliers (HUN-64)."""

import json

import pytest

from easy_personal.core import config, store
from easy_personal.surfaces import cli
from tests_helpers import add_event


@pytest.fixture(autouse=True)
def home(tmp_path, monkeypatch):
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path / "home"))
    monkeypatch.setenv("HOME", str(tmp_path / "userhome"))
    return tmp_path


def run(capsys, *argv, code=0):
    rc = cli.main(list(argv))
    out = capsys.readouterr()
    assert rc == code, out
    return out


def run_json(capsys, *argv):
    return json.loads(run(capsys, *argv).out)


def test_add_list_edit_delete_roundtrip(capsys):
    ws = run_json(capsys, "workspace", "add", "acme", "--color", "#112233", "--icon", "A",
                  "--description", "Acme work", "--email", "me@acme.test",
                  "--repo", "/work/a", "--repo", "/work/b/")  # fmt: skip
    assert ws["name"] == "acme" and ws["repos"] == ["/work/a", "/work/b"]
    listing = run_json(capsys, "workspace", "list")
    assert [w["name"] for w in listing["workspaces"]] == ["orphan", "acme"]

    out = run_json(capsys, "workspace", "edit", "acme", "--rename", "company",
                   "--add-repo", "/work/c", "--remove-repo", "/work/a", "--description", "")  # fmt: skip
    assert out["id"] == ws["id"] and out["name"] == "company"
    assert out["repos"] == ["/work/b", "/work/c"] and out["description"] is None

    run(capsys, "workspace", "delete", "company", "--yes")
    assert [w["name"] for w in run_json(capsys, "workspace", "list")["workspaces"]] == ["orphan"]


def test_delete_without_yes_asks_and_no_keeps_it(capsys, monkeypatch):
    run(capsys, "workspace", "add", "acme")
    monkeypatch.setattr("builtins.input", lambda _prompt="": "n")
    run(capsys, "workspace", "delete", "acme")
    assert len(run_json(capsys, "workspace", "list")["workspaces"]) == 2
    monkeypatch.setattr("builtins.input", lambda _prompt="": "y")
    run(capsys, "workspace", "delete", "acme")
    assert len(run_json(capsys, "workspace", "list")["workspaces"]) == 1


def test_orphan_is_refused_everywhere(capsys):
    run(capsys, "workspace", "add", "orphan", code=2)
    run(capsys, "workspace", "edit", "orphan", "--rename", "x", code=2)
    run(capsys, "workspace", "edit", "orphan", "--description", "x", code=2)
    run(capsys, "workspace", "delete", "orphan", "--yes", code=2)
    run(capsys, "workspace", "edit", "x", "--rename", "orphan", code=2)
    assert [w["name"] for w in run_json(capsys, "workspace", "list")["workspaces"]] == ["orphan"]


def test_errors_exit_2_without_echoing_input(capsys):
    run(capsys, "workspace", "add", "acme")
    out = run(capsys, "workspace", "add", "acme", code=2)
    assert "exists" in out.err and out.out == ""
    out = run(capsys, "workspace", "add", "BAD-SECRET-NAME", code=2)
    assert "BAD-SECRET-NAME" not in out.err + out.out
    run(capsys, "workspace", "edit", "nope", "--description", "x", code=2)
    run(capsys, "workspace", "edit", "acme", "--rename", "orphan", code=2)
    run(capsys, "workspace", "add", "other", "--repo", "relative", code=2)


def test_rename_to_existing_name_changes_nothing(capsys):
    run(capsys, "workspace", "add", "a", "--repo", "/x")
    run(capsys, "workspace", "add", "b")
    run(capsys, "workspace", "edit", "a", "--rename", "b", "--add-repo", "/y", code=2)
    a = next(w for w in run_json(capsys, "workspace", "list")["workspaces"] if w["name"] == "a")
    assert a["repos"] == ["/x"]


def test_usage_workspace_counts_repos_and_subfolders_only(capsys, tmp_path):
    work = tmp_path / "userhome" / "work"
    with store.opened(config.store_path()) as conn:
        for n, project in enumerate([f"{work}/a", f"{work}/a/pkg", f"{work}/b", f"{work}/bc", "/other"]):
            add_event(conn, n, project)
        conn.commit()
    run(capsys, "workspace", "add", "company", "--repo", "~/work/a", "--repo", "~/work/b")
    run(capsys, "workspace", "add", "empty")
    assert run_json(capsys, "usage")["messages"] == 5
    assert run_json(capsys, "usage", "--workspace", "company")["messages"] == 3
    assert run_json(capsys, "usage", "--workspace", "empty")["messages"] == 0
    by = run_json(capsys, "usage", "--workspace", "company", "--by", "project")
    assert by["total"]["messages"] == 3 and {g["key"] for g in by["groups"]} == {"a", "pkg", "b"}
    assert run_json(capsys, "outliers", "--workspace", "company")["sessions"] == []
    run(capsys, "usage", "--workspace", "nope", code=2)
