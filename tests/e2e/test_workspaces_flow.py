"""HUN-71: the workspace feature end to end, across fake agent accounts (FR-028..FR-036, NFR-002/003/009).

One temp machine per surface: three Claude accounts + one Codex (+ an account to ignore, and
`~/.claude.json`-style files that are not accounts), a fake keychain, a fake `claude` that keeps a
per-CLAUDE_CONFIG_DIR config and logs every call. The same flow runs through the CLI (subprocesses)
and through the dashboard JSON API (Starlette TestClient with the per-launch token); the two end
states must be identical. Nothing outside the temp machine may be touched.

The flow: detect -> confirm / ignore -> ingest -> workspaces -> connectors -> picks -> sync (dry run,
then real) -> move -> alias -> rename -> delete workspace -> adopt legacy -> usage filters -> remove.
"""

import json
import re
from pathlib import Path

import e2e_support as sup
import pytest
from e2e_support import CANARIES, MESSAGES, ApiSurface, CliSurface, World

SINCE, UNTIL = "2026-10-01", "2026-11-01"
DEFAULT, WORK, WIFE, SCRATCH, CODEX = ".claude", ".claude-work", ".claude-wife", ".claude-scratch", ".codex"
HAND = {"type": "stdio", "command": "/usr/bin/some-other-server", "args": ["--serve"]}  # not ours
LAUNCHER = {"type": "stdio", "command": "/opt/x/easy-personal-launch"}
SURFACES = {"cli": CliSurface, "api": ApiSurface}
SLACK = "SLACK_MCP_XOXB_TOKEN"


# --- small helpers -----------------------------------------------------------------------------


def cfg(w: World) -> dict[str, list[str]]:
    """The fake Claude user scope: config dir (basename, `<unset>` = default) -> sorted entry names."""
    state = json.loads(w.claude_state.read_text()) if w.claude_state.exists() else {}
    return {("<unset>" if k == "<unset>" else Path(k).name): sorted(v) for k, v in state.items()}


def expect_cfg(w: World, **names):
    """Assert the per-account Claude config; `default` is the unset-CLAUDE_CONFIG_DIR account."""
    want = {("<unset>" if k == "default" else "." + k.replace("_", "-")): sorted(v) for k, v in names.items()}
    got = {k: v for k, v in cfg(w).items() if v}
    assert got == {k: v for k, v in want.items() if v}, f"claude config differs: {got} != {want}"


def acct_ids(s) -> dict[str, int]:
    return {Path(a["folder"]).name: a["id"] for a in s.accounts()}


def counts(w: World) -> int:
    return len(w.writes())


def entry_args(w: World, config_dir: str, alias: str) -> list[str]:
    return json.loads(w.claude_state.read_text())[config_dir][alias]["args"]


def only_ok(result, text=""):
    assert result.ok, result.text
    if text:
        assert text in result.text, result.text
    return result


def refused(result, text=""):
    assert not result.ok, f"expected a refusal, got ok: {result.text}"
    if text:
        assert text in result.text, result.text


# --- the flow ----------------------------------------------------------------------------------


def run_flow(s, w: World) -> dict:
    """Run the whole flow on surface `s`, asserting each step; return what the surfaces must agree on."""
    out: dict = {}
    ids = acct_ids(s)

    # 1. detect (FR-028): only folders with the right content; the *.json files are not accounts
    rows = {Path(a["folder"]).name: a for a in s.accounts()}
    assert set(rows) == {DEFAULT, WORK, WIFE, SCRATCH, CODEX}
    assert rows[DEFAULT]["status"] == "confirmed" and rows[DEFAULT]["config_dir_env"] is None
    assert rows[DEFAULT]["label"] == "Claude default"
    assert rows[WORK]["status"] == "detected" and rows[WORK]["config_dir_env"] == w.folder(WORK)
    assert rows[CODEX]["provider"] == "codex" and rows[CODEX]["config_dir_env"] is None

    # 2. confirm + name, ignore, Codex is listed only
    only_ok(s.confirm(ids[WORK], "Claude work"))
    only_ok(s.confirm(ids[WIFE], "Claude wife"))
    only_ok(s.confirm(ids[CODEX], "Codex"))
    only_ok(s.ignore(ids[SCRATCH]))
    refused(s.ignore(ids[DEFAULT]))  # the default account cannot be hidden
    refused(s.confirm(ids[WORK], ""))  # a label is required
    refused(s.confirm(ids[WORK], "x" * 61))
    by_status = {Path(a["folder"]).name: a["status"] for a in s.accounts()}
    assert by_status == {DEFAULT: "confirmed", WORK: "confirmed", WIFE: "confirmed",
                         SCRATCH: "ignored", CODEX: "confirmed"}  # fmt: skip

    # 3. ingest every confirmed Claude account (FR-030): events tagged by account, ignored one skipped
    only_ok(s.ingest())
    total = s.usage()
    assert total.data["messages"] == len(MESSAGES), total.data  # D1/D2 copied into wife count once
    assert total.data["input_tokens"] == sup.expected_totals()[1]
    for who, keys in (("Claude default", {"msgD1", "msgD2"}), ("Claude work", {"msgW1", "msgW2", "msgW3"}),
                      ("Claude wife", {"msgF1", "msgD1", "msgD2"})):  # wife's copy shows in its own view
        got = s.usage(account=who).data
        assert got["messages"] == len(keys) and got["input_tokens"] == sup.expected_totals(keys)[1], who
    refused(s.usage(account="Claude scratch"))  # not confirmed -> unknown account, no echo
    by_id = s.usage(account=ids[WORK]).data  # an account is addressable by id as well as label
    assert by_id == s.usage(account="claude WORK").data  # ... and by label, case-insensitive
    out["usage_by_account"] = {n: s.usage(account=n).data for n in ("Claude default", "Claude work", "Claude wife")}

    # 4. workspaces (FR-029, FR-035): repo given as a subfolder resolves to its repo
    acme_repos = [str(w.repos["alpha"] / "pkg"), str(w.repos["beta"])]
    only_ok(s.ws_add("acme", acme_repos, color="#336699", email="me@acme.invalid"))
    only_ok(s.ws_add("personal", [str(w.repos["gamma"])]))
    refused(s.ws_add("acme"))  # name is unique
    refused(s.ws_add("orphan"))  # reserved
    refused(s.ws_add("Bad Name"))
    refused(s.ws_add("tinted", color="blue"))
    acme = s.usage(workspace="acme").data
    assert (acme["messages"], acme["input_tokens"]) == sup.expected_totals(projects={"alpha", "beta"})[:2]
    personal = s.usage(workspace="personal").data
    assert (personal["messages"], personal["input_tokens"]) == (1, 50)
    both = s.usage(workspace="acme", account="Claude work").data
    assert both["messages"] == 3 and both["input_tokens"] == 1200  # the filters AND together
    refused(s.usage(workspace="nope"))
    out["usage_by_workspace"] = {n: s.usage(workspace=n).data for n in ("acme", "personal")}

    # 5. connectors in workspaces (FR-029): secrets only to the keychain; Claude Code untouched
    pre = counts(w)
    only_ok(s.connector_add("slack", "acme", {SLACK: CANARIES["slack_acme"]}), "added to workspace 'acme'")
    only_ok(s.connector_add("slack", "personal", {SLACK: CANARIES["slack_personal"]}))
    only_ok(s.connector_add("gmail", "acme", {"GMAIL_OAUTH_PATH": CANARIES["gmail_path"]}))
    ids_before = w.stored_ids()
    refused(s.connector_add("slack", "acme", {SLACK: CANARIES["slack_acme"]}))  # one per provider per workspace
    refused(s.connector_add("slack", "orphan", {SLACK: CANARIES["slack_acme"]}))
    refused(s.connector_add("slack", "nowhere", {SLACK: CANARIES["slack_acme"]}))
    assert w.stored_ids() == ids_before and len(w.keychain()) == 3
    assert set(ids_before) == {"acme-slack", "personal-slack", "acme-gmail"}
    assert counts(w) == pre == 0, "adding a connector must not call `claude` (sync does)"
    cid = w.stored_ids()
    assert all(f"easy-personal/{c}" in w.keychain() for c in cid.values())

    # 6. accounts pick workspaces (FR-031)
    only_ok(s.use(ids[DEFAULT], ["acme"]))
    only_ok(s.use(ids[WORK], ["acme", "personal"]))
    only_ok(s.use(ids[WIFE], ["personal"]))
    refused(s.use(ids[CODEX], ["acme"]))  # Codex is listed only
    refused(s.use(ids[SCRATCH], ["acme"]))  # ignored accounts pick nothing
    refused(s.use(ids[WIFE], ["orphan"]))
    refused(s.use(ids[WIFE], ["nope"]))
    assert counts(w) == 0
    assert {Path(a["folder"]).name: a["workspaces"] for a in s.accounts()}[WORK] == ["acme", "personal"]

    # 7. sync: dry run shows the plan and writes nothing (FR-033); a hand-made server is never touched
    w.seed_claude(w.folder(WORK), "hand-made", HAND)
    plan = s.sync(yes=False)
    for who, alias, key in (("Claude default", "acme-slack", "acme-slack"), ("Claude default", "acme-gmail", "acme-gmail"),
                            ("Claude work", "acme-slack", "acme-slack"), ("Claude work", "acme-gmail", "acme-gmail"),
                            ("Claude work", "personal-slack", "personal-slack"),
                            ("Claude wife", "personal-slack", "personal-slack")):  # fmt: skip
        assert f"{who}: add {alias} -> exec {cid[key]}" in plan.text, plan.text
    assert counts(w) == 0, "a dry run must not write"
    assert not any("hand-made" in line for line in plan.text.splitlines() if "remove" in line)

    # 8. real sync: one `claude` call per step, CLAUDE_CONFIG_DIR per account (unset for the default)
    only_ok(s.sync())
    writes = w.writes()
    assert len(writes) == 6 and all(argv[:2] == ["mcp", "add-json"] for _, argv in writes), writes
    assert sorted(d for d, _ in writes) == sorted(["<unset>"] * 2 + [w.folder(WORK)] * 3 + [w.folder(WIFE)])
    assert w.folder(DEFAULT) not in {d for d, _ in w.fake.write_records()}  # never CLAUDE_CONFIG_DIR=~/.claude
    expect_cfg(w, default=["acme-slack", "acme-gmail"],
               claude_work=["acme-slack", "acme-gmail", "personal-slack", "hand-made"],
               claude_wife=["personal-slack"])  # fmt: skip
    assert entry_args(w, "<unset>", "acme-slack") == ["exec", cid["acme-slack"]]
    assert entry_args(w, w.folder(WORK), "personal-slack") == ["exec", cid["personal-slack"]]
    assert json.loads(w.claude_state.read_text())[w.folder(WORK)]["hand-made"] == HAND
    assert not {SCRATCH, CODEX} & set(cfg(w)) and w.folder(SCRATCH) not in w.claude_state.read_text()

    # 9. idempotent: the second sync makes no `claude` write
    done = counts(w)
    only_ok(s.sync(), "nothing to change")
    assert counts(w) == done

    # 10. move (FR-032): refused before any write when the target has the provider; else alias follows
    refused(s.move(cid["personal-slack"], "acme"), "already has a slack connector")
    refused(s.move(cid["acme-gmail"], "acme"), "already in this workspace")
    assert counts(w) == done
    only_ok(s.move(cid["acme-gmail"], "personal"))
    expect_cfg(w, default=["acme-slack"],
               claude_work=["acme-slack", "personal-slack", "personal-gmail", "hand-made"],
               claude_wife=["personal-slack", "personal-gmail"])  # fmt: skip
    assert w.stored_ids()["personal-gmail"] == cid["acme-gmail"], "id is fixed, only the alias moved"
    cid["personal-gmail"] = cid.pop("acme-gmail")

    # 11. alias: custom, clash and invalid refused, reset to default
    only_ok(s.alias(cid["acme-slack"], "ac-slack"))
    expect_cfg(w, default=["ac-slack"],
               claude_work=["ac-slack", "personal-slack", "personal-gmail", "hand-made"],
               claude_wife=["personal-slack", "personal-gmail"])  # fmt: skip
    refused(s.alias(cid["acme-slack"], "personal-slack"), "in use")
    refused(s.alias(cid["acme-slack"], "Bad Alias!"))
    only_ok(s.alias(cid["acme-slack"], None))
    assert "acme-slack" in cfg(w)["<unset>"] and "ac-slack" not in cfg(w)["<unset>"]

    # 12. rename a workspace: default aliases follow, picks stay, usage filter follows the id
    refused(s.ws_rename("acme", "orphan"))
    refused(s.ws_rename("acme", "personal"))
    only_ok(s.ws_rename("acme", "acme-corp"))
    expect_cfg(w, default=["acme-corp-slack"],
               claude_work=["acme-corp-slack", "personal-slack", "personal-gmail", "hand-made"],
               claude_wife=["personal-slack", "personal-gmail"])  # fmt: skip
    assert {Path(a["folder"]).name: a["workspaces"] for a in s.accounts()}[DEFAULT] == ["acme-corp"]
    assert s.usage(workspace="acme-corp").data == acme
    refused(s.usage(workspace="acme"))
    cid["acme-corp-slack"] = cid.pop("acme-slack")

    # 13. delete a workspace: connectors go to orphan and leave every account, keychain stays
    keys_before = dict(w.keychain())
    s.ws_delete("personal", yes=False)  # CLI exits 1 on "no", the API answers pending: both change nothing
    assert counts(w) == len(w.writes())
    assert "personal" in {r["name"] for r in s.ws_list()}, "a refused confirmation must keep the workspace"
    only_ok(s.ws_delete("personal"))
    assert "personal" not in {r["name"] for r in s.ws_list()}
    expect_cfg(w, default=["acme-corp-slack"], claude_work=["acme-corp-slack", "hand-made"])
    with w.db() as conn:
        orphaned = conn.execute("SELECT c.id, c.alias FROM connectors c JOIN workspaces w ON w.id = c.workspace_id "
                                "WHERE w.is_orphan = 1 ORDER BY 2").fetchall()  # fmt: skip
        assert orphaned == [(cid["personal-gmail"], "personal-gmail"), (cid["personal-slack"], "personal-slack")]
        assert conn.execute("SELECT COUNT(*) FROM account_workspaces p JOIN workspaces w ON w.id = p.workspace_id "
                            "WHERE w.is_orphan = 1").fetchone()[0] == 0  # orphan is active in no account
    assert w.keychain() == keys_before, "deleting a workspace never touches the keychain"
    refused(s.use(ids[WIFE], ["personal"]))
    refused(s.usage(workspace="personal"))

    # 14. adopt pre-workspace connectors (FR-034): keychain stays, only OUR default-account entry goes
    w.seed_keychain("easy-personal/discord", {"command": ["npx", "x"], "env": {"DISCORD_TOKEN": CANARIES["legacy"]}})
    w.seed_keychain("easy-personal/sendgrid", {"command": ["npx", "y"], "env": {"SENDGRID_API_KEY": CANARIES["legacy"]}})
    w.seed_claude("<unset>", "discord", LAUNCHER | {"args": ["exec", "discord"]})
    w.seed_claude("<unset>", "sendgrid", HAND)  # same name as a legacy connector, but not ours
    legacy_keys = dict(w.keychain())
    s.adopt(yes=False)
    assert "discord" not in w.stored_ids(), "a refused confirmation adopts nothing"
    only_ok(s.adopt())
    assert w.stored_ids()["discord"] == "discord" and w.stored_ids()["sendgrid"] == "sendgrid"
    expect_cfg(w, default=["acme-corp-slack", "sendgrid"], claude_work=["acme-corp-slack", "hand-made"])
    assert json.loads(w.claude_state.read_text())["<unset>"]["sendgrid"] == HAND, "a hand-made server is never removed"
    assert w.keychain() == legacy_keys
    only_ok(s.adopt(), "nothing to adopt")
    only_ok(s.move("discord", "acme-corp"))
    only_ok(s.move("sendgrid", "acme-corp"))
    expect_cfg(w, default=["acme-corp-slack", "acme-corp-discord", "acme-corp-sendgrid", "sendgrid"],
               claude_work=["acme-corp-slack", "acme-corp-discord", "acme-corp-sendgrid", "hand-made"])  # fmt: skip
    assert entry_args(w, "<unset>", "acme-corp-discord") == ["exec", "discord"]  # launcher keyed by the old id

    # 15. remove one connector: every account drops it, then keychain item and row go
    only_ok(s.remove(cid["acme-corp-slack"]))
    expect_cfg(w, default=["acme-corp-discord", "acme-corp-sendgrid", "sendgrid"],
               claude_work=["acme-corp-discord", "acme-corp-sendgrid", "hand-made"])  # fmt: skip
    assert f"easy-personal/{cid['acme-corp-slack']}" not in w.keychain()
    assert "acme-corp-slack" not in w.stored_ids()
    assert f"easy-personal/{cid['personal-slack']}" in w.keychain()  # the others stay

    # 16. nothing left to do, and final usage the surfaces must agree on
    only_ok(s.sync(), "nothing to change")
    out["usage_final"] = {
        "all": s.usage().data, "acme-corp": s.usage(workspace="acme-corp").data,
        "work": s.usage(account="Claude work").data, "wife+acme": s.usage(workspace="acme-corp", account="Claude wife").data,
        "by_project": s.usage(by="project").data, "by_day_acme": s.usage(by="day", workspace="acme-corp").data,
    }  # fmt: skip
    assert out["usage_final"]["wife+acme"]["messages"] == 2  # wife's copy of D1/D2 lives in repo alpha
    return out


# --- run it once per surface, share the results -----------------------------------------------


_RUNS: dict[str, object] = {}


def flow(kind: str, tmp_path_factory) -> dict:
    """Run (once) the flow on a surface inside the no-touch guard; cache result or error."""
    if kind not in _RUNS:
        w = World(tmp_path_factory.mktemp(f"world-{kind}"))
        run = {"world": w, "error": None}
        try:
            with sup.no_touch_guard() as guard, w.active():
                from leak.harness import LogCapture

                with LogCapture() as logs:
                    s = SURFACES[kind](w)
                    run["out"] = run_flow(s, w)
                    run["state"] = sup.snapshot(w)
                    run["checks"] = sup.sqlite_checks(w)
                    # the whole flow again: nothing changes, `claude` is not written to
                    writes, before = counts(w), sup.snapshot(w)
                    s.ingest()
                    s.adopt()
                    s.sync()
                    run["repeat"] = (writes, counts(w), before, sup.snapshot(w))
                    w.record("log records", logs.text())
            run["guard"] = guard
        except BaseException as exc:  # noqa: BLE001 - re-raised for every test that needs this run
            run["error"] = exc
        _RUNS[kind] = run
    run = _RUNS[kind]
    if run["error"] is not None:
        raise run["error"]
    return run


@pytest.fixture(scope="module")
def cli_run(tmp_path_factory):
    return flow("cli", tmp_path_factory)


@pytest.fixture(scope="module")
def api_run(tmp_path_factory):
    return flow("api", tmp_path_factory)


@pytest.fixture(params=["cli", "api"])
def run(request, tmp_path_factory):
    return flow(request.param, tmp_path_factory)


# AC1: the full flow passes on each surface, with an identical end state
def test_full_flow_passes_on_both_surfaces_with_identical_end_state(cli_run, api_run):
    a, b = cli_run["state"], api_run["state"]
    for key in a:
        assert a[key] == b[key], f"{key}: cli={a[key]} api={b[key]}"
    assert a == b
    assert cli_run["out"] == api_run["out"]  # every usage view agrees
    assert cli_run["state"]["claude_writes"], "the flow wrote Claude config"


# AC2: each `claude` call carries the expected CLAUDE_CONFIG_DIR (unset for the default account)
def test_claude_calls_carry_the_expected_config_dir(run):
    w: World = run["world"]
    records = w.fake.records()
    assert records and all(r["config_dir"] in ("<unset>", w.folder(WORK), w.folder(WIFE)) for r in records)
    seen = {r["config_dir"] for r in records}
    assert seen == {"<unset>", w.folder(WORK), w.folder(WIFE)}
    for r in records:  # every call is the official CLI, user scope, never a project or local scope
        assert r["argv"][0] == "mcp" and r["argv"][1] in ("get", "add-json", "remove")
        if r["argv"][1] != "get":
            assert r["argv"][-2:] == ["--scope", "user"]
    assert not {SCRATCH, CODEX} & {Path(r["config_dir"]).name for r in records}


# AC3: planted secrets never appear in the store, outputs, plan lines, the claude argv log or logs
def test_planted_secrets_never_leave_the_keychain(run):
    w: World = run["world"]
    blobs = dict(sup.store_blobs(w)) | {f"{i} {label}": blob for i, (label, blob) in enumerate(w.outputs)}
    blobs["claude argv log"] = w.claude_log.read_bytes()
    blobs["claude state"] = w.claude_state.read_bytes()
    assert len(blobs) > 60  # the scan really covered the flow's outputs
    assert sup.leaks(blobs, CANARIES) == []
    # positive controls: the scan can see them, and the keychain does hold the typed secrets
    keychain = json.dumps(w.keychain())
    assert all(CANARIES[k] in keychain for k in ("slack_personal", "gmail_path", "legacy"))
    assert CANARIES["slack_acme"] not in keychain, "removing a connector deletes its keychain item"
    assert sup.leaks({"keychain": keychain}, CANARIES), "the scan must be able to find a planted secret"
    # transcript text and tool input/output never reached the store either (the fixtures hold them)
    assert all(CANARIES[k] in "".join(p.read_text() for p in w.agents.rglob("*.jsonl"))
               for k in ("prompt", "assistant", "tool_output", "url_token", "bash_secret"))  # fmt: skip


# AC4: nothing outside the temp machine is touched
def test_nothing_outside_the_temp_machine_is_touched(run):
    w: World = run["world"]
    guard = run["guard"]
    assert guard.hits == [], f"the process opened real agent/store paths: {guard.hits}"
    assert guard.changed == [], f"real ~/.claude* / ~/.codex* / ~/.easy-personal changed: {guard.changed}"
    # read-only sources: the product wrote nothing into any agent folder (transcripts, settings, *.json)
    assert sup.tree_snapshot(w.agents) == w.agent_snapshot
    # and its own data lives only in the temp store dir
    assert (w.data / "store.sqlite").is_file()
    assert not (w.fakehome / ".easy-personal").exists() and not list(w.fakehome.iterdir())


def test_guard_detects_a_touch(tmp_path):
    """Negative control for AC4: the audit hook and the mtime snapshot do see a real-path event."""
    with sup.no_touch_guard(extra_prefixes=[str(tmp_path)]) as guard:
        (tmp_path / "x").write_text("1")
    assert any(str(tmp_path / "x") in h for h in guard.hits)
    sup.REAL_HOME.joinpath(".claude-no-such-dir-for-guard-test")  # a path object only: never created
    before = sup.tree_snapshot(tmp_path)
    (tmp_path / "x").write_text("22")
    assert sup.tree_snapshot(tmp_path) != before


# AC5: store version latest, foreign keys and integrity clean
def test_store_is_at_the_latest_version_and_clean(run):
    checks = run["checks"]
    assert checks["user_version"] == sup.latest_version()
    assert checks["foreign_key_check"] == []
    assert checks["integrity_check"] == [("ok",)]


# edge: run the whole flow twice -> the second sync makes no `claude` write
def test_second_run_is_idempotent(run):
    writes_before, writes_after, state_before, state_after = run["repeat"]
    assert writes_after == writes_before, "a repeated sync/adopt wrote to Claude Code"
    assert state_after == state_before


# edge: a hand-made server survives every operation of the flow
def test_hand_made_servers_are_never_touched(run):
    w: World = run["world"]
    state = json.loads(w.claude_state.read_text())
    assert state[w.folder(WORK)]["hand-made"] == HAND
    assert state["<unset>"]["sendgrid"] == HAND
    removed = {argv[2] for _, argv in w.writes() if argv[1] == "remove"}
    assert not removed & {"hand-made", "sendgrid"}


# --- a failure between plan steps: the next sync converges ------------------------------------


@pytest.mark.parametrize("kind", ["cli", "api"])
def test_claude_failure_between_steps_then_resync_converges(kind, tmp_path):
    w = World(tmp_path / "w")
    with sup.no_touch_guard() as guard, w.active():
        s = SURFACES[kind](w)
        ids = acct_ids(s)
        for name, label in ((WORK, "Claude work"), (WIFE, "Claude wife")):
            only_ok(s.confirm(ids[name], label))
        for ws in ("acme", "personal"):
            only_ok(s.ws_add(ws))
        only_ok(s.connector_add("slack", "acme", {SLACK: CANARIES["slack_acme"]}))
        only_ok(s.connector_add("slack", "personal", {SLACK: CANARIES["slack_personal"]}))
        only_ok(s.use(ids[DEFAULT], ["acme"]))
        only_ok(s.use(ids[WORK], ["acme", "personal"]))
        only_ok(s.use(ids[WIFE], ["personal"]))
        import os

        os.environ["FAKE_CLAUDE_FAIL"] = "add-json personal-slack"  # plan order is by account id: default ok, then wife fails
        try:
            result = s.sync()
        finally:
            del os.environ["FAKE_CLAUDE_FAIL"]
        assert not result.ok and "failed" in result.text and "connector sync" in result.text
        partial = cfg(w)
        assert partial.get("<unset>") == ["acme-slack"], partial
        assert not partial.get(WIFE) and not partial.get(WORK), "nothing after the failed step ran"
        with w.db() as conn:
            assert conn.execute("SELECT COUNT(*) FROM account_registrations").fetchone()[0] == 1
        only_ok(s.sync())  # the plan resumes: only what is missing is written
        expect_cfg(w, default=["acme-slack"], claude_work=["acme-slack", "personal-slack"],
                   claude_wife=["personal-slack"])  # fmt: skip
        resumed = [argv[2] for _, argv in w.writes()]
        assert resumed.count("acme-slack") == 2, "steps that succeeded are not repeated"
        only_ok(s.sync(), "nothing to change")
    assert guard.hits == [] and guard.changed == []


# --- the API stays behind its token -----------------------------------------------------------


def test_api_refuses_writes_without_the_token(tmp_path):
    from easy_personal.surfaces import api

    w = World(tmp_path / "w")
    with w.active():
        from starlette.testclient import TestClient

        client = TestClient(api.create_app(sup.TOKEN, sup.PORT), base_url=f"http://127.0.0.1:{sup.PORT}")
        body = json.dumps({"name": "sneaky"})
        for headers in ({}, {api.TOKEN_HEADER: "wrong"}):
            res = client.post("/api/workspaces", content=body, headers={**headers, "content-type": "application/json"})
            assert res.status_code == 403
        res = client.post("/api/workspaces", content=body, headers={api.TOKEN_HEADER: sup.TOKEN})
        assert res.status_code == 415  # JSON content type is required for a write
        wrong_host = TestClient(api.create_app(sup.TOKEN, sup.PORT), base_url="http://evil.invalid")
        assert wrong_host.get("/api/workspaces", headers={api.TOKEN_HEADER: sup.TOKEN}).status_code == 403
    assert not (w.data / "store.sqlite").exists() or "sneaky" not in json.dumps(
        [r for r in sup.snapshot(w)["workspaces"]])


# --- CLI-only: doctor reports no drift after the flow -----------------------------------------


@pytest.mark.xfail(strict=False, reason="DEFECT P2: `doctor` still runs the pre-workspace catalog-id check, so an "
                   "adopted legacy connector (keychain item, assigned to a workspace) reports FAIL and exit 1")
def test_doctor_is_clean_after_the_flow(cli_run):
    w: World = cli_run["world"]
    with w.active():
        rc, out, err = CliSurface(w).run("doctor")
    assert "FAIL" not in out + err, out + err
    assert rc == 0, out + err
    assert not sup.leaks({"doctor": out + err}, CANARIES)


# --- carry-over P3 findings (reported, not fixed) ------------------------------------------------


@pytest.mark.xfail(strict=False, reason="HUN-65 P3: `connector add` prompt at stdin EOF raises a traceback "
                   "(EOFError from getpass/input) instead of a one-line message and a non-zero exit")
def test_connector_add_prompt_at_stdin_eof_is_a_clean_error(tmp_path):
    w = World(tmp_path / "w")
    with w.active():
        s = CliSurface(w)
        only_ok(s.ws_add("acme"))
        for provider, env_stdin in (("slack", ""), ("gmail", "")):  # secret prompt, then plain-field prompt
            rc, _out, err = s.run("connector", "add", provider, "--workspace", "acme", "--yes", stdin=env_stdin)
            assert rc != 0
            assert "Traceback" not in err, err
        assert w.keychain() == {} and w.stored_ids() == {}, "nothing is written when the prompt is cut off"


def test_connector_add_prompt_at_stdin_eof_writes_nothing(tmp_path):
    """The part that must hold today: a cut-off prompt exits non-zero and leaves no keychain item or row."""
    w = World(tmp_path / "w")
    with w.active():
        s = CliSurface(w)
        only_ok(s.ws_add("acme"))
        rc, out, err = s.run("connector", "add", "slack", "--workspace", "acme", "--yes", stdin="")
        assert rc != 0
        assert w.keychain() == {} and w.stored_ids() == {}
        assert not sup.leaks({"out": out + err}, CANARIES)


@pytest.mark.xfail(strict=False, reason="HUN-63 P3: tool_signals is keyed by source path, so a transcript "
                   "copied into two accounts counts its WebFetch/Bash signals twice (events dedupe on message id)")
def test_copied_transcript_counts_tool_signals_once(tmp_path):
    w = World(tmp_path / "w")
    with w.active():
        s = CliSurface(w)
        ids = acct_ids(s)
        only_ok(s.confirm(ids[WIFE], "Claude wife"))
        only_ok(s.ingest())
        with w.db() as conn:
            signals = dict(conn.execute("SELECT value, COUNT(*) FROM tool_signals WHERE kind = 'domain' GROUP BY 1"))
            events = conn.execute("SELECT COUNT(DISTINCT message_id) FROM events WHERE message_id = 'msgD1'").fetchone()[0]
        assert events == 1
        assert signals == {"slack.com": 1}, f"the same WebFetch call is counted per copy: {signals}"


def test_copied_transcript_counts_usage_once(tmp_path):
    """Usage dedupes on message id across accounts (edge case), even though signals do not."""
    w = World(tmp_path / "w")
    with w.active():
        s = CliSurface(w)
        ids = acct_ids(s)
        only_ok(s.confirm(ids[WIFE], "Claude wife"))
        only_ok(s.ingest())
        assert s.usage().data["messages"] == len({m for m in MESSAGES if MESSAGES[m][0] != WORK})


def test_account_id_in_api_matches_cli(cli_run, api_run):
    """The two worlds detect the same folders in the same order, so ids are the same on both surfaces."""
    assert re.findall(r"\bc[0-9a-f]{8}\b", json.dumps(cli_run["state"])) == []  # state is id-normalised
    assert [a[0] for a in cli_run["state"]["accounts"]] == [a[0] for a in api_run["state"]["accounts"]]


@pytest.mark.xfail(strict=False, reason="DEFECT P2: CLI `connector remove <unknown name> --yes` falls back to the "
                   "pre-workspace path and runs `claude mcp remove <name> --scope user` in the default account, "
                   "deleting a hand-made server (the API answers 404 for the same call; sync never does this)")
def test_cli_remove_of_an_unknown_name_leaves_a_hand_made_server(tmp_path):
    w = World(tmp_path / "w")
    with w.active():
        w.seed_claude("<unset>", "my-own-server", HAND)
        s = CliSurface(w)
        s.run("connector", "remove", "my-own-server", "--yes")
        assert "my-own-server" in json.loads(w.claude_state.read_text())["<unset>"]


def test_api_remove_of_an_unknown_name_is_404_and_leaves_a_hand_made_server(tmp_path):
    w = World(tmp_path / "w")
    with w.active():
        w.seed_claude("<unset>", "my-own-server", HAND)
        s = ApiSurface(w)
        status, _ = s.call("DELETE", "/api/connectors/my-own-server", {"confirm": True})
        assert status == 404
        assert "my-own-server" in json.loads(w.claude_state.read_text())["<unset>"]
        assert w.writes() == []
