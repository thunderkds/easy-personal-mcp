# 0001. Native runtime and the connector write targets

**Status**: Accepted (amended 2026-10-21 after the Phase 0 recheck)
**Date**: 2026-10-21
**Deciders**: thunderkds
**Related**: PRD FR-017–FR-021, FR-023, NFR-001, NFR-007, NFR-008 · BRAINSTORMING_LOG.md (Investigation D2–D5)

---

## Context

The Easy Kit plug-ins share two conventions: they ship as Docker services, and they write only to
`reports/`. easy-personal-mcp breaks both, for reasons that are hard to reverse and surprising without
context (2-of-3 gate: hard to reverse, surprising, real trade-off — all three hold; flagged as
ADR-eligible, kept as a DDR unless the owner asks to escalate).

- Agent clients (Claude Code, Codex, Cursor) start each MCP server by running a command **on the host**.
  The connector launcher (`easy-personal-launch exec`) must be that command, so it must run natively.
  *Amended 2026-10-22 (owner):* the registered command is the separate `easy-personal-launch` console script, not `easy-personal exec`: importing the full CLI costs ~300 ms (HUN-44 measurement), over NFR-006's 200 ms; `easy_personal.launch` imports in ~50 ms.
- Secrets live in the OS keychain. Python `keyring` cannot reach it from Docker without `--privileged`.
- Connectors only work if their entries exist in each client's user-level config, so the tool must write there.

---

## Decision

1. **easy-personal-mcp runs natively** (`uv tool install`) as its primary mode. A Docker image may be
   offered later for the analytics/dashboard half only, with `~/.claude` mounted read-only.
2. Besides `~/.easy-personal/` and the reports dir, the tool may write **only** to:
   - the OS keychain, under service names prefixed `easy-personal/`;
   - Claude Code user-scope MCP config, **only via** `claude mcp add-json --scope user` / `claude mcp remove --scope user`;
   - ~~Codex user config~~ and ~~`~/.cursor/mcp.json`~~ — **removed 2026-10-21**: v1 targets Claude Code only (PRD Out of Scope).
3. Every write is user-initiated from the CLI or the on-demand dashboard, shows what will change, and needs confirmation.
   No MCP tool can trigger a write. Dashboard writes require the per-launch token and a matching `Host`/`Origin` (NFR-008).
4. Project-scope files (`.mcp.json`, `.claude/settings.json`, hooks) are never written; they belong to the kernel's `setup.sh`.
5. When a CLI is missing or a precondition fails, the tool shows the snippet and writes nothing.

---

## Alternatives Considered

| Alternative | Pros | Cons | Why not chosen |
|-------------|------|------|----------------|
| Native + write via official CLIs (chosen) | Secrets never in plaintext; config written by the tool that owns it | Deviates from family Docker parity; depends on CLI flags | **Selected** |
| Docker-only like siblings | Family parity, isolation | No keychain, cannot be the launcher → secrets would sit in plaintext env files | Breaks the no-secrets rule |
| Edit `~/.claude.json` directly | No CLI dependency | File also holds sign-in and project trust; Claude Code says it writes it for itself | A bad write breaks Claude Code |
| Snippet only, never write | Zero write risk | Manual per agent; defeats "set up once, works in all three" (US-009) | Kept as the fallback only |

---

## Consequences

### Positive
- Connector secrets never appear in any config file or dotfile.
- One connector setup is registered in Claude Code without hand-editing config.

### Negative (accepted trade-offs)
- First Easy Kit plug-in without Docker as the primary mode.
- The launcher sits in every connector's start path; a broken install breaks all connectors at once.
- Headless Linux needs an unlocked Secret Service; otherwise connectors refuse to configure.

### Follow-up
- [x] ~~v1.1: verify the Codex CLI add command and Cursor header interpolation~~ — dropped with the Codex/Cursor writers.
- [ ] M3: `easy-personal doctor` checks keychain backend, CLIs on PATH, and that each connector entry still points to the launcher.
