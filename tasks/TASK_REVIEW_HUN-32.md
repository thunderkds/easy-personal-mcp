# TASK_REVIEW — HUN-32: Spike transcript shapes and fixtures

> Sibling of `tasks/TASK_GUIDE_HUN-32.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | pass | `tests/fixtures_check/test_no_real_content.py` (AC2, AC3); AC1/AC4 covered by `docs/spikes/transcript-shapes.md` (manual review) |
| Verification command run | pass | 2026-10-21T09:02:39Z `python3 -m pytest tests/fixtures_check -q` (uv run also works): `5 passed`. Real-id grep over tests/ and docs/ (ids from scan, kept outside repo): 0 files match |
| Negative cases hold | pass | Mutation: changed a fixture cwd to a /home/ path -> `2 failed, 3 passed` (test_no_home_path_or_username, test_cwd_is_invented); reverted -> 5 passed |
| verify | SKIP | /verify 2026-10-21: no runtime surface (fixtures, spike note, a test, and a throwaway script under experiments/ that ships nowhere). Its acceptance check `python3 -m pytest tests/fixtures_check -q` passed (5 passed) at Stage 4 — pass on the acceptance check, SKIP on runtime |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | pass | 9 new files in `main..spike/HUN-32-transcript-shapes`; code-review 0 P0/P1 (1 P2 owner question, 1 P3) — `reports/code-review_spike-HUN-32-transcript-shapes_20261021T163033.html`; security (inline, built-in unavailable) pass — `reports/security-review_spike-HUN-32-transcript-shapes_20261021T163033.html` (main checkout) |
| Full smoke suite still green (no regression) | pass | no other tests exist yet; only suite is tests/fixtures_check (5 passed) |
| **UI: Visual regression (diff or verdict pasted)** | N/A | pure-backend spike, no UI |
| **UI: Design-system compliance (tokens/colors/typography verified)** | N/A | pure-backend spike, no UI |
| **UI: Responsiveness at target viewports** | N/A | pure-backend spike, no UI |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE**: (captured before any implementation commit)

```
2026-10-21T09:00:31Z
$ ls tests/fixtures docs/spikes 2>&1
ls: cannot access 'tests/fixtures': No such file or directory
ls: cannot access 'docs/spikes': No such file or directory
```

**AFTER**: 2026-10-21 `ls tests/fixtures docs/spikes`:
```
docs/spikes:
transcript-shapes.md

tests/fixtures:
claude_code
```
`python3 -m pytest tests/fixtures_check -q` -> `5 passed`.

**DELTA**: The parser task can now build against per-version synthetic fixtures and a findings note (record types, FR-002 field presence, measured volume, proposed NFR-005), with a test that proves the fixtures hold no real content.

**WITNESS**: qa-expert (HUN-32 session, 2026-10-21); trace attribution via `memory/event-trace` not checked by the implementing agent.
