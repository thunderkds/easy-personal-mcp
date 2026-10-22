import json
import os
import sys

import keyring
import pytest

from connectors.fake_backend import MemoryKeyring
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
    canaries.render_verifier_report(repo)
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
    mp.setenv("EASY_PERSONAL_VERIFIER_REPORTS", str(root / "no-verifier-container"))
    # connector_status runs in-process: a memory keyring holding a secret canary and a fake
    # `claude` that knows only "slack"; the owner's keychain and Claude config are never read.
    fake_bin = root / "bin"
    fake_bin.mkdir()
    fake = fake_bin / "claude"
    fake.write_text(f"#!{sys.executable}\nimport sys\nsys.exit(0 if sys.argv[1:] == ['mcp', 'get', 'slack'] else 1)\n")
    fake.chmod(0o755)
    mp.setenv("PATH", f"{fake_bin}:{os.environ['PATH']}")
    old_keyring = keyring.get_keyring()
    memory = MemoryKeyring()
    memory.set_password("easy-personal/slack", "connector", canaries.values["api_key"])
    keyring.set_keyring(memory)
    out = harness.Collected()
    try:
        harness.collect_cli(env, projects, out)  # first call to `ingest` fills the store
        with harness.LogCapture() as logs:
            harness.collect_mcp(harness.build_app(), out, logs)
            harness.collect_api(out, logs, canaries.values["api_key"], root / "api-write")
        harness.collect_mcp_stdio(env, root / "stdio-stderr.log", out)
    finally:
        keyring.set_keyring(old_keyring)
        mp.undo()
    out.outputs.update(harness.store_blobs(home))
    return out
