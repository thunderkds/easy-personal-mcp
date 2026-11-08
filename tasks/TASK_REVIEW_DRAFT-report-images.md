# TASK_REVIEW — DRAFT-report-images: an opened report shows its embedded screenshots (FR-094)

> Sibling of `tasks/DRAFT_report_images.md`. Everything here is **filled by the reviewer at Stage
> 4/5** — it is deliberately NOT in the guide, because the implementing agent re-reads the guide on
> every turn and never fills these two sections.
>
> Consumers resolve each section **guide first, this file second** (`.claude/hooks/lib/guide_sections.py`):
> a legacy guide that still carries these sections inline keeps working unchanged, and a stray
> review file can never override an inline section.

---

<!-- Optional: the final report, only for a task run in auto mode (the user's `[auto]` tag,
`docs/claude-md/tracks.md` `## Auto mode`). To use it, copy the block below out of this comment, to this
place, and replace every <...>. Keep `Mode: auto` as its first line, exactly as written: T182's hook reads
it. Write it to this file before the run ends; the chat message is a copy, not the report. The same
block is the Supervisor's final chat message. It is not the `delivery-report` page and not
Evidence. A taste decision (what the user wants) also ends with the flag. Write `DEBATED` only when ≥2
Advocate `Agent` calls ran for that question, naming each Advocate's option. A decision with no Advocates is
labelled `RULE`, `FACT` or `SAFER OPTION`, never `DEBATED`. A decision flagged `⚠ decided for you` is a choice, and is never labelled `FACT`. A
safety action a rule requires (for example the backup before a delete) is `RULE` without the flag.
At Risk Medium or High, a `RULE:` counts only when it names a written rule (`path:line`,
`path § <heading>`, or `docs/claude-md/tracks.md` `### <section>`), and a `FACT:` needs a Citation
(`path:line "quoted text"`); without one it is a choice and must be `DEBATED`. Good: `2. backed up the CSV
— RULE: `docs/claude-md/tracks.md` `### Stops that stay``. Bad: `1. kept recurring.csv — RULE: Surgical Changes`.

## Auto decisions

Mode: auto
1. <what was decided> — RULE: <the rule>
2. <what was decided> — FACT: <path>:<line> "<quoted text>"
3. <what was decided> — DEBATED: <option> (Advocate 1: <its option>; Advocate 2: <its option>) ⚠ decided for you — please review
4. <what was decided> — SAFER OPTION: <why> ⚠ decided for you — please review
Paused: <only when a stop that stays paused the task: which one, and why>
Checks: tests `<command>` — <result>; reviews <skill — result>; real run `<command>` — printed `<what it printed>` (auto-checked, not /verify)
Not done: merge (yours); <auto verify: running (T182) | auto verify: <result> | /verify (yours)>

-->

## Evidence

| Check | Result | Notes / output snippet |
|-------|--------|------------------------|
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☑ pass | `tests/surfaces/test_api.py::test_csp_on_every_response` (exact string, page, /api, 403 token, 403 Host) and `::test_csp_keeps_script_and_egress_on_default_src` (img-src pinned to `["'self'", "data:"]`); guard probe: `img-src *`, `img-src + https://evil.example`, widened connect-src, widened font-src, `script-src 'unsafe-inline'` each FAIL it; unamended baseline passes. `tests/surfaces`: 340 passed |
| Verification command run | ☑ pass | `uv run pytest -q`: 1759 passed, 6 deselected, 1 xfailed; `npx vitest run` (in web/): 24 files, 786 tests passed |
| Negative cases hold | ☑ pass | remote image/font/bg/attr-selector: 0 sink connections, inline script blocked (see AFTER) |
| verify | ☐ pass / ☐ fail / ☐ N/A | [what was observed — must literally state "pass" or "fail" here too, e.g. "skill run, feature confirmed working — pass": the merge gate scans this Notes column for the word "pass", not just the Result column] |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☐ pass / ☐ fail | [what was reviewed vs. skipped, and why] |
| Full smoke suite still green (no regression) | ☐ pass / ☐ fail | |
| **Companion score (easy-verifier)** | ☐ pass / ☐ fail / ☐ N/A | [`overall` and `change_risk` as the tool returned them, the scope, the ref, and the absolute path of the verifier report after a literal `report:` label — e.g. "overall 86 (>= 80), change_risk pass, scope changes, ref `main...HEAD`, report: `/home/me/.easy-verifier-reports/kit/score_changes_20261028T0530Z.html`". The gate reads the path that follows `report:` and nothing else, so with no label, or a path that is absent, the row is unscored — write ☑ N/A instead. ☑ N/A with a reason when neither companion tool was run: the merge gate blocks on ☑ fail only, never on an absent or N/A row] |
| **UI: Visual regression (diff or verdict pasted)** | ☐ pass / ☐ fail / ☐ N/A | [screenshot path or LLM verdict — required for UI tasks, Hard-Stop Gate 6] |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☐ pass / ☐ fail / ☐ N/A | [method used + output] |
| **UI: Responsiveness at target viewports** | ☐ pass / ☐ fail / ☐ N/A | [viewports tested, any overflow findings] |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (captured 2026-11-08T14:46Z on the unchanged code, branch `report-images` at `a34e427`; real headless Chrome through a freshly installed `uv tool install --refresh` tree; harness output saved in `tasks/evidence-DRAFT-report-images/before/`):

```
source CSP line: 61:CSP = "default-src 'self'; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'"
page response CSP: default-src 'self'; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'
shot.html    state: {"sheets":1,"imgs":[[true,0,0]]}   (imgComplete true, naturalWidth 0, naturalHeight 0; PNG is 120x60)
             security log: "Loading the image 'data:image/png;base64,<bytes>"
orderly.html state: {"sheets":1,"imgs":[[true,0,0],[true,0,0]]}   security log: 2 x "Loading the image 'data:image/png;base64,<bytes>"
leak.html    security log: 5 entries (img, font, background-image, attribute-selector image, style attribute image), all refused
evil.html    title "safe"; security log: "Executing inline script violates the following Content Security Policy directive 'default-src 'self''"; /api requests from tab: []
expected PNG 120x60
sink connections: 0 []
```

**AFTER** (2026-11-08T14:48Z, same harness, commit `f4ba405`, fresh `--refresh` install; `tasks/evidence-DRAFT-report-images/after/`):

```
page response CSP: default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'
shot.html    {"sheets":1,"imgs":[[true,120,60]]}   security log: []      (PNG is 120x60)
orderly.html {"sheets":1,"imgs":[[true,1280,720],[true,1280,720]]}   security log: []
leak.html    5 security entries, all refused (img, font, background-image, attribute selector, style attribute); sink connections: 0 []
evil.html    title "safe"; "Executing inline script violates ... 'default-src 'self''"; /api requests from tab: []
usage.png and kit.png byte-identical before vs after (cmp)
```

**DELTA**: An opened report now shows the screenshots embedded in it, while a remote image, font or script is still refused.

**WITNESS**: [who ran it and when — derived from `memory/event-trace/Txxx.jsonl`, never the
implementing agent alone]
