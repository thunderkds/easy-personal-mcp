"""Adopt pre-workspace connectors into the orphan workspace (FR-034, DDR-0001).

Legacy = a catalog id that has a keychain item `easy-personal/<id>` and/or an entry in the DEFAULT
Claude account (`claude mcp get <id>` that points at `easy-personal-launch exec <id>`) but no `connectors` row. Adoption inserts
`connectors(id=<name>, alias=<name>, workspace=orphan)` and, only after confirmation, runs
`claude mcp remove <name> --scope user` in the default account. An entry of that name that is not our
launcher entry is never removed. The keychain is never written.
No other account and no other name is touched. No line holds a secret.
"""

import shlex
import shutil
import sqlite3
from dataclasses import dataclass

from easy_personal.connectors import catalog, writer


@dataclass(frozen=True)
class Legacy:
    id: str
    keychain: bool
    claude: bool  # the default account holds an entry that points at our launcher (ours: may be removed)
    foreign: bool = False  # same name, but not our launcher entry: never removed

    def as_dict(self) -> dict:
        return {"id": self.id, "keychain": self.keychain, "claude": self.claude, "foreign": self.foreign}


def detect(conn: sqlite3.Connection) -> list[Legacy]:
    """Legacy catalog ids not yet stored. LaunchError: no usable keychain backend."""
    stored = {r[0] for r in conn.execute("SELECT id FROM connectors")}
    has_claude = shutil.which("claude") is not None
    env = writer.account_env(None)  # the default account only: CLAUDE_CONFIG_DIR unset
    found = []
    for name in catalog.ids():
        if name in stored:
            continue
        in_keychain = writer._keychain_item(name)
        got = writer._entry_exists(name, env) if has_claude else None
        ours = got is not None and writer.points_at_launcher(got, name)
        if in_keychain or ours:
            found.append(Legacy(name, in_keychain, ours, got is not None and not ours))
    return found


def _pending(name: str) -> str:
    return f"CLAUDE_CONFIG_DIR unset: claude mcp remove {shlex.quote(name)} --scope user"


def adopt(conn: sqlite3.Connection, confirm: writer.Confirm, out: writer.Out, assume_yes: bool = False) -> int:
    found = detect(conn)
    if not found:
        out("nothing to adopt: no pre-workspace connector found.")
        return 0
    out("pre-workspace connectors found:")
    for item in found:
        note = "" if item.keychain else " (no keychain item: it fails at launch until you add it again)"
        out(f"  {item.id}: keychain {'yes' if item.keychain else 'no'}, Claude Code default account "
            f"{'yes' if item.claude else 'no'}{note}")  # fmt: skip
    out("will change:")
    for item in found:
        out(f"  store: adopt {item.id} (id {item.id}, alias {item.id}) into orphan")
        if item.claude:
            out(f"  Claude Code (default account): remove {item.id}")
        elif item.foreign:
            out(f"  Claude Code (default account): {item.id} is not an easy-personal entry; left as it is")
    out("  keychain: untouched")
    if not assume_yes and not confirm():
        out("aborted; nothing was changed.")
        return 1
    orphan = conn.execute("SELECT id FROM workspaces WHERE is_orphan = 1").fetchone()[0]
    env = writer.account_env(None)
    failed = 0
    for item in found:
        try:
            with conn:
                conn.execute(
                    "INSERT INTO connectors (id, alias, alias_custom, provider, workspace_id) VALUES (?, ?, 0, ?, ?)",
                    (item.id, item.id, item.id, orphan),
                )
        except sqlite3.IntegrityError:
            out(f"skipped {item.id}: the store refused it (changed meanwhile).")
            failed = 1
            continue
        out(f"done: {item.id} adopted into orphan.")
        if not item.claude:
            continue
        done = writer.remove_entry(item.id, env)
        if done.returncode != 0:
            failed = 1
            out(f"pending: `claude` could not remove {item.id} (exit {done.returncode}); the row stays in orphan. "
                f"Run: {_pending(item.id)}")  # fmt: skip
            if done.stderr.strip():
                out(done.stderr.strip())
        else:
            out(f"done: removed {item.id} from Claude Code (default account).")
    if not failed:
        out("assign them to a workspace with `easy-personal connector move <id> <workspace>` when you are ready.")
    return failed


def doctor(conn: sqlite3.Connection, out: writer.Out) -> None:
    """One informational line when legacy connectors are waiting; never fails the check."""
    from easy_personal.launch import LaunchError

    try:
        found = detect(conn)
    except LaunchError:
        return  # the keychain line of `doctor` already reports the missing backend
    if found:
        out(f"note legacy connectors not adopted: {', '.join(i.id for i in found)}; "
            "run `easy-personal connector adopt-legacy`")  # fmt: skip
