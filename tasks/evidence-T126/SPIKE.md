# T126 spike: steering a running `claude -p` over stream-json control requests

**Date**: 2026-11-19 (runs 11:19–11:24 UTC) · **CLI**: Claude Code 2.1.328 (`~/.local/share/claude/versions/2.1.328`)
**Run by**: backend-developer (T126), three runs, each one process started by the spike script and closed by it (stdin closed, exit 0; `pgrep` after: none left).

## How it ran

- A temp folder in the session scratchpad (`git init`, empty), never a repo of the owner's.
- `claude -p --input-format stream-json --output-format stream-json --verbose --model haiku` (run 1);
  `... --model sonnet --effort low --allowedTools "Bash(printenv:*)"` (runs 2 and 3).
- Environment: the parent's minus `CLAUDECODE`, `CLAUDE_CODE_*`, `ANTHROPIC_*`, `CLAUDE_EFFORT`, `CLAUDE_PID` (as the app does);
  the signed-in account's `CLAUDE_CONFIG_DIR` kept. stderr to `/dev/null`.
- Each control request was one stdin line `{"type":"control_request","request_id":"<id>","request":{...}}`; the script waited
  up to 15 s for the `control_response` whose `response.request_id` matched. Prompts: one tiny "Reply with the single word OK."
  (run 1) and "Run `printenv CLAUDE_EFFORT` and reply with its output" (runs 2, 3).
- Raw records: scratchpad `spike/out.jsonl`, `out2.jsonl`, `out3.jsonl` (not committed: the context answer lists the
  owner's MCP tool names).

## Answers (each `control_response`, as received)

Every answer is `{"type":"control_response","response":{"subtype":"success"|"error","request_id":"<id>", ...}}`.
All came back in < 1 s (5–709 ms; the slowest was `set_model` with a full model id).

### `set_model {model}`

| Sent `model` | Answer |
|---|---|
| `sonnet` | `success`, `response: {"model": "claude-sonnet-5-5"}` |
| `opus` | `success`, `{"model": "claude-opus-5-5"}` |
| `opusplan` | `success`, `{"model": "claude-sonnet-5-5"}` (opus only in plan mode) |
| `opus[1m]` | `success`, `{"model": "claude-opus-5-5"}` |
| `sonnet[1m]` | `success`, `{"model": "claude-sonnet-5-5"}` |
| `haiku[1m]` | **`error`**: `'haiku[1m]' isn't available: the haiku setting switches models in plan mode and has no 1M form. Run /model haiku, or /model sonnet[1m] for a 1M context window.`, `error_code: "alias_1m_unsupported"`, `model` = the model still in use |
| `default` | `success`, `{"model": "claude-opus-5-5"}` |
| `not-a-model-xyz` | **`error`**: `Model 'not-a-model-xyz' not found`, `error_code: "not_found"` |
| `claude-sonnet-5-5` | `success` (a full id works too; not offered: fixed aliases only) |
| `haiku` | `success`, `{"model": "claude-haiku-5-5"}` |

A `set_model` success came with one stream `user` record (its content was not recorded; the app's parser reads only
`tool_result` blocks of `user` records, so it adds no item unless it is one).
The next `system/init` carried the new model, and the next turn ran on it (`assistant.message.model`).

### `set_permission_mode {mode}`

| Sent `mode` | Answer |
|---|---|
| `plan`, `acceptEdits`, `auto`, `dontAsk`, `default` | `success`, `response: {"mode": "<mode>"}`; then a `system/status` record with `permissionMode` |
| `bypassPermissions` | **`error`**: `Cannot set permission mode to bypassPermissions because it is disabled by settings or configuration`, `error_code: "bypass_disabled"` (this account's settings; the CLI decides) |
| `bogusMode` | **`error`**: `Cannot set permission mode: must be one of acceptEdits, auto, bypassPermissions, default, dontAsk, plan`, `error_code: "invalid_mode"` |

The next `system/init` showed the mode set last (`plan`).

### `get_context_usage {detail?}`

`success`, `response` = `{categories: [{id, name, tokens, color, kind, isDeferred?}], totalTokens, maxTokens, rawMaxTokens,
percentage, model, autoCompactThreshold, isAutoCompactEnabled, autocompactSource, gridRows, memoryFiles, mcpTools,
mcpInstructions, agents, slashCommands, skills, messageBreakdown, apiUsage}`. Example numbers after one turn on haiku:
`totalTokens 27501`, `maxTokens 1000000`, `percentage 3`; category ids `system_prompt`, `system_tools`, `mcp_instructions`,
`mcp_tools_deferred`, `system_tools_deferred`, `skills`, `messages`, `autocompact_buffer`, `free_space`, `kind` one of
`used` / `deferred` / `buffer` / `free`. Works before the first turn too. `detail: "summary"` answered in the same shape as
the default `full`. `mcpTools`, `mcpInstructions`, `skills.skillFrontmatter` hold names of the owner's servers and skills:
the app passes on **numbers only** (used, limit, percent, and per category its id, kind and tokens).

### `apply_flag_settings {settings: {effortLevel}}` (the effort question)

| Run | Sent | Answer | `CLAUDE_EFFORT` in the next turn's Bash tool |
|---|---|---|---|
| 2 | started with `--effort low` | — | `low` |
| 2 | `effortLevel: "high"` | `success` (no `response`) | **`high`** |
| 2 | `effortLevel: "bogus"` | `success` | `high` (**ignored, yet "success"**) |
| 3 | started with `--effort low` | — | `low` |
| 3 | `effortLevel: "max"` | `success` | **`max`** |
| 3 | `effortLevel: "xhigh"` | `success` | **`xhigh`** |

Claude Code sets `CLAUDE_EFFORT` in its Bash tool's environment from the session's current effort (binary: `TJe({sessionId,
effortLevel, source:"agent"})`), so the tool output is the CLI's own effort for that turn.

## Findings → what T126 builds

1. **Model mid-session: yes.** List: `default`, `opus`, `sonnet`, `haiku`, `opusplan`, `opus[1m]`, `sonnet[1m]`
   (`haiku[1m]` is refused by the CLI, so it is not offered). The CLI's error text is shown as the reason.
2. **Mode mid-session: yes**, the FR-109 list. `bypassPermissions` may still be refused by the account's own settings;
   the app shows the CLI's line.
3. **Context: yes**, numbers only.
4. **Effort mid-session: yes** — it applies from the next turn (low → high, max, xhigh proven). Because an unknown level
   answers `success` and changes nothing, the server sends only the fixed list `low`, `medium`, `high`, `xhigh`, `max`
   (`claude --help`: `--effort <level>`). The request never carries a `source` field (the binary has a branch that persists
   `effortLevel` into user settings when `source: "userSettings"` is given: the app never sends it).
5. Effort has no echo: the app shows the level it last sent with a `success`, never a value read back.
