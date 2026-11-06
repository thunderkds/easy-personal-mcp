"""`GET /api/kit/workspaces` never shows a verifier reason code or scope, a workspace description or email,
an easy-ui `target`, or a full repo path (HUN-238, NFR-002, FR-087, FR-088).
"""

import json

from starlette.testclient import TestClient

from easy_personal.core import config, owners, store, workspaces
from easy_personal.sources import ui, verifier
from easy_personal.surfaces import api
from leak import harness
from leak.canaries import Canaries
from tests_helpers import add_event

TOKEN, PORT = "leak-238", 5539
URL = "/api/kit/workspaces"


def _repo_with_report(path, canary, value=None):
    """A repo whose newest verifier report carries label-valid canaries in `scope` and `reason_code`."""
    reports = path / "reports"
    reports.mkdir(parents=True)
    abstained = value is None
    report = {
        "schema_version": 1, "scope": canary, "generated_at": "2026-02-01T09:00:00Z",
        "overall": {"value": value, "abstained": abstained, "reason_code": canary if abstained else None},
        "dimensions": [{"dimension": "api", "rating": 0, "abstained": True, "reason_code": canary}],
        "evidence": canary,
    }  # fmt: skip
    (reports / "evidence-report-project-20260201T090000-000000Z.json").write_text(json.dumps(report))


def test_the_rollup_response_and_logs_hold_no_planted_text_and_no_full_path(tmp_path, monkeypatch):
    canaries = Canaries.new()
    root = tmp_path.resolve()
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(root / "home"))
    monkeypatch.setenv("EASY_PERSONAL_VERIFIER_REPORTS", str(root / "no-container"))
    owners.clear_cache()
    label_canary = canaries.values["claude_json_env_secret"]  # a value the label check lets through
    scored, abstained = root / "deep" / "tree" / "scored-repo", root / "deep" / "tree" / "abstained-repo"
    scored.mkdir(parents=True)
    abstained.mkdir(parents=True)
    _repo_with_report(scored, label_canary, 55.0)
    _repo_with_report(abstained, label_canary)
    ui_dir = canaries.render_ui_reports(root / "ui-reports")
    with store.opened(config.store_path()) as conn:
        for n, repo in enumerate((scored, abstained)):
            add_event(conn, n, str(repo), ts="2026-02-10T12:00:00.000000Z")
            conn.execute("UPDATE events SET tool_names = 'mcp__easy-ui__open,Read' WHERE byte_offset = ?", (n,))
        conn.commit()
        assert verifier.ingest(conn)["projects"] == 2
        assert ui.ingest(conn, ui_dir)["sessions"] == 2
        workspaces.add(
            conn, "acme", description=canaries.values["api_key"], email=canaries.values["email"],
            repos=[str(scored), str(abstained)],
        )  # fmt: skip
        stored = " ".join(str(r) for r in conn.execute("SELECT * FROM verifier_scores"))
    assert label_canary in stored  # the planted reason code and scope really are in the store: the scan below is not vacuous
    client = TestClient(api.create_app(TOKEN, PORT), base_url=f"http://127.0.0.1:{PORT}")
    bodies = []
    with harness.LogCapture() as logs:
        for query in ("", "?since=2026-01-01", "?until=2027-01-01", "?account=nope"):
            res = client.get(URL + query, headers={api.TOKEN_HEADER: TOKEN})
            bodies.append(f"{res.status_code} {dict(res.headers)} {res.text}")
            if not query:
                first = res.json()
        log_text = logs.text()
    (acme,) = first["workspaces"]
    assert acme["verifier"]["repos_scored"] == 1 and acme["verifier"]["repos_total"] == 2  # the response really carries data
    assert (acme["ui"]["sessions"], acme["ui"]["calls"]) == (2, 2)  # one session and one call per repo
    assert first["ui_checks"]["sessions"] == 2
    for blob in (*bodies, log_text):
        canaries.assert_clean(URL, blob)
        assert label_canary not in blob
        assert str(root) not in blob and "deep/tree" not in blob  # folder names only, never a full path
    assert "scored-repo" in bodies[0]
    owners.clear_cache()
