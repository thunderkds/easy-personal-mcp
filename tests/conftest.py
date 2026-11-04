from pathlib import Path

import pytest

FIXTURES = Path(__file__).parent / "fixtures" / "claude_code"


@pytest.fixture
def fixtures_root() -> Path:
    return FIXTURES


@pytest.fixture
def conn(tmp_path):
    from easy_personal.core import store

    connection = store.connect(tmp_path / "store.sqlite")
    yield connection
    connection.close()


@pytest.fixture(autouse=True)
def _no_real_easy_context(monkeypatch):
    """Tests never reach the real easy-context service: point the client at a dead port."""
    monkeypatch.setenv("EASY_PERSONAL_CONTEXT_URL", "http://127.0.0.1:9/mcp")


@pytest.fixture(autouse=True)
def _no_real_verifier_reports(monkeypatch, tmp_path_factory):
    """Tests never read the owner's ~/.easy-verifier-reports: point the container at an empty dir."""
    monkeypatch.setenv(
        "EASY_PERSONAL_VERIFIER_REPORTS", str(tmp_path_factory.mktemp("verifier-none"))
    )


REAL_DATA_DIR = (Path.home() / ".easy-personal").resolve()  # taken at import, before any HOME patch


@pytest.fixture(autouse=True)
def _never_open_the_real_store(monkeypatch):
    """NFR-003: a test that opens anything under the owner's real ~/.easy-personal fails."""
    import sqlite3

    real_connect = sqlite3.connect

    def guarded(database, *args, **kwargs):
        if isinstance(database, str | Path) and str(database) != ":memory:":
            target = Path(database).resolve()
            if target == REAL_DATA_DIR or REAL_DATA_DIR in target.parents:
                raise AssertionError(f"test opened the real store: {target}")
        return real_connect(database, *args, **kwargs)

    monkeypatch.setattr(sqlite3, "connect", guarded)


@pytest.fixture(autouse=True)
def _isolated_data_dir(monkeypatch, tmp_path_factory):
    """Every test gets its own EASY_PERSONAL_HOME; tests that set their own keep overriding it."""
    monkeypatch.setenv("EASY_PERSONAL_HOME", str(tmp_path_factory.mktemp("epm-home")))


@pytest.fixture(autouse=True)
def _isolated_agent_root(monkeypatch, tmp_path_factory):
    """Account detection scans a fresh empty dir, never the owner's real ~/.claude* / ~/.codex*."""
    monkeypatch.setenv("EASY_PERSONAL_AGENT_ROOT", str(tmp_path_factory.mktemp("agent-root")))


@pytest.fixture(autouse=True)
def _isolated_easy_memory_home(monkeypatch, tmp_path_factory):
    """HUN-184: tests never read the owner's ~/.easy-memory; tests that plant a store override this."""
    monkeypatch.setenv("EASY_MEMORY_HOME", str(tmp_path_factory.mktemp("em-none")))


@pytest.fixture
def real_projects(monkeypatch):
    """Every stored project path counts as a real project, never Scratch (FR-069).

    For tests that use made-up or temp-folder project paths and check something else (FR-057 owners,
    rules, filters, golden files). Tests of the Scratch rule itself do not use it.
    """
    from easy_personal.core import owners

    monkeypatch.setattr(owners, "_is_scratch", lambda path: False)
