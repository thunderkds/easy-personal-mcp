**Date**: 2026-11-03
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary; Critical Constraints: metadata only, CSP `default-src 'self'`, connector secrets never echoed)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Read `PRD.md` US-028, FR-023, FR-065 and `web/src/tokens.css` (design tokens)

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner, 2026-11-03 (auto mode): "the UX of the form is not good, let check and update. By the way the notification should show as the float, the loading indicator also."

**Restated intent**:

> Audit and improve the dashboard forms: the workspace form (`WorkspaceForm` in `web/src/WorkspacesView.jsx`), `AddConnector` (same file) and the agent accounts form/actions (`web/src/AgentsView.jsx`). Then make action results and errors float as toasts, and make page/list loading a floating indicator.
>
> **Forms**:
>
> * Every field has a visible label; a hint (`aria-describedby`) where the format matters (Color, Email, Repos = absolute path, Icon = short text).
> * Name is marked required.
> * Colour: an `<input type="color">` next to the `#rrggbb` text input, kept in sync. Empty colour stays allowed.
> * Check fields before review: empty Name, a colour that is not `#rrggbb`, a bad email. Show each error next to its field (`aria-invalid`, error text linked by `aria-describedby`), focus the first bad field, and send no request.
> * Save / Review changes shows a busy label ("Saving…" / "Reviewing…") and is disabled while a request runs.
> * Fields laid out in a tidy grid on wide screens, one column on a phone; action buttons grouped with the primary button first.
> * The Confirm/Cancel plan (`Changes`) stays inline, but next to the form's action buttons (not above the form), and keeps its focus move.
>
> **Floating notifications**: one toast host (`web/src/Toast.jsx`, provider in `App.jsx`, hook `useNotify()`), fixed in the bottom-right corner with a 16 px gutter, full width at the bottom on a phone. Success/info toasts hide after 5 s; error toasts stay until closed; each has a close button; at most 3 (oldest dropped). Live regions: `role="status"` for success, `role="alert"` for errors. Use it for action results and action errors in WorkspacesView (`Changes` result/error) and AgentsView (result/error). A failed result (`ok: false`) is an error toast. Respect `prefers-reduced-motion`.
>
> **Floating loading indicator**: one component (e.g. `web/src/Loading.jsx`) that shows a small fixed pill with a spinner and the existing label text ("Loading usage", "Loading board", …), with `role="status"`. Replace the inline `<p role="status">Loading …</p>` page/list loading lines in App, Board, Agents, FirstRun, Kit, Memory, WarningsPlan, Usage, Jobs and Workspaces views. The Rail card's own "Loading" stays inline (card-local). The LiveView link status line stays as it is (it is a connection state, not loading). A failed page load (`role="alert"` error line) stays inline.

**Out of scope**:

* Any backend or API change
* New write actions or new fields
* Dark/light theme changes beyond using existing tokens
* Persisting toasts across reloads

**Requirement Refs**: US-028, FR-065

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent confirmed to match the user's request (auto (Supervisor), 2026-11-03)
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion below traces to a line in the Requirement
- [X] All Requirement Refs exist in `PRD.md` and are fully covered by the Acceptance Criteria above

---

## Dependencies & Reachability

**Depends on**: None

**Entry point**: `easy-personal dashboard` → Workspaces view (`WorkspaceForm`, `AddConnector`), Agent accounts view; toast host `ToastProvider` in `web/src/App.jsx`

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to requirement |
| -- | -- | -- |
| 1 | Submitting the workspace form with an empty Name shows "Name is required" next to Name, sets `aria-invalid`, focuses Name and sends no request; `#12345` in Color and `a@` in Email give their own field errors the same way | FR-065 |
| 2 | The colour picker and the `#rrggbb` text stay in sync both ways; clearing the text saves `color: null` as today | FR-065 |
| 3 | While a save/review request is pending the button reads "Saving…" / "Reviewing…" and is disabled; it returns after the answer | FR-065 |
| 4 | The Confirm/Cancel plan renders after the form's action buttons and still gets focus | FR-065 |
| 5 | A successful action shows a toast with `role="status"` inside the toast host and it disappears after 5 s (fake timers); a failed action shows a `role="alert"` toast that stays until its close button is pressed; a 4th toast drops the oldest | FR-065 |
| 6 | Action results/errors in WorkspacesView and AgentsView no longer render as inline blocks in the page flow; connector secret values never appear in any toast | FR-065 |
| 7 | Each listed view shows the floating loading indicator (`role="status"`, its label) while loading, and no inline loading paragraph; a failed load still shows the inline `role="alert"` line | FR-065 |
| 8 | Existing view tests still pass (updated only where they queried the old inline text position) | FR-023 |

---

## Evaluation & Acceptance (How we know the agent worked correctly)

> The reviewer fills **Evidence** in `tasks/TASK_REVIEW_<this task ID>.md`. Not done until every row has evidence.

### Verification Command (exact, runnable)

```bash
cd web && npx vitest run && cd .. && npm --prefix web run build && uv run pytest tests/test_web_toolchain.py -q
```

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| Workspaces → New workspace, empty submit, 1280×720 | headless screenshot BEFORE and AFTER | field errors next to fields; tidy grid; busy button |
| A successful and a failed action | headless screenshot | toast floats bottom-right, page does not move |
| Usage view while loading (slow fixture/route) | headless screenshot | floating loading pill, page content does not jump |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors / fonts / radius | reuse `web/src/tokens.css` variables | no hard-coded new colours; dark and light both readable |
| Accessibility | vitest + manual keyboard check | labels, `aria-describedby` hints/errors, `aria-invalid`, toast live regions, close button reachable by keyboard, focus order unchanged |
| Motion | CSS | animation off under `prefers-reduced-motion: reduce` |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 1280×720 and 375×812 | screenshot + `scrollWidth` | no horizontal page scroll; on 375 px the form is one column and toasts span the width with a 16 px gutter without covering the form's buttons permanently (they can be closed) |

---

## Approach

**Vital slice**: Toast host + useNotify → wire Workspaces/Agents results → Loading pill in all views → workspace form validation, hints, colour picker, busy button, plan placement → AddConnector/Agents form polish.
**Cut list**: no toast queue persistence, no undo actions in toasts, no new form fields, no form-library dependency, no new npm package.

---

## Edge Case Checklist

- [ ] Two quick actions → two toasts, newest on top or bottom consistently
- [ ] Toast host unmount during a timer → no state update warning
- [ ] Error text from the API is shown as plain text (no HTML injection)
- [ ] A secret field value is never put in a toast or an error message
- [ ] Loading shorter than a frame → no flicker requirement beyond what is simple; no layout shift
- [ ] Orphan workspace (no form) unaffected
- [ ] Planted secret/text fixtures (from the leak suite) never appear in any store row, MCP/CLI/API response, page or log.

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/Toast.jsx` (new), `web/src/Loading.jsx` (new) | toast host + hook; floating loading pill |
| `web/src/App.jsx` | provider; loading |
| `web/src/WorkspacesView.jsx`, `web/src/AgentsView.jsx` | form UX; toasts |
| `web/src/BoardView.jsx`, `FirstRun.jsx`, `KitView.jsx`, `MemoryView.jsx`, `WarningsPlanView.jsx`, `UsageView.jsx`, `JobsView.jsx` | loading pill |
| `web/src/app.css` | form grid, field hint/error, toast, loading pill styles |
| `web/src/*.test.jsx` | tests |
| `src/easy_personal/dashboard/static/` | rebuilt page (gitignored) |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/core/`, `src/easy_personal/surfaces/` | No backend change |
| `web/src/LiveView.jsx` link status, `web/src/Rail.jsx` card loading | Stay inline by design |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | Kit framework files |

---

## Test Plan

Vitest: validation per field, colour sync, busy label, plan placement, toast timing with fake timers, max 3, alert vs status, loading pill per view; build; headless screenshots (BEFORE now, AFTER at the end) at 1280×720 and 375×812 on a copy of the owner store (never the live store).

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into the review file's Evidence table (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] Supervisor notified: task ready for Stage 4 review