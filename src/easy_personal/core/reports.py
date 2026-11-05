"""Saved investigate reports (HUN-221, FR-076..FR-078, DDR-0003).

The owner's own Claude Code session sends its `investigate` report to MCP `save_report`. The server stores
that text as sent (within limits, after a secret check) next to its own *rule snapshot*, taken at save time.
All numbers come from the server (NFR-004); the text is never parsed into numbers and the server writes no
text of its own. "Now" for the before / now view is worked out on every read and never stored.
"""

import json
import math
import re
import sqlite3
from datetime import UTC, datetime
from importlib.metadata import version

from easy_personal.core import config, metrics, rules

SUMMARY_MAX = 2000
FIXES_MAX = 5
FIX_TEXT_MAX = 300
CITE_MAX = 100  # `tool` and `number` of a fix: short names, not prose
LIST_LIMIT = 50
NOT_MEASURED = "not measured now"
NOT_IN_USE = {"state": "not_in_use", "reason": "not in use"}  # FR-080
FIX_KEYS = {"text", "tool", "number"}

# Secret-looking text is refused (FR-076): key prefixes, a PEM header, or a 32+ character letter-and-digit run.
# A UUID (hyphens split it into runs of at most 12) and a file path (slashes split it) never match.
_SECRET = re.compile(
    r"\bsk-[A-Za-z0-9]"
    r"|\bgh[pousr]_[A-Za-z0-9]|\bgithub_pat_"
    r"|\bxox[A-Za-z]?[-_]?[A-Za-z0-9]{4,}"
    r"|\b(?:AKIA|ASIA)[A-Z0-9]{4,}"
    r"|-----BEGIN"
    r"|[A-Za-z0-9]{32,}"
)


def _check_text(field: str, value, limit: int | None) -> str:
    if not isinstance(value, str):
        raise ValueError(f"{field}: must be text; nothing was saved")  # noqa: TRY004 (every refusal is a ValueError)
    if not value.strip():
        raise ValueError(f"{field}: must not be empty; nothing was saved")
    if limit is not None and len(value) > limit:
        raise ValueError(f"{field}: {len(value)} characters, the limit is {limit}; nothing was saved")
    if _SECRET.search(value):
        raise ValueError(f"{field}: looks like a secret (a key, token or private-key header); nothing was saved")
    return value


def validate(summary, fixes) -> tuple[str, list[dict]]:
    """Return the clean `(summary, fixes)` or raise ValueError naming the failing field; the value is never echoed."""
    summary = _check_text("summary", summary, SUMMARY_MAX)
    if not isinstance(fixes, list):
        raise ValueError("fixes: must be a list; nothing was saved")  # noqa: TRY004
    if len(fixes) > FIXES_MAX:
        raise ValueError(f"fixes: {len(fixes)} items, the limit is {FIXES_MAX}; nothing was saved")
    clean = []
    for n, fix in enumerate(fixes):
        name = f"fixes[{n}]"
        if not isinstance(fix, dict) or not FIX_KEYS.issuperset(fix) or "text" not in fix:
            raise ValueError(f"{name}: must be an object with `text` and optional `tool`, `number`; nothing was saved")
        item = {"text": _check_text(f"{name}.text", fix["text"], FIX_TEXT_MAX)}
        # an optional field sent as "" (models often fill optional arguments that way) counts as absent
        if fix.get("tool") not in (None, ""):
            item["tool"] = _check_text(f"{name}.tool", fix["tool"], CITE_MAX)
        number = None if fix.get("number") == "" else fix.get("number")
        if number is not None:
            if isinstance(number, bool) or not isinstance(number, str | int | float):
                raise ValueError(f"{name}.number: must be a number or text; nothing was saved")
            if isinstance(number, float) and not math.isfinite(number):
                raise ValueError(f"{name}.number: must be a finite number; nothing was saved")
            if isinstance(number, str):
                _check_text(f"{name}.number", number, CITE_MAX)
            item["number"] = number
        clean.append(item)
    return summary, clean


def _check_scope(conn: sqlite3.Connection, workspace: str | None, account: str | None) -> None:
    """Refuse an unknown scope with the field named (the value is not echoed)."""
    if workspace is not None:
        try:
            metrics._scope(conn, workspace, None, None)
        except ValueError:
            raise ValueError("workspace: unknown workspace; nothing was saved") from None
    if account is not None:
        try:
            metrics._scope(conn, None, None, None, account)
        except ValueError:
            raise ValueError("account: unknown account; nothing was saved") from None


def save(
    conn: sqlite3.Connection,
    summary,
    fixes,
    workspace: str | None = None,
    account: str | None = None,
    now: datetime | None = None,
) -> dict:
    """Store one report with the server's rule snapshot for its scope; return `{id, saved_utc}`.

    Any refusal raises ValueError before anything is written.
    """
    summary, fixes = validate(summary, fixes)
    workspace, account = workspace or None, account or None
    _check_scope(conn, workspace, account)
    now = now or datetime.now(UTC)
    found = rules.evaluate(conn, now, workspace=workspace, account=account, checked=True)
    snapshot = {
        "as_of_utc": found["as_of_utc"],
        "checked": found["checked"],
        "fired": found["hits"],  # the whole warning hit: the page draws the Warnings & plan card from it (FR-079)
        "abstained": found["abstained"],
    }
    saved_utc = now.astimezone(UTC).strftime("%Y-%m-%dT%H:%M:%SZ")
    with conn:
        cursor = conn.execute(
            "INSERT INTO investigate_reports (saved_utc, server_version, workspace, account, summary, "
            "fixes_json, snapshot_json) VALUES (?, ?, ?, ?, ?, ?, ?)",
            (
                saved_utc,
                version("easy-personal-mcp"),
                workspace,
                account,
                summary,
                json.dumps(fixes, sort_keys=True),
                json.dumps(snapshot, sort_keys=True),
            ),
        )
    return {"id": cursor.lastrowid, "saved_utc": saved_utc}


def list_reports(conn: sqlite3.Connection) -> dict:
    """The newest `LIST_LIMIT` reports, newest first: save time, scope, rules fired then, summary's first line."""
    rows = conn.execute(
        "SELECT id, saved_utc, workspace, account, summary, snapshot_json FROM investigate_reports "
        "ORDER BY id DESC LIMIT ?",
        (LIST_LIMIT,),
    ).fetchall()
    return {
        "reports": [
            {
                "id": id_,
                "saved_utc": saved_utc,
                "workspace": workspace,
                "account": account,
                "fired_count": sum(1 for r in json.loads(snapshot)["checked"] if r["fired"]),
                "summary_first_line": (summary.splitlines() or [""])[0],
            }
            for id_, saved_utc, workspace, account, summary, snapshot in rows
        ]
    }


def _fill_from_config(entry: dict, current: dict[str, rules.Rule]) -> dict:
    """A fired entry saved before FR-079 has no `label`: fill what the config says now, never `steps`."""
    rule = current.get(entry["rule_id"])
    if "label" in entry or rule is None:
        return entry
    filled = {k: getattr(rule, k) for k in ("label", "priority", "why", "fix", "window_days")}
    return {**entry, **filled}


def _before(snapshot: dict) -> dict[str, tuple[str, dict]]:
    fired = {f["rule_id"]: f for f in snapshot["fired"]}
    current = {}
    if any("label" not in f for f in fired.values()):
        try:
            current = {r.id: r for r in rules.load_rules(config.config_path())}
        except ValueError:  # an unreadable config: the stored fields are all there is
            pass
    out = {}
    for rule in snapshot["checked"]:
        state = {"state": "fired" if rule["fired"] else "clear"}
        state.update({k: rule[k] for k in ("measured", "threshold", "fired")})
        if rule["fired"] and rule["rule_id"] in fired:
            state.update(_fill_from_config(fired[rule["rule_id"]], current))
        out[rule["rule_id"]] = (rule["title"], state)
    for rule in snapshot["abstained"]:
        out[rule["rule_id"]] = (rule["title"], {"state": "abstained", "reason": rule["reason"]})
    return out


def _now(found: dict | None) -> dict[str, dict]:
    if found is None:
        return {}
    fired = {h["rule_id"]: h for h in found["hits"]}
    out = {}
    for rule in found["checked"]:
        state = {"state": "fired" if rule["fired"] else "clear"}
        state.update({k: rule[k] for k in ("measured", "threshold", "fired")})
        if rule["rule_id"] in fired:
            state["summary"] = fired[rule["rule_id"]]["summary"]
        out[rule["rule_id"]] = state
    for rule in found["abstained"]:
        out[rule["rule_id"]] = {"state": "abstained", "reason": rule["reason"]}
    return out


def get_report(conn: sqlite3.Connection, report_id: int, now: datetime | None = None) -> dict | None:
    """One report with a before / now entry per rule of its snapshot, or None for an unknown id.

    Before = the saved snapshot; now = the same rule measured on this call for the same scope (never stored).
    A rule missing from the current config says `not measured now`; a workspace or account deleted since the
    save makes every rule `not measured now`. Each side keeps its own threshold.
    """
    row = conn.execute(
        "SELECT id, saved_utc, server_version, workspace, account, summary, fixes_json, snapshot_json "
        "FROM investigate_reports WHERE id = ?",
        (report_id,),
    ).fetchone()
    if row is None:
        return None
    id_, saved_utc, server_version, workspace, account, summary, fixes, snapshot = row
    snapshot = json.loads(snapshot)
    try:
        found = rules.evaluate(conn, now, workspace=workspace, account=account, checked=True)
    except ValueError:  # the saved workspace or account no longer exists
        found = None
    after = _now(found)
    scope = {"workspace": workspace, "account": account}
    # a rule evaluate left out because its kit part is not in use: `not in use` now (FR-080)
    not_in_use = set()
    if found:  # the same moment `evaluate` measured at
        moment = datetime.fromisoformat(found["as_of_utc"])
        not_in_use = {r.id for r in rules.load_rules(config.config_path()) if rules.left_out(r, conn, moment, scope)}
    after.update({rule_id: NOT_IN_USE for rule_id in not_in_use})
    missing = {"state": "not_measured", "reason": NOT_MEASURED}
    return {
        "id": id_,
        "saved_utc": saved_utc,
        "server_version": server_version,
        "workspace": workspace,
        "account": account,
        "summary": summary,
        "fixes": json.loads(fixes),
        "as_of_utc": snapshot["as_of_utc"],
        "now_as_of_utc": found["as_of_utc"] if found else None,
        "rules": [
            {"rule_id": rule_id, "title": title, "before": before, "now": after.get(rule_id, missing)}
            for rule_id, (title, before) in sorted(_before(snapshot).items())
            if not (rule_id in not_in_use and before["state"] == "abstained")  # nothing worth showing (FR-080)
        ],
    }
