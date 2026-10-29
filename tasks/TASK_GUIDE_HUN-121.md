**Date**: 2026-10-29
**Complexity Level**: C2
**Risk Level**: Low
**Priority**: P1
**Assigned agent**: Frontend-Implementer
**Agent guide**: `agents/frontend.md`

---

## Mandatory Startup (Do Not Skip)

Before writing any code:

1. Read `PROJECT_SPEC.md` (glossary: *Workspace*, *Agent account*, *API-equivalent cost*)
2. Read the memory slice in your spawn prompt (`<!-- memory-slice -->`). Read `memory/MEMORY.md` in full only if your work reaches a file, hook, skill or decision the slice does not cover
3. Read this file completely
4. Read your role guide (`Agent guide` above) with the Read tool and no `limit`
5. Note the **Complexity Level** above and apply the matching process from the Complexity matrix in your role guide
6. Verify on fixtures only: temp `EASY_PERSONAL_HOME` + `EASY_PERSONAL_CLAUDE_PROJECTS` + `EASY_PERSONAL_AGENT_ROOT`; never open the real `~/.easy-personal/` or `~/.claude*/`

---

## Requirement (Pillar 1 — Adapt the requirement)

Owner request 2026-10-29 (during the `[auto]` run): "update the UI like this also file:///home/hungnguyenhuu/workspace/image/1709717801518.png".

The reference image (owner-supplied, look only) is a dark social-feed template: deep indigo/purple page, a monospace font for all text, three columns — a left nav with collapsible groups ("Profile", "Main", "Tools", …) and small colored dots per item; a center column of rounded cards with thin dividers, blue→purple gradient progress bars and round accent buttons; a right rail with a large ring ("75% Completed") and short lists with "N items" headers and ⋯ menus.

**Restated intent** (Supervisor, auto mode):

> Restyle the whole dashboard to the reference look, on top of the HUN-83 dark shell: a new indigo token set (page, card, card-raised, border, text, muted, blue→purple accent gradient, 5 dot colors), one monospace font stack for all text, cards with thin dividers and section headers ("Title (N)"), and gradient progress bars. The sidebar groups its 5 views into collapsible groups — "Insights" (Usage, Warnings & plan, Kit) and "Setup" (Workspaces, Agents) — each item with a colored dot, with the Scope block (HUN-118) at the top. At ≥ 80rem a right rail shows three cards built only from data the views already load: "Cache reuse" ring (cache hit ratio of the current scope, last 7 days), "Next actions (N)" (top 3 warning titles, from HUN-120), and "Plan limits" (5h/7d bars per confirmed account, from HUN-115/116). Below 80rem the rail is hidden; its data stays on the views.

Owner follow-up 2026-10-29: "looks like the timeline" — the center column should read like the reference's **Timeline** feed. So the Usage "Session timeline" becomes a feed of session cards: each card has a round icon with the project initial, the project name (accent) and a one-line summary ("3 models · 42 messages"), the local time right-aligned after a thin divider, the existing session chart as the card body, a gradient token bar (share of the period's tokens), and a footer row of counts (messages · tool calls · tokens · gaps). The Warnings action cards (HUN-120) and Kit project cards (HUN-119) use the same feed-card shape (icon, title, time at right, body, footer).

**Out of scope**: the template's name, avatar, social features (posts, likes, comments, friends, events); a search box; new API routes; light mode.

**Requirement Refs**: owner request 2026-10-29 (reference image 2).

### Requirement Fidelity Gate (sign off BEFORE implementation)

- [X] Restated intent matches the owner's request — auto (Supervisor), 2026-10-29
- [X] Domain terms align with `PROJECT_SPEC.md` glossary
- [X] Every Acceptance Criterion traces to the Requirement

---

## Dependencies & Reachability

**Depends on**: HUN-116, HUN-118, HUN-119, HUN-120 (all restyle the same views; branch from the last of them). **Entry point**: every dashboard view.

---

## Acceptance Criteria

| \# | Criterion (testable) | Traces to |
| -- | -- | -- |
| 1 | `tokens.css` holds one indigo token set incl. accent gradient start/end and 5 dot colors; every color in `app.css` and JSX `fill`/`stroke` comes from a token; text ≥ 4.5:1 on card surfaces | look |
| 2 | One monospace font stack (system monospace, no web font download) for all text | typography |
| 3 | Sidebar: groups "Insights" and "Setup" are `<button aria-expanded>` toggles; each view item has a colored dot; active item `aria-current="page"`; Scope block on top | left nav |
| 4 | Cards: header row "Title (N)" where a count exists, thin dividers between rows; progress bars use the accent gradient | center cards |
| 5 | Right rail at ≥ 80rem with 3 cards (Cache reuse ring, Next actions (N), Plan limits); each card renders its empty state ("no data") without errors; each SVG has `role="img"` + `aria-label` | right rail |
| 6 | Right rail makes no new API route; it reuses `/api/usage/summary`, `/api/warnings`, `/api/accounts` with the current scope | no new API |
| 8 | Usage session timeline renders as feed cards (icon with project initial, title + summary, right-aligned local time, chart body, gradient token-share bar, footer counts); Warnings and Kit cards use the same `FeedCard` component | timeline look |
| 9 | After an account is confirmed, renamed or ignored on Agents, App reloads the scope options, so the sidebar Account list and the ignored-accounts note update without a page reload (HUN-118 review P2) | review carry-over |
| 7 | All existing Vitest tests pass (selectors updated only where the nav markup changes); no new npm dependency | no regression |

---

## Evaluation & Acceptance

### Success Criteria (observable, pass/fail)

| \# | Given | Expect | How checked |
| -- | -- | -- | -- |
| 1 | collapse "Setup" | Workspaces/Agents buttons hidden; `aria-expanded="false"` | Vitest |
| 2 | 2 warning hits | rail "Next actions (2)" lists 2 titles | Vitest |
| 3 | `cache_hit_ratio: null` | ring shows "no data" | Vitest |
| 4 | account limits 92 % | rail bar uses the red tone | Vitest |
| 5 | 2 sessions, 300 and 100 tokens | 2 feed cards; token bars 75 % / 25 %; footer "N messages" | Vitest |

### Verification Command (exact, runnable)

```bash
npm --prefix web test && npm --prefix web run build && uv run pytest -q
```

### Evidence (filled by reviewer at Stage 4/5)

> **Moved.** See `tasks/TASK_REVIEW_HUN-121.md` (review sections are never taken from the board).

## Demonstration

> **Moved.** See `tasks/TASK_REVIEW_HUN-121.md` (review sections are never taken from the board).

## UI / Design Acceptance Criteria

### 1. Visual Regression

| Screen | Method | Expected |
| -- | -- | -- |
| All 5 views at 1440 px | headless Chrome + LLM vision vs reference image 2 | reads like the reference: indigo, monospace, 3 columns, gradient bars, ring in the rail |

### 2. Design-System Compliance

| Criterion | Method | Expected |
| -- | -- | -- |
| Colors from tokens | grep hex/rgb outside `tokens.css` | none |
| Font | computed style | monospace stack everywhere |
| Contrast | computed | ≥ 4.5:1 |

### 3. Layout / Responsiveness

| Viewport | Method | Expected |
| -- | -- | -- |
| 375 px | screenshot | sidebar behind menu; no rail; no horizontal scroll |
| 1024 px | screenshot | sidebar + content; no rail |
| 1440 px | screenshot | 3 columns |

---

## Approach

**Pattern reference**: HUN-83 `tokens.css` / `app.css` / `Widgets.jsx` (`Ring`, `Progress`); `App.jsx` shell.
**Vital slice**: tokens + font + sidebar groups + card header/divider style + rail with 3 cards.
**Cut list**: ⋯ menus; avatars; channel list; animations; per-card settings.

Copy the **look** only, never the template's name, avatar, icons or text.

---

## Edge Case Checklist

- [ ] rail data request fails → that rail card shows "no data", the view still works
- [ ] scope change → rail refetches with the same scope as the view
- [ ] long account labels in the rail → ellipsis, `title` attribute

---

## Files to Change (Predicted)

| File | Change |
| -- | -- |
| `web/src/tokens.css`, `web/src/app.css` | indigo tokens, monospace, cards |
| `web/src/App.jsx` + `.test.jsx` | grouped sidebar, rail slot |
| `web/src/Rail.jsx` + `Rail.test.jsx` (new) | right rail |
| `web/src/Widgets.jsx` | gradient progress |
| `web/src/FeedCard.jsx` (new), `web/src/Timeline.jsx` + `.test.jsx` | feed-card timeline |
| `web/src/WarningsPlanView.jsx`, `web/src/KitView.jsx` | use `FeedCard` |

## Files Must NOT Touch

| File | Reason |
| -- | -- |
| `src/easy_personal/**` (except rebuilt static) | no API change |
| `.claude/`, `agents/`, `skills/`, `templates/`, `scripts/` | kit files |
| `web/package.json` dependencies | no new dependency |

---

## Test Plan

Vitest for AC 3–7; screenshots at 375/1024/1440 for all 5 views.

---

## Completion Checklist

- [ ] Implementation done
- [ ] Tests written AND pass — output pasted into `tasks/TASK_REVIEW_<this task ID>.md` (Hard-Stop Gate 5)
- [ ] UI Evidence rows filled (Hard-Stop Gate 6)
- [ ] `Skill({ skill: "verify" })` run on fixtures
- [ ] Supervisor notified: task ready for Stage 4 review