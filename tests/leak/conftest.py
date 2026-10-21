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
    home = root / "home"
    env = harness.child_env(home, projects)
    mp = pytest.MonkeyPatch()
    mp.setenv("EASY_PERSONAL_HOME", str(home))
    mp.setenv("EASY_PERSONAL_CLAUDE_PROJECTS", str(projects))
    out = harness.Collected()
    try:
        harness.collect_cli(env, projects, out)  # first call to `ingest` fills the store
        with harness.LogCapture() as logs:
            harness.collect_mcp(harness.build_app(), out, logs)
        harness.collect_mcp_stdio(env, root / "stdio-stderr.log", out)
    finally:
        mp.undo()
    out.outputs.update(harness.store_blobs(home))
    return out
