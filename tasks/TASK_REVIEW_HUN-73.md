# TASK_REVIEW — HUN-73: Release prep v0.2.0

> Sibling of `tasks/TASK_GUIDE_HUN-73.md`. Everything here is **filled by the reviewer at Stage
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
| **New test(s) cover Acceptance Criteria (file paths pasted)** | ☒ pass | `tests/test_readme.py::test_status_names_v0_2_0_not_planning` and `::test_connector_remove_rule_for_pre_workspace_connectors` (written first; 2 failed against the old README, then pass). `tests/test_skeleton.py::test_version_flag_prints_package_version` already checks --version against package metadata. |
| Verification command run | ☒ pass | `uv lock` Resolved 49 packages; readme+site tests `36 passed in 0.57s`; full `746 passed, 2 skipped, 4 deselected, 1 xfailed`; `All checks passed!`; `easy-personal --version` prints `0.2.0` |
| Negative cases hold | ☒ pass | Test asserts `v0.1.0` is absent from README and `Status: planning` is absent. Before the change the new tests failed (2 failed, 12 passed). |
| verify | ☑ pass | Supervisor 2026-10-26: `uv run easy-personal --version` -> `0.2.0`; `uv run pytest tests/test_readme.py tests/test_site_content.py -q` -> 36 passed — pass |
| Review scope bounded to the change's blast radius (affected set, not whole repo) | ☑ pass | Reviewed README (4 lines), pyproject version, uv.lock (own package version only), test_readme.py. No src change — pass |
| Full smoke suite still green (no regression) | ☒ pass | `746 passed, 2 skipped, 4 deselected, 1 xfailed, 1 warning in 94.43s` |
| easy-verifier score (advisory) | ☐ N/A | N/A — easy-verifier MCP server not loaded in this profile |
| **UI: Visual regression (diff or verdict pasted)** | ☑ N/A | N/A — docs + version only |
| **UI: Design-system compliance (tokens/colors/typography verified)** | ☑ N/A | N/A — docs + version only |
| **UI: Responsiveness at target viewports** | ☑ N/A | N/A — docs + version only |

---

## Demonstration

> Anchors what this task delivered to an observable before/after pair. BEFORE has no `N/A` path:
> if the task changes executable code, BEFORE is a pasted, timestamped terminal capture taken
> **before any implementation commit exists**; if it does not (docs, templates, skill-instruction
> text), BEFORE is the **verbatim prior content** of what changed — a quoted excerpt, not a command.

**BEFORE** (Mon Oct 26 03:41:09 PM UTC 2026, before any change):
```
$ uv run easy-personal --version
0.1.0
README.md:7:  Status: v0.1.0. It runs natively on Linux. ...
README.md:37: The first command prints `0.1.0`. ...
README.md:109: ... The tabs are Usage, Warnings & plan, Kit and Connectors.
README.md:162: ... A connector from before workspaces (for example `slack`) is removed as before.
```

**AFTER**:
```
$ uv run easy-personal --version
0.2.0
README.md:7:  Status: v0.2.0. ...
README.md:37: The first command prints `0.2.0`. ...
README.md:109: ... The tabs are Usage, Warnings & plan, Kit, Workspaces and Agents.
README.md:162: ... is removed only if its Claude Code entry starts `easy-personal-launch`; any other name is refused and nothing is removed.
```

**DELTA**: The package reports v0.2.0 and the README describes the Workspaces and Agents tabs and the exact rule for removing a pre-workspace connector.

**WITNESS**: Supervisor, 2026-10-26 — re-ran the readme/site tests and `--version` on branch HUN-73.
