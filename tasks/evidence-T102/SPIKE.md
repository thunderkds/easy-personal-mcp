# T102 spike: the Claude Code routes behind write sessions (FR-109..FR-114)

**Date**: 2026-11-17 · **`claude --version`**: `2.1.326 (Claude Code)` · Linux 7.0 · Python 3.14 (probe scripts) · tmux 3.6
**Scripts**: `rust/spikes/write-sessions/` (`cc.py` helpers; `p1`..`p7`; `perm_server.py`, `perm_http_server.py`, `perm_fwd_server.py`, `hook.py`).
**Invocations**: about 26 `claude` runs, each a tiny prompt on `--model haiku`. Every sample below has model and user text replaced by `<text>`; no secret, token or transcript line is pasted.

## How the probes ran (safety)

- Each probe runs in a scratch git repo `/tmp/t102/<name>`; `claude` is started with an argument list (never a shell).
- Environment: the parent Claude Code markers are dropped (`CLAUDECODE`, `AI_AGENT`, `CLAUDE_CODE_*`, `CLAUDE_PID`, `CLAUDE_EFFORT`); `CLAUDE_CONFIG_DIR` is kept, because the machine's default folder `~/.claude` is **not logged in** (`claude auth status` says `loggedIn: false`) and the logged-in account lives in the folder `CLAUDE_CONFIG_DIR` already points at. No login file was copied; no API key variable exists in the probe environment.
- Hooks and MCP config reach `claude` only through `--settings <tmp file>` and `--mcp-config <tmp file> --strict-mcp-config`. No Claude settings file and no `~/.claude.json` was edited by a script. `--setting-sources project` keeps the owner's user-level hooks and permission rules out of the probes.
- Things **Claude Code itself** wrote into the logged-in config folder while running: transcripts of the scratch folders under `projects/-tmp-t102-*`, two plan files `plans/make-a-one-step-plan-*.md` (plan-mode probe), and the trust entry for `/tmp/t102/p6` (the interactive trust dialog cannot be skipped; `-p` runs have none). The owner can delete these; nothing else was changed.
- The interactive probes (AC 3 is headless by nature, AC 6) ran under a **private tmux server** (`tmux -L t102`, never the owner's default one), so they were done headless; no manual commands are left for the owner.

## Acceptance criteria and verdicts

| # | Criterion | Result | Verdict |
|---|---|---|---|
| 1 | stream-json input: two user messages, two results; line shape and event types written down | Two `result` events from one process, which stays alive; session id is the same in both; shapes below | **GO** |
| 2 | Slash command sent as a user message | A project command and a project skill run as in the CLI; text built-ins answer as a result; terminal-only ones are refused; differences below | **GO** (with listed differences) |
| 3 | SIGINT during a long answer | `SIGINT` to the process: the turn aborts **and the process exits** (code 0). The documented interrupt is a `control_request` on stdin: the turn aborts, the process stays, the next message is answered | **SIGINT: NO-GO as "Esc"; control_request interrupt: GO** |
| 4 | `--permission-prompt-tool`: Bash, AskUserQuestion, ExitPlanMode; input shape and accepted results | All three reach the tool; allow / deny shapes and `updatedInput.answers` verified; shapes below | **GO** |
| 5 | Transport that reaches the running app | Both work, with a 200 s wait: (a) loopback HTTP MCP URL in `--mcp-config` (needs `MCP_TOOL_TIMEOUT`), (b) stdio child + unix socket (no setting needed). `mcp_http.rs` itself is **not** the running app (see findings) | **GO**: recommend (a) served by the dashboard process |
| 6 | Hooks: PreToolUse `AskUserQuestion`, PermissionRequest, empty exit 0, what the terminal shows | All work through `--settings`; PreToolUse shows no question UI; PermissionRequest shows its dialog while the hook waits, and the hook's answer closes it | **GO** (two corrections to FR-114, below) |
| 7 | `--resume <id> --fork-session` | New id; original transcript byte-identical (same line count and hash); plain `--resume` keeps the id and appends | **GO** |
| 8 | Findings, commands, JSON samples, go/no-go | This file | done |

### Go / no-go per route

| Route | Verdict |
|---|---|
| stream-json input (`-p --input-format stream-json --output-format stream-json --verbose`) | GO |
| Slash commands as user messages | GO |
| Interrupt by SIGINT | NO-GO (the process exits). Use `control_request` `interrupt` instead: GO |
| Stop by SIGTERM to the process group | GO (the stdio child of the permission tool also ended with `claude`) |
| `--permission-prompt-tool` for Bash | GO |
| `--permission-prompt-tool` for AskUserQuestion (`updatedInput.answers` honoured) | GO |
| `--permission-prompt-tool` for ExitPlanMode (allow leaves plan mode; deny keeps planning) | GO |
| Transport (a): loopback HTTP MCP URL, held 200 s | GO with `MCP_TOOL_TIMEOUT` set; **NO-GO without it** (cut at 60 s, then retried) |
| Transport (b): stdio child + unix socket, held 200 s | GO, no setting needed |
| Hook PreToolUse `AskUserQuestion` answers it | GO |
| Hook PermissionRequest allows / denies Bash | GO |
| Hook fail-open (exit 0 and no output; or a hook that times out) | GO: the terminal asks as usual |
| Fork (`--resume <id> --fork-session`) | GO |

## 1. stream-json input (AC 1)

Command (the app's future command line, plus probe flags):

```
claude -p --input-format stream-json --output-format stream-json --verbose --setting-sources project --model haiku
```

Stdin: one JSON object per line. Accepted shapes (both tested):

```json
{"type":"user","message":{"role":"user","content":"<text>"}}
{"type":"user","message":{"role":"user","content":[{"type":"text","text":"<text>"}]}}
```

Stdout events (one JSON per line). Sequence seen for two messages: `system/init`, `assistant`, `rate_limit_event`, `result`, then `system/init` again, `assistant`, `result` (the process stayed alive between them; closing stdin ends it with exit 0).

| `type` / `subtype` | When | Keys that matter |
|---|---|---|
| `system` / `init` | at the start **and again for every user message** | `session_id`, `cwd`, `model`, `permissionMode` (`default` with no flag), `tools[]`, `mcp_servers[{name,status,source}]`, `slash_commands[]`, `skills[]`, `plugins[]`, `capabilities[]`, `claude_code_version`, `apiKeySource` (`none` = subscription login) |
| `assistant` | each assistant message | `message.content[]` blocks of `type` `text`, `thinking` or `tool_use` (`name`, `input`, `id`); `session_id`; `parent_tool_use_id`; `uuid` |
| `user` | tool results (`tool_result` blocks, `is_error`), and an echo of the owner's message only with `--replay-user-messages` (`"isReplay":true`) | `message.content[]`, `session_id` |
| `result` | end of each turn | `subtype` (`success` or `error_during_execution`), `is_error`, `num_turns`, `result` (text), `session_id`, `terminal_reason`, `stop_reason`, `permission_denials[{tool_name,tool_use_id,tool_input}]`, `total_cost_usd`, `duration_ms` |
| `rate_limit_event` | sometimes before a result | `rate_limit_info{status, resetsAt, rateLimitType}` |
| `system` / `thinking_tokens`, `system` / `status` | thinking, mode changes | (ignore) |
| `stream_event` | only with `--include-partial-messages` | `event.type` (`content_block_delta`, ...) |
| `control_response` | answer to a `control_request` (see AC 3) | `response.subtype`, `response.request_id` |

`result` sample (text replaced):

```json
{"type":"result","subtype":"success","is_error":false,"num_turns":1,"result":"<text>","session_id":"<text>",
 "terminal_reason":"completed","stop_reason":"end_turn","permission_denials":[],"total_cost_usd":0.0015,"duration_ms":2993}
```

Session id: the field `session_id` on every event; the same for the whole process. `init.capabilities` lists `interrupt_receipt_v1`, `interrupt_cancel_queued_v1`, `ask_user_question_unanswered_v1`, `mcp_call_check_permissions_v1`, among others. `--help` also lists `--permission-prompts host|none` and the `--permission-mode` choices `acceptEdits, auto, bypassPermissions, manual, dontAsk, plan` (**no `default` value**: leave the flag out for the default; `manual` is the interactive default mode).

Script: `rust/spikes/write-sessions/p1_stream_json.py`, `p1b_shapes.py`.

## 2. Slash commands (AC 2)

Sent as an ordinary user line, text starting with `/` (`p2_slash.py`, `/tmp` scratch repo with `.claude/commands/ping.md` and `.claude/skills/pong/SKILL.md`):

| Sent | What came back |
|---|---|
| `/ping` (project command) | executed; assistant text and `result` (`num_turns` 1) |
| `/pong` (project skill) | executed; same shape |
| `/context` | one `assistant` text + `result` with `num_turns` 0 (about 8.8 KB of text, no model call) |
| `/cost` | same shape, usage text of the plan |
| `/help` | `result` text: `/help isn't available in this environment.` (`num_turns` 0) |
| `/nosuchcommand` | the **model** answers that the command is not available (`num_turns` 1), nothing ran |

`init.slash_commands` and `init.skills` list what is available in the session (the project command and skill appear); `init.terminal_slash_commands` (`doctor`, `color`, `focus`, `reload-plugins`) are terminal-only. **Difference from the CLI:** terminal UI commands (`/help` and any picker-style one) are refused with a one-line result; local text commands (`/context`, `/cost`) return their text as a `result` with `num_turns` 0, not as tool calls. The session panel must show a `result` with `num_turns` 0 as the command's output.

## 3. Interrupt (AC 3)

`p3_sigint.py <sigint|control>`: a 2000-word prompt, streaming started (first `content_block_delta`), 1 s later:

- **`kill -INT <claude pid>`**: result in 0.06 s: `{"subtype":"error_during_execution","is_error":true,"terminal_reason":"aborted_streaming"}`, then the process **exits** (code 0). A second message cannot be sent. This is not Esc behaviour.
- **`control_request` on stdin** (the route the stream-json protocol offers):

```json
{"type":"control_request","request_id":"req_1","request":{"subtype":"interrupt"}}
```

  Answer: `{"type":"control_response","response":{"subtype":"success","request_id":"req_1","response":{"still_queued":[]}}}`, then the same `result` (`error_during_execution`, `aborted_streaming`) in 0.04 s; **the process stays alive** and the next user line (`say ok`) was answered with `success`.

Verdict: the Interrupt button must send the `control_request`; SIGINT is only a stop. (`SIGINT` of the process group was not run separately; a process that exits on `SIGINT` sent to its pid has the same effect.)

## 4. `--permission-prompt-tool` (AC 4)

Setup (`perm_server.py`, a stdio MCP server with one tool `permission_prompt`; `p4_permission_tool.py <scenario>`):

```
claude -p --input-format stream-json --output-format stream-json --verbose --setting-sources project --model haiku \
  --mcp-config /tmp/t102/<x>_cfg/mcp.json --strict-mcp-config --permission-prompt-tool mcp__perm__permission_prompt
```

`mcp.json`: `{"mcpServers":{"perm":{"command":"python3","args":[".../perm_server.py"],"env":{...}}}}`. The tool is **not** listed in `init.tools`; `init.mcp_servers` shows it as `connected`. The tool is called **only when the CLI would prompt**: `echo ...` (read-only, auto-allowed) never reached it; `touch ...` did.

**Input** (the `arguments` of the `tools/call`; `_meta` has `claudecode/toolUseId` and `progressToken`):

```json
{"tool_name":"Bash","input":{"command":"<text>","description":"<text>"},"tool_use_id":"toolu_..."}
{"tool_name":"AskUserQuestion","input":{"questions":[{"question":"<text>","header":"<text>","options":[{"label":"<text>","description":"<text>"}],"multiSelect":false}]},"tool_use_id":"toolu_..."}
{"tool_name":"ExitPlanMode","input":{"plan":"<text>","planFilePath":"<path under the config dir's plans folder>"},"tool_use_id":"toolu_..."}
```

**Result**: the tool returns one MCP text content whose text is this JSON:

| Sent back | Effect observed |
|---|---|
| `{"behavior":"allow","updatedInput":{...the input...}}` | Bash ran; `tool_result` `is_error:false` |
| `{"behavior":"allow"}` (no `updatedInput`) | accepted; Bash ran (the CLI's own error text calls `updatedInput` optional) |
| `{"behavior":"deny","message":"<text>"}` | Bash not run; the model receives `message` as an error `tool_result` (`is_error:true`); `result.permission_denials` lists the call |
| text that is not that JSON | **fails closed**: tool not run, the model gets `The permission prompt tool returned an invalid permission result. Expected {behavior: 'allow', updatedInput?: object} or {behavior: 'deny', message: string}. updatedPermissions ...` (also an optional `updatedPermissions` exists) |
| AskUserQuestion: `{"behavior":"allow","updatedInput":{"questions":[...],"answers":{"<question text>":"<label>"}}}` | honoured: the model got `The user answered: "<question>"="<label>"...`. **A free-text answer that is not one of the labels** (the "Other" case) is accepted the same way. `multiSelect` answer format: **not tested** |
| ExitPlanMode: `allow` with `updatedInput` = the input | plan approved, plan mode left, the next tool ran |
| ExitPlanMode: `{"behavior":"deny","message":"<text>"}` | stays in plan mode; the model gets the message as an error and asks for feedback (**"Keep planning" = deny with a note**) |

## 5. Transport that reaches the running app (AC 5)

Facts from the code: `mcp_http.rs` is `mcp --transport http`, a **separate process** (its own port, default 8765; stateless, one fresh service per POST; `GET`/`DELETE` are 405; no token; Host/Origin guard). It is not part of the `dashboard` process that will hold the session runner and the pending prompts (`crates/app/src/main.rs`: `mcp_http` is only called from `mcp::main`). A permission prompt must end up in the dashboard process, so the existing `mcp_http` server cannot hold it.

What claude accepts (all with `--mcp-config <tmp file> --strict-mcp-config`):

- **(a) HTTP URL**: `{"mcpServers":{"perm":{"type":"http","url":"http://127.0.0.1:<port>/mcp","headers":{"X-Spike-Session":"...","Authorization":"Bearer ..."}}}}`. `perm_http_server.py` copies `mcp_http.rs`'s rules (POST, 200 JSON, 202 for notifications, GET/DELETE 405, Host check). Claude connected (status `connected`) to it **and** to the real `easy-personal-rs mcp --transport http` in the same run. The configured headers arrive on every POST, so a per-session secret and session id travel in the temp config. Claude sends `server/discover`, `initialize`, `notifications/initialized`, one `GET` (405 is fine), `tools/list`, `tools/call`. No SSE is needed.
- **Hold time over HTTP**: with a 65 s wait and no setting, claude cut the request at 60 s (connection reset on the server) and sent a **second** `tools/call` for the same prompt; the run ended without the command running (132 s). With `MCP_TOOL_TIMEOUT=900000` in `claude`'s environment: one call, answered after 70 s (run 76 s) and after 200 s (run 211 s); the command ran. The app builds `claude`'s environment (FR-102), so it can set the variable.
- **(b) stdio child + unix socket**: `perm_fwd_server.py` is a stdio MCP child (`<binary> mcp-permission`-like) that forwards each call over a unix socket (mode 0600) to "the app" (`p5b_stdio_socket.py`), which saw the call with the session name given in the temp config's `env`, read the peer uid with `SO_PEERCRED` (it was the owner's), and answered after 70 s and after 200 s with **no** timeout setting; the command ran. The child ended when `claude` ended (no orphan).

**Recommendation: (a) HTTP, served by the dashboard process itself** (a new loopback route, not `mcp_http.rs` and not a tool on the shared MCP server). Reasons: no second process or new subcommand, no socket file or peer-uid code, the pending prompt is a value in the same process that shows the card, a per-session bearer token in the 0600 temp `--mcp-config` (deleted when the session ends) ties a prompt to its session, and the surface "no new MCP tool" of DDR-0006 stays true because this endpoint is a private route that serves only `permission_prompt` for one session. Conditions: set `MCP_TOOL_TIMEOUT` (use a large value; a prompt that waits longer than it is cut and retried), answer a retried `tools/call` by the same `tool_use_id` once, keep the route off the dashboard page token, and keep the Host/Origin guard. **Fallback if the variable ever stops working: (b)**, which needed no setting.

Not tested: a wait longer than 200 s, the owner stopping the session while a prompt waits (expected: the HTTP request is dropped; the app must settle the card), and a multi-session run.

## 6. Hooks (AC 6)

`p6_hooks.py` (interactive `claude --setting-sources project --settings /tmp/t102/p6_cfg/settings.json --model haiku --permission-mode manual` in the scratch repo, driven with `tmux -L t102 send-keys` / `capture-pane`). Settings file (temp, passed by `--settings`):

```json
{"hooks":{"PreToolUse":[{"matcher":"AskUserQuestion","hooks":[{"type":"command","command":"python3 .../hook.py","timeout":130}]}],
          "PermissionRequest":[{"hooks":[{"type":"command","command":"python3 .../hook.py","timeout":8}]}]}}
```

**Hook input (stdin JSON), keys only:**

- `PreToolUse`: `cwd, effort, hook_event_name, permission_mode, prompt_id, scratchpad_dir, session_id, tool_input, tool_name, tool_use_id, transcript_path`
- `PermissionRequest`: the same **without `tool_use_id`**, plus `permission_suggestions`. (`tool_input` for Bash: `command`, `description`; for AskUserQuestion: `questions`.)

**Outputs tried and what the terminal did:**

| Hook printed | Result |
|---|---|
| PreToolUse, matcher `AskUserQuestion`: `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","updatedInput":{"questions":[...],"answers":{"<question text>":"<label>"}}}}` (after a 5 s wait) | The question UI was **never shown** (screen sampled every second, twice). While the hook ran the only line was the spinner `(running PreToolUse hook · 5s ...)`; then the terminal printed `Answered Claude's questions: · <question> → <label>` and the model used the answer |
| PermissionRequest: `{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}` (instant) | `touch` ran; no dialog seen |
| PermissionRequest: `...{"behavior":"deny","message":"<text>"}` (instant) | not run; no dialog seen |
| exit 0, no output (instant) | the normal terminal dialog appeared ("Do you want to proceed?"); Escape declined; file not created |
| PermissionRequest allow after a **5 s wait** | the terminal dialog **was visible while the hook waited** (seen from 3 s to 8 s) and closed by itself when the hook allowed; the command ran |
| a hook that sleeps 12 s with `timeout` 8 | the dialog stayed after the timeout (fail-open); Escape declined; file not created |

Verdicts: PreToolUse for `AskUserQuestion` GO (no terminal UI while it waits, only a spinner); PermissionRequest GO but **the terminal shows its own dialog during the wait**, and the hook answer dismisses it; exit 0 / no output / timeout all leave the terminal prompt as usual (fail-open GO). Sampling was once per second, so a flash shorter than that was not seen for the instant hooks. Not tested: the owner answering the terminal dialog while a PermissionRequest hook is still waiting (does the hook get cancelled?), and a hook in the user-level `<CLAUDE_CONFIG_DIR>/settings.json` (not edited by rule; `--settings` takes the same `hooks` key).

## 7. Fork (AC 7)

`p7_fork.py` (stream-json sessions in one scratch repo; the transcript is `<config dir>/projects/<cwd with non-alphanumerics as "-">/<id>.jsonl`, only its line count and a hash were read):

| Step | Result |
|---|---|
| A: new session, `say ok` | transcript: 29 lines |
| `--resume A --fork-session` | `system/init.session_id` is **a new id**; A's transcript **byte-identical** (same lines, same hash); the fork has its own file |
| `--resume A` (no fork) | **same id**; A's transcript grew (29 to 40 lines): the append that loses messages when a terminal has the session open |

## Findings the PRD text gets wrong or leaves open

1. **FR-110 "Interrupt sends SIGINT (as Esc)"** is wrong for this version: `SIGINT` ends the process. Send the stdin `control_request` `interrupt` (AC 3). Stop stays SIGTERM then SIGKILL.
2. **FR-111 "this app's own MCP server"**: `mcp_http.rs` is a separate, stateless process, not the running dashboard. The permission tool must be served by the dashboard process (recommended: a private loopback HTTP route with a per-session bearer in the temp `--mcp-config`), with `MCP_TOOL_TIMEOUT` set in the child environment. Without it a prompt older than 60 s is cut and retried (a duplicate prompt).
3. **FR-111 result shapes** are confirmed: `allow` (with or without `updatedInput`), `deny` + `message`, `answers` keyed by the **question text**, free text accepted. Anything else fails closed.
4. **FR-114 "no terminal UI"** holds for `AskUserQuestion` (PreToolUse), not for a PermissionRequest: its terminal dialog is on screen while `hook ask` waits. The card text "Answer in terminal" is still right; an app answer closes the dialog.
5. **FR-114 matching**: the PermissionRequest input has **no `tool_use_id`**; match by `session_id` (+ `tool_name` and `tool_input`).
6. **FR-109 permission mode list**: the `--permission-mode` flag has no `default` value (`acceptEdits, auto, bypassPermissions, manual, dontAsk, plan`). "default" means leave the flag out. In this account an interactive terminal starts in **auto** mode; a `-p` session without the flag starts in `default`: a parity difference the form's mode list should surface.
7. **FR-110 slash commands**: terminal-only commands (`/help`, `doctor`, `color`, `focus`, `reload-plugins`) are refused; local text commands return a `result` with `num_turns` 0.
8. **FR-109 environment scrub (FR-102)**: the probes dropped the parent session markers (`CLAUDECODE`, `CLAUDE_CODE_*`). `agent_run.rs` does not scrub them; when the app is started from inside a Claude Code session they would be inherited. Whether that changes a child's behaviour was **not tested**: scrub them in app sessions to be safe.
9. **Trust dialog**: an interactive `claude` in an untrusted folder asks first; `-p` does not. The hook bridge for terminal sessions is not affected.
10. `init` is re-sent for every user message: the panel must not treat it as a new session.
