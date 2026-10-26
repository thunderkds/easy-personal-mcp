import sqlite3
from pathlib import Path

import pytest

from easy_personal.core import config

REAL_DATA_DIR = (Path.home() / ".easy-personal").resolve()


def test_data_dir_is_never_the_real_home(tmp_path):
    assert config.data_dir().resolve() != REAL_DATA_DIR
    assert REAL_DATA_DIR not in config.data_dir().resolve().parents


def test_opening_the_real_store_path_is_refused():
    with pytest.raises(AssertionError, match="real store"):
        sqlite3.connect(REAL_DATA_DIR / "store.sqlite")  # refused before any file is touched


def test_account_detection_never_lists_the_real_home(conn):
    from easy_personal.core import accounts

    accounts.detect(conn)
    real_home = Path.home().resolve()
    assert config.agent_root().resolve() != real_home
    assert real_home not in config.agent_root().resolve().parents
    for account in accounts.list_accounts(conn):
        assert not Path(account["folder"]).resolve().is_relative_to(real_home)
