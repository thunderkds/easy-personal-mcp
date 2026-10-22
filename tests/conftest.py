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
