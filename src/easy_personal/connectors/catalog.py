"""Provider catalog: declarative entries in `easy_personal/catalog/*.yaml` (PRD FR-016).

Restricted line format, no YAML library: `key: value` per line, `field:` repeats. See slack.yaml.
"""
import re
import shlex
from dataclasses import dataclass
from pathlib import Path

PACKAGED = Path(__file__).resolve().parent.parent / "catalog"
_ID = re.compile(r"[a-z0-9][a-z0-9_-]{0,63}")
_ENV = re.compile(r"[A-Z_][A-Z0-9_]*")
_PINNED = re.compile(r"(@[a-z0-9._-]+/)?[a-z0-9._-]+@\d+\.\d+\.\d+([-+][0-9A-Za-z.-]+)?")
_SINGLE = ("category", "provider", "claude_ai_connector", "command")


class CatalogError(Exception):
    """Bad or unknown catalog entry; the message is safe to print."""


@dataclass(frozen=True)
class Field:
    env: str
    label: str
    secret: bool


@dataclass(frozen=True)
class Entry:
    id: str
    category: str
    provider: str
    claude_ai_connector: bool
    command: list[str]
    fields: tuple[Field, ...]


def _field(entry_id: str, text: str) -> Field:
    parts = [p.strip() for p in text.split("|")]
    if len(parts) != 3 or not _ENV.fullmatch(parts[0]) or not parts[1] or parts[2] not in (
        "secret", "plain"
    ):
        raise CatalogError(f"{entry_id}: bad field line (want ENV_NAME | label | secret|plain)")
    return Field(parts[0], parts[1], parts[2] == "secret")


def _check_pinned(entry_id: str, command: list[str]) -> None:
    """A package runner (npx, uvx, pipx run) must name its package at an exact version."""
    if command[:1] in (["npx"], ["uvx"]):
        rest = command[1:]
    elif command[:2] == ["pipx", "run"]:
        rest = command[2:]
    else:
        return
    package = next((t for t in rest if not t.startswith("-")), "")
    if not _PINNED.fullmatch(package):
        raise CatalogError(f"{entry_id}: package '{package}' must be pinned (name@1.2.3)")


def parse(entry_id: str, text: str) -> Entry:
    if not _ID.fullmatch(entry_id):
        raise CatalogError("invalid catalog entry name")
    single: dict[str, str] = {}
    fields: list[Field] = []
    for line in text.splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        key, _, value = line.partition(":")
        key, value = key.strip(), value.strip()
        if key == "field":
            fields.append(_field(entry_id, value))
        elif key in _SINGLE and key not in single:
            single[key] = value
        else:
            raise CatalogError(f"{entry_id}: unknown or repeated key '{key}'")
    missing = [k for k in _SINGLE if not single.get(k)]
    if missing or not fields:
        raise CatalogError(f"{entry_id}: missing {', '.join(missing) or 'field'}")
    if single["claude_ai_connector"] not in ("yes", "no"):
        raise CatalogError(f"{entry_id}: claude_ai_connector must be yes or no")
    if len({f.env for f in fields}) != len(fields):
        raise CatalogError(f"{entry_id}: duplicate field")
    command = shlex.split(single["command"])
    _check_pinned(entry_id, command)
    return Entry(
        id=entry_id,
        category=single["category"],
        provider=single["provider"],
        claude_ai_connector=single["claude_ai_connector"] == "yes",
        command=command,
        fields=tuple(fields),
    )


def ids() -> list[str]:
    return sorted(p.stem for p in PACKAGED.glob("*.yaml"))


def load(entry_id: str) -> Entry:
    path = PACKAGED / f"{entry_id}.yaml"
    if not _ID.fullmatch(entry_id) or not path.is_file():
        raise CatalogError(f"unknown provider '{entry_id}'; available: {', '.join(ids())}")
    return parse(entry_id, path.read_text())
