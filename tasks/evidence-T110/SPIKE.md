# T110 spike: reply to an idle session through a Stop hook (PRD question 42, option A)

**Date**: 2026-11-18 · **`claude --version`**: `2.1.327 (Claude Code)` · Linux 7.0 · Python 3.14 (probe scripts) · tmux 3.6
**Scripts**: `rust/spikes/reply-sessions/` (`cc.py` helpers, `stop_hook.py` the probe hook, `p1_headless.py`, `p1b_shapes.py`, `p2_latency_text.py`, `p3_loop_transcripts.py`, `p4_interactive.py`).
**Invocations**: 10 headless `claude -p` runs and 10 interactive runs, all `--model haiku`, prompt "say ok". Text from the model or the owner is replaced by `<text>` below, except my own test words (`banana`, `alpha-typed`, `/help`, `END-MARKER-7Q`).

## How the probes ran (safety)

- Scratch git repos under `/tmp/t110/<name>`; `claude` started with an argument list (never a shell); parent Claude Code markers dropped from the environment.
- Login: `CLAUDE_CONFIG_DIR` kept as set (the logged-in account; `claude auth status` says `loggedIn: true`). The default folder `~/.claude` is not logged in (`env -u CLAUDE_CONFIG_DIR claude auth status` says `loggedIn: false`, as in T102), so the default-account route was not usable. No login or settings file was read for content, copied or edited by a script.
- Hooks reach `claude` only through `--settings <tmp file>` (`/tmp/t110/*_cfg*/settings.json`) with `--setting-sources project`. The hook script is `rust/spikes/reply-sessions/stop_hook.py`; its modes come from a temp JSON file.
- Interactive probes ran in a private tmux server (`tmux -L t110`), never the owner's terminal. Only sessions I started were killed, by session name.
- Things Claude Code itself wrote in the logged-in config folder: transcripts under `projects/-tmp-t110-*` and the folder-trust entry for `/tmp/t110/p4` (the interactive trust dialog cannot be skipped). The owner can delete them. One unrelated one-time dialog ("Teach auto mode about your environment?") appeared after the first interactive turn; I dismissed it with Esc (no choice stored by me).
- This account runs in auto mode (`auto mode on` in the status line); it did not matter to a Stop hook probe.

## Acceptance criteria and verdicts

| # | Criterion | Result | Verdict |
|---|---|---|---|
| 1 | Block with reason continues the session with the text | Headless: 2 assistant messages, the second says `banana` (`num_turns` 2). Interactive: the same, the terminal prints the text, then Claude answers `banana`. Transcript shape below | **GO** |
| 2 | Loop guard | Stop input keys listed below; `stop_hook_active` is `false` on the first Stop and `true` on the Stop that follows a blocked one; `block_if_inactive` gives exactly one extra turn (2 Stop calls, `num_turns` 2). Claude Code sets **no cap itself**: an always-blocking hook ran 4 extra turns (`num_turns` 5, `stop_hook_active` true on calls 1..4) until the hook stopped | **GO** (the loop guard is the hook's job) |
| 3 | No reply | Exit 0 with no output: the turn ends as usual, the terminal is back at its prompt (interactive runs with a 20 s hold that exits 0, and the instant-empty second Stop). Hook cost as recorded by Claude Code (`stop_hook_summary.hookInfos[].durationMs`): 18, 19, 25, 28 ms (Python start included) | **GO** (< 100 ms) |
| 4 | Terminal while the hook holds (interactive) | Owner can type; text is accepted into the input box; Enter queues it and it is sent as the next user message after the hook returns; **Esc and Ctrl+C both kill the hook at once** and the terminal is back at its prompt; nothing typed is lost. Details below | **GO** |
| 5 | Hook timeout (`timeout: 30`, hook sleeps 60 s) | At 30 s the hook is killed, no output; the turn ends quietly (`done` line, prompt back). No error line on screen, no `stop_hook_summary` record in the transcript | **GO** (fails open) |
| 6 | Text shape | A 4,000-character reason with 49 line breaks arrives whole (Claude copied the final marker line; transcript content equals `Stop hook feedback:\n` + reason exactly). A reason `/help` arrives as plain text: no slash command ran (no local-command record; Claude answered with prose) | **GO** |
| 7 | Headless | `claude -p` with the same hook: the block continues the run, one extra turn, still one `result` event | **GO** (a `-p` session needs no exclusion) |
| 8 | Findings, go/no-go, hold-rule recommendation | This file | done |

**Overall: GO** for the Stop-hook reply (PRD question 42 option A). No blocker; one question for the owner is at the end.

## 1. Block with reason (AC 1) and 7. headless

Command (one run; hook modes `["block_if_inactive","block_if_inactive","empty"]`, reason file = "Reply with only the single word banana."):

```
claude -p "say ok" --output-format stream-json --verbose --setting-sources project --settings /tmp/t110/p1_cfg/settings.json --model haiku
```

`settings.json`:

```json
{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"python3 <repo>/rust/spikes/reply-sessions/stop_hook.py"}]}]}}
```

Hook stdout (the whole protocol): `{"decision":"block","reason":"<text>"}`.

stream-json sequence (headless): `system/init`, `assistant`, `rate_limit_event`, `user` (`isSynthetic: true`, content `[{"type":"text","text":"<text>"}]`), `system/notification` (`key`, `text`, `priority`), `assistant`, `result` (`success`, `num_turns` 2). The second assistant message contains `banana`; the `result` text contains `banana`.

Transcript records after the first assistant message (JSONL under the config folder, text replaced):

```json
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text"}],"stop_reason":"end_turn"}}
{"type":"user","isMeta":true,"message":{"role":"user","content":"Stop hook feedback:\n<text>"}}
{"type":"attachment","attachment":{"type":"hook_blocking_error","hookName":"Stop","hookEvent":"Stop","toolUseID":"<text>","blockingError":{"blockingError":"<text>","command":"<text>"}}}
{"type":"system","subtype":"stop_hook_summary","hookCount":1,"hookInfos":[{"command":"<text>"}],"hookErrors":["<text>"],"preventedContinuation":false,"hasOutput":true,"level":"suggestion"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text"}],"stop_reason":"end_turn"}}
{"type":"system","subtype":"stop_hook_summary","hookCount":1,"hookInfos":[{"command":"<text>","durationMs":22}],"hookErrors":[],"preventedContinuation":false,"hasOutput":false,"level":"suggestion"}
```

So the owner's text is stored by Claude Code in three places: a **`user` record with `isMeta: true` whose string content is `Stop hook feedback:\n<text>`** (the prefix is `Stop hook feedback:` then a newline), an `attachment` record of type `hook_blocking_error` (`blockingError.blockingError`), and the `stop_hook_summary` system record (`hookErrors[0]`). It is not a typed prompt: `isMeta: true` marks it. An app that counts "typed prompts" in transcripts must skip `isMeta` records.

On screen (interactive) the text appears as:

```
● Ran 1 stop hook
  ⎿  Stop hook reported a problem
     <text>
● banana
```

The owner's own reply is therefore shown under the label "Stop hook reported a problem" (cosmetic; not changeable from the hook).

## 2. Stop hook input and the loop guard (AC 2)

Full key list of the Stop hook's stdin JSON (with the type of each value):

`background_tasks` (list), `cwd` (str), `effort` (dict), `hook_event_name` (str, `Stop`), `last_assistant_message` (str: what Claude just said), `permission_mode` (str), `prompt_id` (str), `session_crons` (list), `session_id` (str), `stop_hook_active` (bool), `transcript_path` (str).

Stop calls in the `block_if_inactive` run (from the hook log):

| call | `stop_hook_active` | hook output | result |
|---|---|---|---|
| 0 | `false` | block + reason | a new turn starts |
| 1 | `true` | none | the run ends; `num_turns` 2 |

Always-block run (`["block","block","block","block","empty"]`, headless, 1 run): Stop calls 0..4 had `stop_hook_active` `false,true,true,true,true`; `num_turns` 5; the run ended only when the hook stopped blocking. Claude Code does not cap the loop: a hook that always blocks loops until it stops (or the owner presses Esc).

## 3. No reply and latency (AC 3)

`HOOK_MODES=["empty"]` (exit 0, no output), `claude -p "say ok"`, 3 runs with the hook and 3 without, alternating:

| run | wall s | `duration_ms` (result) | turns | Stop calls |
|---|---|---|---|---|
| hook 1 | 7.64 | 1883 | 1 | 1 |
| no hook 1 | 8.25 | 2446 | 1 | 0 |
| hook 2 | 7.67 | 1935 | 1 | 1 |
| no hook 2 | 7.61 | 1859 | 1 | 0 |
| hook 3 | 7.41 | 1410 | 1 | 1 |
| no hook 3 | 7.44 | 1721 | 1 | 0 |

The end-to-end difference is below the model's own run-to-run noise (about 0.5 s), so the hook's cost is read from the hook's own record: `durationMs` 18 to 28 ms for a Python start that exits at once (4 samples). A hook that is itself a small Rust binary or a `curl` to loopback costs about the same.

## 4. Terminal while the hook holds (AC 4)

`HOOK_MODES=["sleep:20"]` (hook sleeps 20 s, then exits 0 with no output); `claude --setting-sources project --settings <tmp> --model haiku` in a tmux pane; prompt `say ok`. Screen facts (`tmux capture-pane`):

| Time | What the terminal shows |
|---|---|
| answer printed | `● ok` (Claude's answer is shown at once) |
| while holding | a spinner line `✻ Undulating… (running Stop hook · 4s · ↓ 4 tokens)` with a counter that goes up each second; the input box `❯` is visible and empty; the footer reads `esc to interrupt` |
| 20 s | the spinner line becomes `✻ Sautéed for 22s · done 11:17 AM` and the session is idle |

What the owner can do during the hold:

| Action during the hold | Result | Class |
|---|---|---|
| Type `alpha-typed` (no Enter) at +4 s | the text appears in the input box (`❯ alpha-typed`) | **(a) accepted into the input box** |
| Enter at +8 s | the line shows `⎿ Queued · ctrl+x ctrl+s to send now`; the input box says `Press up to edit queued messages`; when the hook returned at 20 s the queued text was **sent as the next user message** (Claude answered it) | **(b) sent after the hook returns** |
| Esc at +5 s | the hook process is killed at once (it never reached its exit); the turn ends with `⎿ Interrupted · What should Claude do instead?`; the input box is empty and ready; no further turn | **hook ended early** |
| Ctrl+C at +5 s | identical to Esc (hook killed, `Interrupted · What should Claude do instead?`, the session stays open at its prompt; one press does not exit Claude Code here) | **hook ended early** |
| Type text (no Enter) at +4 s, Esc at +9 s | hook killed, `Interrupted`; the typed text is **still in the input box** | not lost |
| Type + Enter at +4/+6 s (queued), Esc at +9 s | hook killed, `Interrupted`; the queued message is **sent at once** as a new turn (Claude answered it) | not lost |

Nothing typed was lost in any run. A killed hook leaves no `stop_hook_summary` record and the Stop does not run again for that turn end. After Esc or Ctrl+C the session is ordinary and idle.

Typed message and block reason together (`delay_block:12`, owner types and presses Enter at +4/+6 s, the hook blocks at 12 s): the block reason is shown and its turn runs first (`banana`), then the queued typed message runs as its own turn. Order is preserved, none is dropped. (Stop calls: `false` (blocked), `true`, then `false` after the queued turn.)

## 5. Hook timeout (AC 5)

Settings: `"timeout": 30`; hook mode `sleep:60`; interactive; nothing typed.

- While held: the spinner counter shows `running Stop hook · 28s … 31s`.
- At +30 s: the hook is killed; the screen changes to `✻ Cogitated for 32s · done 11:22 AM`, prompt back. **No error line**, and nothing in the transcript (no `stop_hook_summary`). The turn ended as with no output (fail open).
- The default timeout (no `timeout` key) was **not** measured here; set `timeout` explicitly in the installed entry.

## 6. Text shape (AC 6)

| Reason sent | What Claude Code did |
|---|---|
| 4,000 characters, 49 line breaks, instruction on line 1 and `END-MARKER-7Q` as the last line | New turn; the answer contained `END-MARKER-7Q`. Transcript content length 4020 = `Stop hook feedback:\n` (20) + 4000, equal to prefix + reason exactly: no truncation, no change |
| `/help` | New turn; transcript content `Stop hook feedback:\n/help`; **no** local-command or slash-command record; Claude answered in prose (about what it can help with). The text is plain text, never a command |

Because of the `Stop hook feedback:\n` prefix, even a reason that starts with `/` never starts the message with `/`.

## 7. Headless (AC 7)

Same hook, `claude -p "say ok"`: block continues the run (see section 1; `num_turns` 2, one `result`). A stream-json `-p` session therefore behaves the same: the Stop hook runs at every turn end there too, so the app must not hold app-driven (stream-json) sessions; they take replies as user messages.

## Go / no-go per route

| Route | Verdict |
|---|---|
| Stop hook `{"decision":"block","reason":"<text>"}` continues the same session (interactive and `-p`) | GO |
| `stop_hook_active` as loop guard | GO (true on every Stop after a block) |
| Exit 0 with no output ends the turn as usual | GO |
| Esc / Ctrl+C ends a holding hook, terminal back at once | GO |
| Typing in the terminal during the hold | GO (accepted; Enter queues, sent after) |
| Hook `timeout` | GO (killed, quiet end of turn) |
| 4,000-character multi-line text; leading `/` | GO |
| Claude Code's own cap on repeated blocks | none seen up to 4 blocks |

## 8. Recommendation: the hold rule

**Choose (ii): hold only a session the owner marked "reply from the app" in the Live view, with a hold cap shorter than the hook's `timeout`.** (Not (i).)

Reason, from the facts above:
- A holding hook makes the owner's terminal look busy (`running Stop hook · Ns` spinner) for the whole hold. With (i), every turn end of every watched session would sit busy for up to the cap, even when the owner is at the terminal and never replies from the app. With (ii) only sessions the owner chose are affected, so the cost is paid exactly where the owner asked for it.
- The terminal is never trapped: Esc or Ctrl+C kills the hook at once and returns the prompt; typed text and queued messages are kept; a hook that exceeds its `timeout` ends the turn quietly. So the opt-in is a convenience rule, not a safety need.
- Claude Code has no loop cap, and repeated blocks are the hook's job: the hook must check `stop_hook_active` (block only when it is `false`, or limit the number of consecutive blocks per session in the app), and must return no output (exit 0) when no reply is waiting, which costs about 20 ms.
- A held hook in a session the owner is not watching delays "done" for that session; (ii) avoids that by default.
- The app's own stream-json (app-driven) sessions should never be held: they take replies as user messages (FR-109..FR-112); the hook answers "no reply" at once for any session that is not marked.
- Install the entry with an explicit `timeout` (cap plus a margin). When the owner presses Esc during a hold, the app should treat that as "owner is at the terminal: stop holding this turn" (the hook is gone; nothing to clean up on the Claude side).

## Notes for the build tasks (not decisions)

- The text shows under "Stop hook reported a problem" in the terminal and is stored by Claude Code as an `isMeta` user record, an `attachment` record and the summary record (see section 1). The app's transcript parser must skip `isMeta` records when it counts typed prompts.
- `last_assistant_message` is in the Stop input: the Live view can show what Claude just said without reading the transcript.
- Owner Enter-queued text and a block reason can both be pending: the reason's turn runs first, then the queued text.

## Question for the owner

Is it acceptable that the reply appears in the terminal under the label "Stop hook reported a problem"? It cannot be changed from the hook (the label belongs to Claude Code); the text itself is shown in full below it.
