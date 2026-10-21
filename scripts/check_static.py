#!/usr/bin/env python3
"""Release check for the dashboard build (HUN-48). Stdlib only, self-contained.

    check_static.py [--root DIR] [--wheel FILE]   run every check; exit 1 on any problem
    check_static.py [--root DIR] --write-stamp    record the source hash (run by `npm run build`)

Checks: static build present; build not stale (hash of web/ sources equals the stamp written at
build time); no remote URL or source map in built files; lockfile has no telemetry package and
every direct dependency is pinned exactly; optional wheel contains the static assets.
"""

import argparse
import hashlib
import json
import re
import sys
import zipfile
from pathlib import Path

STATIC_REL = Path("src/easy_personal/dashboard/static")
STAMP = ".web-stamp"
SOURCE_FILES = ("index.html", "package.json", "package-lock.json", "vite.config.js")
URL_RE = re.compile(r"(?:https?:)?//[A-Za-z0-9][A-Za-z0-9.\-]*\.[A-Za-z]{2,}[^\s\"'`)<>]*|https?://[^\s\"'`)<>]+")
# Strings that appear in bundles but are never fetched: XML namespace ids, React's error-doc text.
ALLOWED_URL_PREFIXES = (
    "http://www.w3.org/",
    "https://www.w3.org/",
    "https://react.dev/errors",
    "https://react.dev/link/",
)
TELEMETRY_RE = re.compile(
    r"(^|/)(@?(sentry|posthog|mixpanel|segment|amplitude|datadog|newrelic|bugsnag|rollbar|"
    r"logrocket|fullstory|hotjar|heap|plausible|google-analytics|react-ga|gtag|"
    r"analytics|telemetry|rudder|statsig|launchdarkly|appinsights|opentelemetry)"
    r"[\w.\-]*)(/|$)",
    re.IGNORECASE,
)
PIN_RE = re.compile(r"^\d+\.\d+\.\d+(-[\w.]+)?$")


def source_hash(root: Path) -> str:
    web = root / "web"
    files = [web / n for n in SOURCE_FILES if (web / n).is_file()]
    files += sorted(p for p in (web / "src").rglob("*") if p.is_file())
    h = hashlib.sha256()
    for p in sorted(files):
        h.update(p.relative_to(web).as_posix().encode() + b"\0")
        h.update(p.read_bytes() + b"\0")
    return h.hexdigest()


def check_present(static: Path) -> list[str]:
    if not (static / "index.html").is_file():
        return [f"static build missing: {static / 'index.html'} (run `npm --prefix web run build`)"]
    if not any(p.is_file() and p.name != STAMP for p in (static / "assets").rglob("*")):
        return [f"static build missing: no files under {static / 'assets'}"]
    return []


def check_fresh(root: Path, static: Path) -> list[str]:
    stamp = static / STAMP
    if not stamp.is_file():
        return [f"static build stale: {stamp} missing (rebuild with `npm --prefix web run build`)"]
    if stamp.read_text().strip() != source_hash(root):
        return ["static build stale: web/ sources changed since the last build"]
    return []


def check_no_remote(static: Path) -> list[str]:
    problems = []
    for p in sorted(static.rglob("*")):
        if not p.is_file() or p.name == STAMP:
            continue
        text = p.read_text(errors="replace")
        for m in URL_RE.findall(text):
            if m.startswith(ALLOWED_URL_PREFIXES):
                continue
            problems.append(f"remote URL in {p.relative_to(static)}: {m[:80]}")
        if re.search(r"sourceMappingURL", text) or p.suffix == ".map":
            problems.append(f"sourcemap present: {p.relative_to(static)}")
    return problems


def check_lock(root: Path) -> list[str]:
    web = root / "web"
    problems = []
    try:
        pkg = json.loads((web / "package.json").read_text())
        lock = json.loads((web / "package-lock.json").read_text())
    except (OSError, ValueError) as e:
        return [f"cannot read web/package.json or web/package-lock.json: {e}"]
    for section in ("dependencies", "devDependencies", "optionalDependencies"):
        for name, spec in pkg.get(section, {}).items():
            if not PIN_RE.match(spec):
                problems.append(f"dependency not pinned exactly: {name} {spec}")
    for key, entry in lock.get("packages", {}).items():
        if not key:
            continue
        name = key.split("node_modules/")[-1]
        if TELEMETRY_RE.search(name):
            problems.append(f"telemetry/analytics package in lockfile: {name}")
        resolved = entry.get("resolved", "")
        if resolved and not resolved.startswith("https://registry.npmjs.org/"):
            problems.append(f"lockfile package not from registry.npmjs.org: {name}")
    return problems


def check_wheel(wheel: Path) -> list[str]:
    try:
        names = zipfile.ZipFile(wheel).namelist()
    except (OSError, zipfile.BadZipFile) as e:
        return [f"cannot read wheel {wheel}: {e}"]
    prefix = "easy_personal/dashboard/static/"
    if not any(n == prefix + "index.html" for n in names) or not any(
        n.startswith(prefix + "assets/") for n in names
    ):
        return [f"wheel lacks static assets under {prefix}: {wheel.name}"]
    return []


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    ap.add_argument("--wheel", type=Path)
    ap.add_argument("--write-stamp", action="store_true")
    args = ap.parse_args(argv)
    root = args.root.resolve()
    static = root / STATIC_REL
    if args.write_stamp:
        static.mkdir(parents=True, exist_ok=True)
        (static / STAMP).write_text(source_hash(root) + "\n")
        return 0
    problems = check_present(static)
    if not problems:
        problems += check_fresh(root, static) + check_no_remote(static)
    problems += check_lock(root)
    if args.wheel:
        problems += check_wheel(args.wheel)
    for p in problems:
        print(f"FAIL: {p}", file=sys.stderr)
    if not problems:
        print("check_static: ok")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
