> Board ID: HUN-245. Created from the draft on 2026-11-09. Rewritten 2026-11-15 for the owner's copy model.
> Split out of HUN-230 note 3, "button to send command to claude", at Stage 2 on 2026-11-08.

**Date**: 2026-11-15
**Complexity Level**: C2
**Risk Level**: Medium
**Priority**: P2
**Assigned agent**: frontend-developer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

1. Read `PROJECT_SPEC.md`
2. Read the memory slice in your spawn prompt
3. Read this file completely
4. Read `agents/frontend.md`
5. Read `PRD.md` FR-092 (rewritten 2026-11-15), US-038, NFR-002, open question 34
6. Read `web/src/WorkspacesView.jsx` (how a workspace card shows its repos; `getJson('/api/workspaces')`), `web/src/WorkspacesView.test.jsx` (test style), `web/src/test/fixtures.js`, the design tokens at the top of `web/src/app.css`, and `docs/design/graphite-mint/` for the card and button styles
- Do not open `~/.easy-personal`, `~/.claude*` or `~/.codex*`

---

## Requirement

**Original request** (owner, 2026-11-08): "button to send command to claude". **Execution model chosen by the owner 2026-11-15**: copy ("as your recommend, let continue", answering the recommendation "copy: the button copies a fixed command, you paste it into your own session; it runs nothing"). Recorded on `main` in `ad5d88b`.

**Restated intent** (PRD FR-092):
> 1. On the Workspaces page, each workspace card that has at least one stored repo gets a small **Copy a Claude command** block: a select of the fixed allow-list, a select of that workspace's repos (shown only when it has more than one), and a **Copy** button.
> 2. The allow-list is held in code (a constant array), never typed by the user (⚠ decided for the owner, who may change it): `Start a session` → `claude "/wake"`; `Continue the last session` → `claude --continue`; `Verify` → `claude "/verify"`.
> 3. Copy puts exactly one line on the clipboard: `cd -- '<repo>' && <command>` (the `--` ends the options, so a path that starts with `-` is not read as one; Supervisor decision 2026-11-15). The repo path is wrapped in single quotes and each `'` inside it is written `'\''` (POSIX shell). A repo path that holds any C0 control character (U+0000-U+001F) or U+007F offers no Copy: a fixed note "This repo path has a control character; copy is off." The line is built by one pure function with its own tests.
> 4. After Copy: a short status "Copied" (polite live region). When `navigator.clipboard.writeText` is missing or refuses, the line is shown in a read-only `<input>`, selected, with the note "Copy it with Ctrl+C."
> 5. Nothing is sent to the server, stored or logged: no new route, no `fetch`, no `localStorage`. The page never runs anything.

**Out of scope**: the run and queue models; any backend (Python or Rust); a free-text command; editing the allow-list in the UI; `web/src/App.jsx` and the nav; other views.

### Requirement Fidelity Gate
- [x] Restated intent confirmed by the owner (copy model, 2026-11-15)
- [x] Every Acceptance Criterion traces to the Requirement
- [x] Requirement Refs: FR-092, US-038, NFR-002, question 34 in `PRD.md`

---

## Dependencies & Reachability
**Depends on**: none
**Entry point**: `CopyCommand` rendered by `web/src/WorkspacesView.jsx`

## Dependency rule
No new npm package.

---

## Acceptance Criteria

| # | Criterion (testable) | Traces to |
|---|---|---|
| 1 | `shellLine(repo, command)` unit tests: plain path; path with a space; path with `'`; path with `$(…)`, backtick, `;`, `&&` (all stay inside the quotes, byte-exact expected strings); paths `-x` and `--help` (byte-exact `cd -- '-x' && …`); path with `\n`, `\0`, `\t`, U+007F → refused | intent 3 |
| 2 | Component tests: a workspace with 0 repos shows no block; with 1 repo no repo select; with 2 repos a repo select; each allow-list entry copies its exact line (`navigator.clipboard.writeText` mocked, argument asserted) | intents 1-3 |
| 3 | Clipboard refused (mock rejects) → read-only input holding the line, selected, plus the note | intent 4 |
| 4 | A test asserts no `fetch` call and no `localStorage` write happen on Copy | intent 5 |
| 5 | `npm --prefix web test` and `npm --prefix web run build` pass | all |
| 6 | Real browser check at 1280x720 and 390x844 against the real server (Rust `serve --static-dir` with a temp store holding a workspace with 2 repos, one path with a space and a `'`): screenshots, clipboard text read back | UI |
| 7 | `security-review` at Stage 4 (Risk Medium: text for a shell) | intent 3 |

### UI / Design Acceptance Criteria
- Uses the existing Graphite Mint tokens and the existing button/select styles (no new colours); CSS only in a new block at the END of `web/src/app.css` headed `/* HUN-245 */`
- Keyboard: every control reachable by Tab, Copy works with Enter/Space, visible focus
- 390 px wide: the block wraps, no sideways scroll

---

## Evaluation & Acceptance

### Verification Command
```bash
npm --prefix web test -- --run 2>&1 | tail -5 && npm --prefix web run build 2>&1 | tail -3
```

### Evidence (filled by reviewer at Stage 4/5)
> Moved to `tasks/TASK_REVIEW_HUN-245.md`.

## Demonstration
> **Moved.** BEFORE/AFTER/DELTA/WITNESS live in `tasks/TASK_REVIEW_HUN-245.md`. BEFORE: a screenshot of a workspace card today (no copy block) at 1280x720.

---

## Approach
Test first (`tdd`): `shellLine` and its tests, then the component, then the mount in the workspace card. Vital slice: the pure function, one component, one mount line.

## Edge Case Checklist
- Orphan workspace (no repos by definition) → no block
- A repo path with a trailing `/` → copied as stored
- Two cards on one page → each block's state is its own

## Files to Change (Predicted)
`web/src/CopyCommand.jsx` (new: allow-list, `shellLine`, component), `web/src/CopyCommand.test.jsx` (new), `web/src/WorkspacesView.jsx` (one import, one mount in the card), `web/src/app.css` (new block at the end), `web/src/test/fixtures.js` (additions at the end only, if needed), `tasks/TASK_REVIEW_HUN-245.md`

## Files Must NOT Touch
`web/src/App.jsx`, the nav (`Rail.jsx`), `web/src/WatchView.jsx`, any backend file (`src/easy_personal/**`, `rust/**`), `PRD.md`, any `PROJECT_KANBAN.md`, `memory/**`

## Test Plan
The ACs above, each as an automated test or a pasted run, plus the verification command's real output.

## Completion Checklist
- [ ] Every AC with pasted output in `tasks/TASK_REVIEW_HUN-245.md`
- [ ] Commits on branch `hun-245-copy-command` only; final report with `Status:`
