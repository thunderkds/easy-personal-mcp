"""Reconcile each Claude account's user-scope MCP entries with the store (FR-033, DDR-0001).

Desired = connectors of the workspaces an account picked; registered = `account_registrations`, the
aliases easy-personal wrote there. `plan` lists the difference; its lines are the confirmation text.
`apply` runs `claude mcp add-json|remove --scope user` with that account's CLAUDE_CONFIG_DIR (removed for
the default account) and records each step right after it succeeds, so a failure can be re-run.
A name that is not registered is never removed or overwritten. No line holds a secret.
"""

import shlex
import shutil
import sqlite3
from dataclasses import dataclass
from pathlib import Path

from easy_personal.connectors import keychain, writer
from easy_personal.core import accounts
from easy_personal.launch import LaunchError

APPLIED = ("remove", "forget", "add", "record")  # actions apply() runs; `refused` / `skipped` are notes


@dataclass(frozen=True)
class Step:
    account_id: int
    account: str
    config_dir_env: str | None
    action: str  # add | remove | record | forget | refused | skipped
    alias: str = ""
    connector_id: str = ""
    note: str = ""

    def line(self) -> str:
        note = f" ({self.note})" if self.note else ""
        target = f"exec {self.connector_id}"
        return {
            "add": f"{self.account}: add {self.alias} -> {target}{note}",
            "remove": f"{self.account}: remove {self.alias} ({target}){note}",
            "record": f"{self.account}: record {self.alias} -> {target} (already in Claude Code)",
            "forget": f"{self.account}: forget {self.alias} ({target}) (already gone from Claude Code)",
            "refused": f"{self.account}: refused {self.alias} -> {target}: an MCP server with this name "
                       "exists that easy-personal did not add; nothing is overwritten",
            "skipped": f"{self.account}: skipped{note}",
        }[self.action]  # fmt: skip

    def as_dict(self) -> dict:
        return {"account_id": self.account_id, "account": self.account, "action": self.action,
                "alias": self.alias, "connector_id": self.connector_id, "line": self.line()}  # fmt: skip


def _pairs(conn: sqlite3.Connection, sql: str, account_id: int) -> dict[str, str]:
    return dict(conn.execute(sql, (account_id,)).fetchall())


def plan(conn: sqlite3.Connection, account_id: int, drop: frozenset[str] = frozenset()) -> list[Step]:
    """Steps that make one account match its picks. `drop`: connector ids treated as not desired."""
    _, provider, folder, label, status, config_dir_env = conn.execute(
        "SELECT id, provider, folder, label, status, config_dir_env FROM agent_accounts WHERE id = ?",
        (account_id,),
    ).fetchone()
    name = label or Path(folder).name
    step = lambda action, alias="", cid="", note="": Step(
        account_id, name, config_dir_env, action, alias, cid, note
    )
    if provider != "claude" or status not in ("confirmed", "ignored"):
        return [step("skipped", note="not a confirmed Claude account")]
    registered = _pairs(conn, "SELECT alias, connector_id FROM account_registrations WHERE account_id = ?", account_id)
    if status == "ignored" and not registered:
        return []
    if not Path(folder).is_dir():
        return [step("skipped", note="folder missing")]
    if status == "ignored":  # its picks no longer count: remove every entry we added there
        desired = {}
    else:
        desired = _pairs(conn, "SELECT c.alias, c.id FROM connectors c JOIN account_workspaces p "
                               "ON p.workspace_id = c.workspace_id WHERE p.account_id = ?", account_id)  # fmt: skip
    desired = {a: c for a, c in desired.items() if c not in drop}
    has_claude = shutil.which("claude") is not None
    env = writer.account_env(config_dir_env)
    get = (lambda alias: writer._entry_exists(alias, env)) if has_claude else (lambda alias: None)
    first, then = [], []  # removals before additions, so a re-used alias is free when it is added
    for alias, cid in sorted(registered.items()):
        if desired.get(alias) == cid:
            if not has_claude:
                continue
            got = get(alias)
            if got is None:
                then.append(step("add", alias, cid, "missing from Claude Code: re-add"))
            elif not writer.points_at_launcher(got, cid):
                first.append(step("remove", alias, cid, "does not point at the launcher"))
                then.append(step("add", alias, cid, "re-add"))
        elif has_claude and get(alias) is None:
            first.append(step("forget", alias, cid))
        else:
            first.append(step("remove", alias, cid))
    for alias, cid in sorted(desired.items()):
        if registered.get(alias) == cid:
            continue
        got = get(alias) if alias not in registered else None  # a registered alias is ours: removed above
        if got is None:
            then.append(step("add", alias, cid))
        elif writer.points_at_launcher(got, cid):
            then.append(step("record", alias, cid))
        else:
            then.append(step("refused", alias, cid))
    return first + then


def plan_all(conn: sqlite3.Connection, account: str | None = None, drop: frozenset[str] = frozenset()) -> list[Step]:
    """Plan for one account (id or folder) or for every confirmed Claude account, plus every ignored
    Claude account that still holds registrations (they are removed)."""
    if account is not None:
        ids = [accounts._find(conn, account)]
    else:
        ids = [r[0] for r in conn.execute(
            "SELECT id FROM agent_accounts WHERE provider = 'claude' AND (status = 'confirmed' OR (status = 'ignored' "
            "AND id IN (SELECT account_id FROM account_registrations))) ORDER BY id"
        )]  # fmt: skip
    return [s for i in ids for s in plan(conn, i, drop)]


def _snippet(s: Step, launcher: str) -> str:
    prefix = (f"CLAUDE_CONFIG_DIR={shlex.quote(s.config_dir_env)}" if s.config_dir_env is not None
              else "env -u CLAUDE_CONFIG_DIR")  # fmt: skip
    if s.action == "add":
        command = f"mcp add-json {shlex.quote(s.alias)} {shlex.quote(writer.add_json(s.connector_id, launcher))}"
    else:
        command = f"mcp remove {shlex.quote(s.alias)}"
    return f"{prefix} claude {command} --scope user"


def apply(conn: sqlite3.Connection, steps: list[Step], out: writer.Out) -> int:
    """Run the steps in order; record each one right after it succeeds; stop at the first failure."""
    work = [s for s in steps if s.action in APPLIED]
    if not work:
        out("nothing to change.")
        return 0
    launcher = writer.launcher_path() if any(s.action == "add" for s in work) else ""
    if not shutil.which("claude"):
        out("the `claude` CLI is not on PATH; nothing was written. Run these once it is installed:")
        for s in work:
            if s.action in ("add", "remove"):
                out(f"  {_snippet(s, launcher)}")
        return 0
    for s in work:
        env = writer.account_env(s.config_dir_env)
        done = None
        if s.action == "add":
            done = writer.add_entry(s.alias, s.connector_id, launcher, env)
        elif s.action == "remove":
            done = writer.remove_entry(s.alias, env)
        if done is not None and done.returncode != 0:
            out(f"failed: {s.line()} (`claude` exit {done.returncode}); stopped here.")
            if done.stderr.strip():
                out(done.stderr.strip())
            out("run `easy-personal connector sync` again to finish.")
            return 1
        with conn:  # one small transaction per step: registrations match what succeeded
            if s.action in ("add", "record"):
                conn.execute("INSERT OR REPLACE INTO account_registrations VALUES (?, ?, ?)",
                             (s.account_id, s.alias, s.connector_id))  # fmt: skip
            else:
                conn.execute("DELETE FROM account_registrations WHERE account_id = ? AND alias = ?",
                             (s.account_id, s.alias))  # fmt: skip
        out(f"done: {s.line()}")
    out("restart Claude Code in the changed accounts to use the change.")
    return 0


def remove_connector(
    conn: sqlite3.Connection, connector_id: str, confirm: writer.Confirm, out: writer.Out, assume_yes: bool = False
) -> int | None:
    """Unregister a connector from every account, then delete its keychain item and row.

    None when `connector_id` is not a stored connector (the caller keeps the pre-workspace path).
    The keychain item and the row are deleted only once no account holds a registration.
    """
    row = conn.execute("SELECT alias FROM connectors WHERE id = ?", (connector_id,)).fetchone()
    if row is None:
        return None
    holders = [r[0] for r in conn.execute(
        "SELECT DISTINCT account_id FROM account_registrations WHERE connector_id = ? ORDER BY account_id",
        (connector_id,),
    )]  # fmt: skip
    steps = [s for a in holders for s in plan(conn, a, frozenset({connector_id}))
             if s.connector_id == connector_id or s.action == "skipped"]  # fmt: skip
    out(f"will change (connector {connector_id}, alias {row[0]}):")
    for s in steps:
        out(f"  {s.line()}")
    out(f"  keychain: delete service easy-personal/{connector_id}")
    out(f"  store: delete connector {connector_id}")
    if any(s.action == "skipped" for s in steps):
        out("an account that holds it can not be changed now; nothing was removed.")
        return 1
    if not assume_yes and not confirm():
        out("aborted; nothing was removed.")
        return 1
    if steps:
        rc = apply(conn, steps, out)
        if rc:
            return rc
    left = conn.execute(
        "SELECT COUNT(*) FROM account_registrations WHERE connector_id = ?", (connector_id,)
    ).fetchone()[0]
    if left:  # `claude` missing: only snippets were printed
        out("the connector stays until every account has dropped it; run this again afterwards.")
        return 0
    keychain.remove(connector_id)
    with conn:
        conn.execute("DELETE FROM connectors WHERE id = ?", (connector_id,))
    out(f"connector {connector_id} removed.")
    return 0


def doctor(conn: sqlite3.Connection, out: writer.Out) -> int:
    """Per account: each registered alias is present and points at `easy-personal-launch exec <id>`;
    each stored connector has its keychain item. Reports only; never removes or writes anything."""
    bad = 0

    def report(ok: bool, what: str, detail: str) -> None:
        nonlocal bad
        bad += not ok
        out(f"{'ok' if ok else 'FAIL'} {what}: {detail}")

    for (cid,) in conn.execute("SELECT id FROM connectors ORDER BY id").fetchall():
        try:
            if not writer._keychain_item(cid):
                report(False, cid, "no keychain item; remove and add the connector again")
        except LaunchError:
            pass  # the keychain line above already failed
    if not shutil.which("claude"):
        return bad
    rows = conn.execute(
        "SELECT a.id, a.label, a.folder, a.config_dir_env, r.alias, r.connector_id "
        "FROM account_registrations r JOIN agent_accounts a ON a.id = r.account_id ORDER BY a.id, r.alias"
    ).fetchall()
    for _, label, folder, config_dir_env, alias, cid in rows:
        name = f"{label or Path(folder).name} {alias}"
        if not Path(folder).is_dir():
            report(False, name, "account folder missing")
            continue
        got = writer._entry_exists(alias, writer.account_env(config_dir_env))
        if got is None:
            report(False, name, "drift: missing from Claude Code; `connector sync` re-adds it")
        elif not writer.points_at_launcher(got, cid):
            report(False, name, f"drift: does not point at easy-personal-launch exec {cid}; `connector sync` replaces it")
        else:
            report(True, name, f"points at the launcher (exec {cid})")
    return bad
