import json

import pytest

from leak import harness
from leak.canaries import Canaries


@pytest.fixture(scope="module")
def canaries() -> Canaries:
    return Canaries.new()


@pytest.fixture(scope="module")
def leak_run(tmp_path_factory, canaries):
    """Planted transcripts -> real ingest -> every surface called. Returns what each emitted."""
    root = tmp_path_factory.mktemp("leak")
    projects = canaries.render_fixture(root / "projects")
    repo = canaries.render_kit_trace(root / "repo")
    (projects / "-repo").mkdir()
    (projects / "-repo" / "session.jsonl").write_text(
        json.dumps({"type": "user", "version": "2.1.295", "cwd": str(repo),
                    "sessionId": "00000000-0000-4000-8000-0000000000bb",
                    "timestamp": "2026-02-01T09:00:00.000Z"}) + "\n"
    )  # fmt: skip
    ui_reports = canaries.render_ui_reports(root / "ui-reports")
    home = root / "home"
    env = harness.child_env(home, projects, ui_reports)
    mp = pytest.MonkeyPatch()
    mp.setenv("EASY_PERSONAL_HOME", str(home))
    mp.setenv("EASY_PERSONAL_CLAUDE_PROJECTS", str(projects))
    mp.setenv("EASY_PERSONAL_UI_REPORTS", str(ui_reports))
    out = harness.Collected()
    try:
        harness.collect_cli(env, projects, out)  # first call to `ingest` fills the store
        with harness.LogCapture() as logs:
            harness.collect_mcp(harness.build_app(), out, logs)
            harness.collect_api(out, logs)
        harness.collect_mcp_stdio(env, root / "stdio-stderr.log", out)
    finally:
        mp.undo()
    out.outputs.update(harness.store_blobs(home))
    return out
