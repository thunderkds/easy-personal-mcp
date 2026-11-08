Cross-repo dependency XR-1 from easy-memory-mcp `PRD_v0.4.md`. Its producer, [HUN-197](https://linear.app/thunderkds/issue/HUN-197/report-json-writes-reportjson-for-the-easy-personal-dashboard), merged into easy-memory-mcp `main` on 2026-11-03 (`261c6f6`).

This is an intake row only. Stage 2 on this repo writes the TASK_GUIDE and sets Complexity and Risk.

## Requirement

Read `~/.easy-memory/<repo key>/report.json` directly. Do not call the MCP (DDR-0002), and read metadata only. Add a "memory on vs off" card to the Memory panel (FR-051..053 area).

## File contract (format `v` 1)

Fields:

* `v`, `generated` (UTC ISO), `min_sessions` (10), `sessions_logged`, `no_data`, `no_claude_log`, `forced`
* `groups.on` and `groups.off`, each with `sessions`, `tokens_all_median`, `tokens_io_median` and `turns_median`. Each median is null when its group is empty.
* `note_opened` with `opened` and `indexed`
* `too_few`, `on_vs_off_pct` (null when `too_few` is true or the off median is 0)
* `store` with `notes` and `bytes`

The file never holds prompt text, answer text, note text, a session id or a file path.

## Rules

* Abstain on `v` > 1: show "unsupported report format" and draw no numbers.
* When `too_few` is true, say "too few sessions (N on / M off, need 10 each)" and show no saving.
* A missing file means "no report yet". The easy-memory Stop hook refreshes the file at most once per hour, so show `generated` as its age.
* Known producer quirk: a store folder can hold only `report.json` and no `notes/` folder (made by `report --json --repo <wrong path>`). Treat it as an empty project or skip it. Do not crash.

## Acceptance (draft)

1. Fixture `report.json` → the card shows on/off medians and the percentage.
2. `too_few` → the message shows and no percentage appears.
3. `v: 2` → abstain.
4. Missing or bad JSON → "no report yet", no exception.
5. Store folder with only `report.json` → no crash.