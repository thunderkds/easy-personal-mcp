# Leak suite (HUN-36)

Proves the content boundary (PRD NFR-002, FR-002): planted secrets and message text never reach the
store, any MCP/CLI output, or a log. Run: `uv run pytest tests/leak -q`.

## How it works

1. `canaries.py` makes a fresh random canary per fixture field (`PATTERNS`), e.g. `sk-LEAKCANARY-<hex>`.
2. `tests/fixtures/leak/session.jsonl.tmpl` holds one `@@name@@` placeholder in every transcript
   field that carries content (prompt, assistant text, thinking, tool input/output, attachment,
   titles, malformed and non-UTF-8 lines, free text in label fields). `Canaries.render_fixture`
   writes it to a temp tree.
3. `conftest.py::leak_run` ingests that tree, then calls every surface and collects the output:
   real CLI subprocesses, every MCP tool in-process, the real stdio MCP server (responses and its
   stderr), captured log records, `sqlite3 .dump`, and the raw store/WAL/SHM bytes.
4. `Canaries.find` looks for each canary in raw, JSON, `\uXXXX`, URL, base64 (3 alignments) and hex
   form. Self-tests prove the scan fails on a deliberately leaking tool, log, and store row.

## Extend it (later tasks)

* New MCP tool: add it to `harness.MCP_CALLS`. A meta-test fails until you do.
* New CLI command: add it to `harness.CLI_CALLS` (or `CLI_SERVER_COMMANDS` if it serves forever).
* New source (kit_trace, verifier, ui, ...): add a `@@placeholder@@` line to the template (or a
  second template) for each content field of that source, and a pattern in `PATTERNS`.
* Dashboard/API: collect each response body into `Collected` and call `canaries.find` on it.
  Import with `from leak import harness` / `from leak.canaries import Canaries` (pytest `pythonpath`).
* A leak you find is a product bug: report it, do not weaken the scan.

Not covered (cut list): fuzzing; OS-level logs (only logs the package writes). Echoed caller
arguments (e.g. an invalid `--since` shown back in an error) are the caller's own input, not content.
