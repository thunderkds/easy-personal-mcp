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
