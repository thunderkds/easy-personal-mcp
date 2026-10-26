import argparse
import getpass
import json
import os
import sys
from datetime import datetime
from importlib.metadata import version
from pathlib import Path

from easy_personal import core
from easy_personal.core import (
    accounts,
    config,
    kit,
    lock,
    metrics,
    plan,
    rules,
    schedule,
    store,
    workspaces,
)
from easy_personal.sources import claude_code, context, kit_trace, ui, verifier
from easy_personal.surfaces import mcp as mcp_surface


def iso_datetime(value: str) -> str:
    datetime.fromisoformat(value)  # raises ValueError -> argparse usage error
    return value


def positive_int(value: str) -> int:
    number = int(value)
    if number < 1:
        raise ValueError("must be at least 1")
    return number


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="easy-personal")
    parser.add_argument("--version", action="version", version=version("easy-personal-mcp"))
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("ping", help="print the shared core status")
    mcp_p = sub.add_parser("mcp", help="serve the MCP server (stdio by default)")
    mcp_p.add_argument("--transport", choices=["stdio", "http"], default="stdio")
    mcp_p.add_argument("--host", default=mcp_surface.LOOPBACK)
    mcp_p.add_argument("--port", type=int, default=8765)
    ingest_p = sub.add_parser(
        "ingest", help="copy new Claude Code transcript metadata to the store"
    )
    ingest_p.add_argument(
        "--source", type=Path, help="transcript root of the default account (default ~/.claude/projects)"
    )
    ingest_p.add_argument(
        "--retention-days",
        type=positive_int,
        help="delete events older than N days (default: keep all)",
    )
    schedule_p = sub.add_parser("schedule", help="run ingest on a systemd user timer")
    schedule_sub = schedule_p.add_subparsers(dest="action", required=True)
    install_p = schedule_sub.add_parser("install", help="install and enable the daily timer")
    install_p.add_argument("--retention-days", type=positive_int, help="passed to scheduled ingest")
    install_p.add_argument("--dry-run", action="store_true", help="print the units, write nothing")
    remove_p = schedule_sub.add_parser("remove", help="disable and delete the timer")
    remove_p.add_argument(
        "--dry-run", action="store_true", help="print what would go, change nothing"
    )
    usage_p = sub.add_parser("usage", help="usage summary for a period (same as MCP usage_summary)")
    usage_p.add_argument("--since", type=iso_datetime, help="ISO date/datetime, inclusive")
    usage_p.add_argument("--until", type=iso_datetime, help="ISO date/datetime, exclusive")
    usage_p.add_argument(
        "--by", choices=metrics.DIMENSIONS, help="break tokens down by this dimension"
    )
    usage_p.add_argument("--workspace", help="only events under this workspace's repo paths")
    usage_p.add_argument("--account", help="only events of this agent account (id or label)")
    outliers_p = sub.add_parser("outliers", help="long or tool-heavy sessions (session_outliers)")
    outliers_p.add_argument("--since", type=iso_datetime, help="ISO date/datetime, inclusive")
    outliers_p.add_argument("--until", type=iso_datetime, help="ISO date/datetime, exclusive")
    outliers_p.add_argument("--workspace", help="only events under this workspace's repo paths")
    outliers_p.add_argument("--account", help="only events of this agent account (id or label)")
    workspace_p = sub.add_parser("workspace", help="create, list, edit or delete workspaces")
    workspace_sub = workspace_p.add_subparsers(dest="action", required=True)
    workspace_sub.add_parser("list", help="list workspaces with their repo paths")
    for action in ("add", "edit"):
        action_p = workspace_sub.add_parser(action, help=f"{action} a workspace")
        action_p.add_argument("name")
        for field in ("color", "icon", "description", "email"):
            action_p.add_argument(f"--{field}", help="empty value clears it" if action == "edit" else None)
        if action == "add":
            action_p.add_argument("--repo", action="append", default=[], help="repo path (repeatable)")
        else:
            action_p.add_argument("--rename", help="new name; the workspace id stays")
            action_p.add_argument("--yes", action="store_true", help="skip the confirmation question")
            action_p.add_argument("--add-repo", action="append", default=[], help="repeatable")
            action_p.add_argument("--remove-repo", action="append", default=[], help="repeatable")
    delete_p = workspace_sub.add_parser("delete", help="delete a workspace; its connectors move to orphan")
    delete_p.add_argument("name")
    delete_p.add_argument("--yes", action="store_true", help="skip the confirmation question")
    sub.add_parser("warnings", help="declared usage rules that fired (same as MCP warnings)")
    sub.add_parser("kit", help="what the Easy Kit did per project (same as MCP kit_summary)")
    sub.add_parser("plan", help="improvement plan by priority (same as MCP improvement_plan)")
    sub.add_parser("dashboard", help="ingest, then serve the local dashboard until Ctrl-C")
    for name, text in (("exec", "run a connector's provider server with keychain secrets"),
                       ("headers", "print a connector's JSON headers from the keychain")):
        sub.add_parser(name, help=text).add_argument("connector")
    connector_p = sub.add_parser("connector", help="add or remove a provider connector")
    connector_sub = connector_p.add_subparsers(dest="action", required=True)
    connector_sub.add_parser("list", help="list the provider catalog by category (no secret values)")
    add_p = connector_sub.add_parser("add", help="store a connector in a workspace (secrets in the keychain)")
    add_p.add_argument("provider")
    add_p.add_argument("--workspace", required=True, help="workspace name (see `workspace list`)")
    add_p.add_argument("--alias", help="Claude Code name (default <workspace>-<provider>)")
    remove_p = connector_sub.add_parser("remove", help="unregister a connector from every account, delete it")
    remove_p.add_argument("name", help="connector id (or a pre-workspace name such as slack)")
    sync_p = connector_sub.add_parser("sync", help="show and apply each Claude account's add/remove plan")
    sync_p.add_argument("--account", help="account id or folder (default: every confirmed Claude account)")
    move_p = connector_sub.add_parser("move", help="move a connector to another workspace (orphan allowed)")
    move_p.add_argument("id", help="connector id")
    move_p.add_argument("workspace", help="target workspace name")
    alias_p = connector_sub.add_parser("alias", help="set a custom Claude Code name, or --default to reset it")
    alias_p.add_argument("id", help="connector id")
    alias_p.add_argument("alias", nargs="?", help="new alias")
    alias_p.add_argument("--default", action="store_true", help="reset to <workspace>-<provider>")
    adopt_p = connector_sub.add_parser("adopt-legacy", help="move pre-workspace connectors (slack, ...) into orphan")
    for action_p in (add_p, remove_p, sync_p, move_p, alias_p, adopt_p):
        action_p.add_argument("--yes", action="store_true", help="skip the confirmation question")
    account_p = sub.add_parser("account", help="agent accounts (~/.claude*, ~/.codex*): list, confirm, ignore, add")
    account_sub = account_p.add_subparsers(dest="action", required=True)
    list_p = account_sub.add_parser("list", help="detect new folders, then list every account")
    list_p.add_argument("--by", choices=["account", "provider"], default="account", help="grouping")
    confirm_p = account_sub.add_parser("confirm", help="name an account and mark it confirmed")
    confirm_p.add_argument("account", help="account id or folder")
    confirm_p.add_argument("--label", required=True, help="1 to 60 characters, e.g. 'Claude work'")
    account_sub.add_parser("ignore", help="hide an account from re-detection").add_argument(
        "account", help="account id or folder"
    )
    account_sub.add_parser("add", help="add an agent folder by path").add_argument("path")
    use_p = account_sub.add_parser("use", help="set the workspaces a Claude account picks (none clears)")
    use_p.add_argument("account", help="account id or folder")
    use_p.add_argument("workspaces", nargs="*", help="workspace names; `connector sync` applies them")
    sub.add_parser("doctor", help="check keychain, claude CLI and connector entries")
    return parser


def _print_json(value) -> None:
    print(json.dumps(value, indent=2, sort_keys=True))


def _ingest(args) -> int:
    timeout = float(os.environ.get("EASY_PERSONAL_LOCK_TIMEOUT", lock.DEFAULT_TIMEOUT))
    try:
        with (
            lock.ingest_lock(config.data_dir() / "ingest.lock", timeout),
            store.opened(config.store_path()) as conn,
        ):
            accounts.detect(conn)
            result = claude_code.ingest_accounts(conn, args.source)
            result["kit_trace"] = kit_trace.ingest(conn)  # reads projects the line above stored
            result["context"] = context.ingest(conn)  # never fails the run (FR-012)
            result["ui"] = ui.ingest(conn)  # never fails the run (FR-013)
            result["verifier"] = verifier.ingest(conn)  # never fails the run (FR-011)
            if args.retention_days is not None:
                result["pruned"] = store.prune(conn, args.retention_days)
    except lock.LockBusy:
        print("another ingest is running; try again later", file=sys.stderr)
        return 75
    _print_json(result)
    return 0


def _ask(label: str, secret: bool) -> str:
    return getpass.getpass(f"{label}: ") if secret else input(f"{label}: ").strip()


def _confirm_text(question: str) -> bool:
    """Ask yes/no; closed stdin (EOF) or Ctrl-C counts as no, so nothing is written."""
    try:
        answer = input(f"{question} [y/N] ")
    except (EOFError, KeyboardInterrupt):
        print()  # end the prompt line
        return False
    return answer.strip().lower() in ("y", "yes")


def _confirm() -> bool:
    return _confirm_text("Apply these changes?")


def _connector_list() -> int:
    from easy_personal.connectors import catalog

    entries = sorted((catalog.load(i) for i in catalog.ids()),
                     key=lambda e: (e.category, not e.claude_ai_connector, e.provider))
    category = None
    for e in entries:
        if e.category != category:
            category = e.category
            print(category)
        official = f"claude.ai connector: yes ({e.claude_ai_link})" if e.claude_ai_connector \
            else "claude.ai connector: no"
        print(f"  {e.id} {e.provider} | {official}")
        print(f"    fields: {', '.join(f.label for f in e.fields)}")
    with store.opened(config.store_path()) as conn:
        rows = conn.execute(
            "SELECT c.id, c.alias, c.provider, w.name FROM connectors c "
            "JOIN workspaces w ON w.id = c.workspace_id ORDER BY w.name, c.alias"
        ).fetchall()
    if rows:
        print("configured (id alias provider workspace)")
    for row in rows:
        print("  " + " ".join(row))
    return 0


def _sync(conn, args) -> int:
    from easy_personal.connectors import sync

    accounts.detect(conn)
    steps = sync.plan_all(conn, args.account)
    for step in steps:
        print(step.line())
    if not any(s.action in sync.APPLIED for s in steps):
        print("nothing to change.")
        return 0
    if not args.yes and not _confirm():
        print("aborted; nothing was written.")
        return 1
    return sync.apply(conn, steps, print)


def _connector(args) -> int:
    from easy_personal.connectors import catalog, legacy, sync, writer
    from easy_personal.launch import LaunchError

    if args.action == "list":
        return _connector_list()
    try:
        with store.opened(config.store_path()) as conn:
            if args.action == "add":
                entry = catalog.load(args.provider)
                return writer.add_to_workspace(
                    conn, entry, args.workspace, args.alias, _ask, _confirm, print, args.yes
                )
            if args.action == "sync":
                return _sync(conn, args)
            if args.action == "move":
                return sync.move_connector(conn, args.id, args.workspace, _confirm, print, args.yes)
            if args.action == "alias":
                if args.default == (args.alias is not None):
                    raise writer.ConnectorError("give either a new alias or --default")
                return sync.set_alias(conn, args.id, None if args.default else args.alias, _confirm, print, args.yes)
            if args.action == "adopt-legacy":
                return legacy.adopt(conn, _confirm, print, args.yes)
            rc = sync.remove_connector(conn, args.name, _confirm, print, args.yes)
        if rc is not None:
            return rc
        return writer.remove(args.name, _confirm, print, args.yes)  # a pre-workspace connector
    except (catalog.CatalogError, writer.ConnectorError, workspaces.WorkspaceError, accounts.AccountError) as exc:
        print(f"easy-personal: {exc}", file=sys.stderr)
        return 2
    except LaunchError as exc:
        print(f"easy-personal: {exc}", file=sys.stderr)
        return 1


def _account(args) -> int:
    with store.opened(config.store_path()) as conn:
        try:
            if args.action == "list":
                _print_json(accounts.grouped(conn, args.by))
                return 0
            accounts.detect(conn)
            if args.action == "confirm":
                _print_json(accounts.confirm(conn, args.account, args.label))
            elif args.action == "ignore":
                _print_json(accounts.ignore(conn, args.account))
            elif args.action == "use":
                _print_json(accounts.use(conn, args.account, args.workspaces))
                print("run `easy-personal connector sync` to apply it to the account's Claude config.", file=sys.stderr)
            else:
                _print_json(accounts.add(conn, args.path))
        except accounts.AccountError as exc:
            print(f"easy-personal: {exc}", file=sys.stderr)
            return 2
    return 0


def _workspace(args) -> int:
    """`workspace` subcommands; a refused operation prints its reason on stderr and exits 2."""
    from easy_personal.connectors import sync, writer

    texts = {f: getattr(args, f, None) for f in ("color", "icon", "description", "email")}
    try:
        with store.opened(config.store_path()) as conn:
            if args.action == "list":
                _print_json({"workspaces": workspaces.list_all(conn)})
            elif args.action == "add":
                _print_json(workspaces.add(conn, args.name, repos=args.repo, **texts))
            elif args.action == "edit":
                current = workspaces.by_name(conn, args.name)
                changes = {f: (v or None) for f, v in texts.items() if v is not None}
                if args.rename is not None:
                    changes["name"] = args.rename
                if args.add_repo or args.remove_repo:
                    drop = {workspaces.normalise_repo(r) for r in args.remove_repo}
                    keep = [r for r in current["repos"] if r not in drop]
                    changes["repos"] = [*keep, *args.add_repo]
                if changes.get("name") == current["name"]:
                    del changes["name"]
                if "name" in changes:  # default aliases follow the name: show the plan, confirm
                    rc = sync.rename_workspace(conn, current["id"], changes, _confirm, print, args.yes)
                    if rc == 0:
                        _print_json(workspaces.get(conn, current["id"]))
                    return rc
                _print_json(workspaces.edit(conn, current["id"], changes))
            else:
                current = workspaces.by_name(conn, args.name)
                return sync.delete_workspace(conn, current["id"], _confirm, print, args.yes)
    except (workspaces.WorkspaceError, writer.ConnectorError) as exc:
        print(f"easy-personal: {exc}", file=sys.stderr)
        return 2
    return 0


def _schedule(args) -> int:
    if args.action == "remove":
        if args.dry_run:
            print(f"would disable {schedule.TIMER} and delete it and {schedule.SERVICE} "
                  f"from {schedule.unit_dir()}")  # fmt: skip
            return 0
        print(f"removed {schedule.TIMER} from {schedule.remove()}")
        return 0
    cmd = schedule.command()
    if args.dry_run:
        for name, text in schedule.render_units(cmd, args.retention_days).items():
            print(f"# {name}\n{text}")
        return 0
    if not schedule.systemd_available():
        print("no systemd user session found; nothing written. Use cron instead:")
        print(schedule.cron_snippet(cmd, args.retention_days))
        return 1
    print(f"installed {schedule.TIMER} in {schedule.install(cmd, args.retention_days)}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command == "ping":
        print(core.ping())
        return 0
    if args.command in ("exec", "headers"):
        from easy_personal import launch

        return launch.main([args.command, args.connector])
    if args.command == "connector":
        return _connector(args)
    if args.command == "account":
        return _account(args)
    if args.command == "doctor":
        from easy_personal.connectors import legacy, sync, writer

        rc = writer.doctor(print)
        with store.opened(config.store_path()) as conn:
            legacy.doctor(conn, print)
            return 1 if sync.doctor(conn, print) or rc else 0
    if args.command == "ingest":
        return _ingest(args)
    if args.command == "schedule":
        return _schedule(args)
    if args.command == "workspace":
        return _workspace(args)
    if args.command == "dashboard":
        from easy_personal.surfaces import api

        print("ingesting before start (waits if another ingest runs)...", file=sys.stderr)
        rc = _ingest(argparse.Namespace(source=None, retention_days=None))
        if rc:
            return rc
        api.serve()
        return 0
    if args.command in ("usage", "outliers"):
        try:
            with store.opened(config.store_path()) as conn:
                period = (args.since, args.until)
                filters = {"workspace": args.workspace, "account": args.account}
                if args.command == "outliers":
                    _print_json(metrics.session_outliers(conn, *period, **filters))
                elif args.by:
                    _print_json(metrics.usage_breakdown(conn, args.by, *period, **filters))
                else:
                    _print_json(metrics.usage_summary(conn, *period, **filters))
        except (workspaces.WorkspaceError, accounts.AccountError) as exc:  # unknown workspace or account
            print(f"easy-personal: {exc}", file=sys.stderr)
            return 2
        return 0
    if args.command == "kit":
        with store.opened(config.store_path()) as conn:
            _print_json(kit.kit_summary(conn))
        return 0
    if args.command in ("warnings", "plan"):
        try:
            with store.opened(config.store_path()) as conn:
                _print_json(
                    rules.evaluate(conn) if args.command == "warnings" else plan.improvement_plan(conn)
                )
        except ValueError as exc:  # bad thresholds in config.yaml
            print(f"easy-personal: {exc}", file=sys.stderr)
            return 2
        return 0
    try:
        mcp_surface.validate_host(args.host)
    except ValueError as exc:
        parser.error(str(exc))
    mcp_surface.serve(args.transport, args.host, args.port)
    return 0


if __name__ == "__main__":
    sys.exit(main())
