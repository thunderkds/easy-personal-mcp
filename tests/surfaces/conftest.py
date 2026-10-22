"""Share the fake keychain and fake `claude` fixtures of tests/connectors with the API tests."""
import importlib.util
import sys
from pathlib import Path

_DIR = Path(__file__).resolve().parents[1] / "connectors"
sys.path.insert(0, str(_DIR))  # the fixtures import `fake_backend` from there
_spec = importlib.util.spec_from_file_location("connector_fixtures", _DIR / "conftest.py")
_mod = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_mod)

fake_claude, launcher, mem_keyring, no_claude, no_keyring = (
    _mod.fake_claude, _mod.launcher, _mod.mem_keyring, _mod.no_claude, _mod.no_keyring,
)  # fmt: skip
