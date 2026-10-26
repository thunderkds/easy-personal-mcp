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
