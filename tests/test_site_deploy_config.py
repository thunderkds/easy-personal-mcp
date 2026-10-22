"""HUN-57 test for the Vercel deploy config (scope: PROJECT_SPEC_SITE.md).

`vercel.json` must serve exactly `site/`; `.vercelignore` must be an allowlist that admits
only `site/**` and `vercel.json`, so `memory/`, `tasks/` and `docs/` are never uploaded.
"""

from __future__ import annotations

import fnmatch
import json
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOWED_NEGATIONS = ["!site/", "!site/**", "!vercel.json"]
SKIP_DIRS = {".git", ".venv", "__pycache__", ".pytest_cache", ".ruff_cache", "node_modules"}


def effective_lines(text: str) -> list[str]:
    return [ln.strip() for ln in text.splitlines() if ln.strip() and not ln.strip().startswith("#")]


def _matches(pattern: str, rel: str) -> bool:
    if pattern.endswith("/"):
        return rel.startswith(pattern)
    return fnmatch.fnmatch(rel, pattern)


def admitted(lines: list[str], rel: str) -> bool:
    """Gitignore-style: the last matching line wins; `!` re-admits."""
    result = True
    for line in lines:
        negate = line.startswith("!")
        if _matches(line[1:] if negate else line, rel):
            result = negate
    return result


def violations(text: str) -> list[str]:
    lines = effective_lines(text)
    problems = []
    if not lines or lines[0] != "*":
        problems.append("first effective line is not `*`")
    problems += [f"unexpected line: {ln}" for ln in lines[1:] if ln not in ALLOWED_NEGATIONS]
    return problems


def _ignore_text() -> str:
    return (ROOT / ".vercelignore").read_text(encoding="utf-8")


def test_vercel_json_serves_site_only() -> None:
    data = json.loads((ROOT / "vercel.json").read_text(encoding="utf-8"))
    assert data == {"outputDirectory": "site"}


def test_vercelignore_is_exact_allowlist() -> None:
    assert violations(_ignore_text()) == []
    assert effective_lines(_ignore_text()) == ["*", *ALLOWED_NEGATIONS]
    assert any(ln.startswith("#") for ln in _ignore_text().splitlines())


def test_negative_extra_readmit_is_detected() -> None:
    assert violations(_ignore_text() + "!memory/\n") == ["unexpected line: !memory/"]
    assert violations(_ignore_text() + "!tasks/**\n")


def test_negative_removed_star_is_detected() -> None:
    without_star = "\n".join(ln for ln in _ignore_text().splitlines() if ln.strip() != "*")
    assert violations(without_star)


def test_comments_and_blanks_ignored_by_parser() -> None:
    assert effective_lines("# c\n\n  \n*\n#x\n!site/\n") == ["*", "!site/"]


def test_admitted_set_on_real_tree_is_site_and_vercel_json() -> None:
    lines = effective_lines(_ignore_text())
    got = set()
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for name in filenames:
            rel = (Path(dirpath) / name).relative_to(ROOT).as_posix()
            if admitted(lines, rel):
                got.add(rel)
    site_files = {p.relative_to(ROOT).as_posix() for p in (ROOT / "site").rglob("*") if p.is_file()}
    assert site_files
    assert got == site_files | {"vercel.json"}


def test_private_folders_not_admitted() -> None:
    lines = effective_lines(_ignore_text())
    for rel in ("reports/x.json", "memory/MEMORY.md", "tasks/TASK_GUIDE_HUN-57.md", ".env", "src/easy_personal/cli.py"):
        assert not admitted(lines, rel)
