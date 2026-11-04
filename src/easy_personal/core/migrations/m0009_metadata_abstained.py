"""Migration 9: clear the stale `abstained` flag on Claude Code metadata rows (HUN-209).

HUN-175 (2026-11-01) added `api-request` and `api-request-shape` to the parser's metadata types, but rows
stored before that kept `abstained = 1`: a re-read (migrations 7 and 8) only updates `entrypoint` and
`tool_error_count` on an existing row, so the flag never cleared and the `records_unreadable` rule fired
on records that are fine. A metadata parse keeps only `record_type`, `version`, `ts_utc` and `session_id`,
and `_abstain` already stored those four, so the cleared row equals what a fresh parse writes. This
migration reads no file and adds or deletes no row. The type names below are a frozen copy of
`sources.claude_code.METADATA_TYPES` as of 2026-11-04: a migration must not follow the live list. Rows
abstained for any other reason (bad shape of a versioned type, unknown type, NULL type) and rows of
other sources stay abstained. Idempotent: a second run matches no row. Undo: the pre-migration copy
`<store>.bak-v8` is the restore path.
"""

import sqlite3

METADATA_TYPES = (
    "agent-name",
    "custom-title",
    "last-prompt",
    "atis-latch",
    "queue-operation",
    "cost-state",
    "file-history-snapshot",
    "file-history-delta",
    "artifact-comment-monitor",
    "artifact-autoreact-ledger",
    "frame-link",
    "api-request",
    "api-request-blob",
    "api-request-shape",
    "tools-used",
    "ai-title",
    "mode",
    "permission-mode",
    "bridge-session",
)


def apply(conn: sqlite3.Connection) -> None:
    marks = ", ".join("?" * len(METADATA_TYPES))
    conn.execute(
        "UPDATE events SET abstained = 0 "
        f"WHERE source = 'claude_code' AND abstained = 1 AND record_type IN ({marks})",
        METADATA_TYPES,
    )
