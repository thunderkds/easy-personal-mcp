import json
import os
import sys

import keyring
import pytest

from connectors.fake_backend import MemoryKeyring
from fakes import jobs_fakes
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
    agent_root = root / "agent-root"  # a fake home: the default Claude, a second Claude and a Codex account folder
    (agent_root / ".claude" / "projects").mkdir(parents=True)  # the default account (id 1), confirmed by detect
    (agent_root / ".claude-work" / "projects").mkdir(parents=True)
    (agent_root / ".codex").mkdir()
    (agent_root / ".codex" / "config.toml").write_text("")
    canaries.render_memory_notes(repo, agent_root / ".claude-work")  # HUN-164: bodies hold a canary
    canaries.render_agent_files(agent_root)  # HUN-115: `.claude.json` and limits file with unread canaries
    env = harness.child_env(home, projects, ui_reports)
    mp = pytest.MonkeyPatch()
    mp.setenv("EASY_PERSONAL_HOME", str(home))
    mp.setenv("EASY_PERSONAL_CLAUDE_PROJECTS", str(projects))
    mp.setenv("EASY_PERSONAL_AGENT_ROOT", str(agent_root))
    mp.setenv("EASY_PERSONAL_UI_REPORTS", str(ui_reports))
    mp.setenv("EASY_PERSONAL_VERIFIER_REPORTS", str(root / "no-verifier-container"))
    # connector_status runs in-process: a memory keyring holding a secret canary and a fake
    # `claude` that knows only "slack"; the owner's keychain and Claude config are never read.
    fake_bin = root / "bin"
    fake_bin.mkdir()
    fake = fake_bin / "claude"
    fake.write_text(f"#!{sys.executable}\nimport sys\nsys.exit(0 if sys.argv[1:] == ['mcp', 'get', 'slack'] else 1)\n")
    fake.chmod(0o755)
    # HUN-163: fake `systemctl` / `crontab` (the real ones never run); the secret canary sits in their arguments
    key = canaries.values["api_key"]
    jobs_fakes.install(
        fake_bin,
        timers=[{"unit": "easy-personal-ingest.timer", "activates": "easy-personal-ingest.service", "next": 1_793_444_400_000_000, "last": 1_793_433_606_000_000},
                {"unit": "tidy.timer", "activates": "tidy.service", "next": 1_793_444_400_000_000, "last": 0}],
        props={"easy-personal-ingest.timer": {"TimersCalendar": "{ OnCalendar=daily ; next_elapse=n/a }"},
               "easy-personal-ingest.service": {"ExecStart": jobs_fakes.exec_start(f"/usr/bin/easy-personal ingest --token {key}"), "ExecMainStatus": "0"},
               "tidy.service": {"ExecStart": jobs_fakes.exec_start(f"/usr/bin/rm -rf /tmp/{key}"), "ExecMainStatus": "0"}},
        crontab=f'0 7 * * 1-5 claude -p "{key}"\n5 4 * * * /usr/bin/backup --key {key}\n',
    )  # fmt: skip
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
