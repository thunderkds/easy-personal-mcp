# T130 spike: the real `claude` in a PTY, drawn by xterm.js in the browser

**Date**: 2026-11-20 (runs 03:54Z to 04:12Z) · **CLI**: Claude Code 2.1.329 · **Run by**: backend-developer (T130)
**Verdict**: all 7 questions **GO** (Q7 and Q6 with conditions, see the table). The idea works as the owner described it.
What is left is a product decision (it reverses DDR-0006's cut item and PRD question 42's "terminal input stays out"), not a technical doubt.

## How it ran

- Throwaway code only, in the session scratchpad `.../scratchpad/t130/` (not in the repo): a Rust bin `bridge/` (`portable-pty` 0.9.0
  + `axum` 0.8.9 with `ws`), a static page `www/` (`xterm.js` 6.0.0 + `addon-fit` 0.11.0, from npm tarballs whose sha512 matched the
  registry's `dist.integrity`: xterm `TQwDdQGt...3sMOYg==`, fit `jYcgT6xt...PF7g==`; `web/node_modules` does not exist), and a Python
  CDP driver (`cdp.py`, `s2.py`) that ran headless `google-chrome` against the page and typed with real key events.
- Scratch git repo `scratchpad/t130/repo` (`git init`, one commit). The bridge started the account's `claude` (`~/.local/bin/claude`,
  resolved to `versions/2.1.329`) in a PTY (120x32 at start, then the page's size) with `--model haiku` (run 1) and
  `--model haiku --permission-mode default` (run 2); environment = `child_env` of `app_session.rs` (no `CLAUDE_CONFIG_DIR`,
  `CLAUDECODE`, `ANTHROPIC_*`, `CLAUDE_CODE_*`, the FR-102 list; `MCP_TOOL_TIMEOUT` not set; spike-only: also `CLAUDE_EFFORT`,
  `CLAUDE_PID`), plus the signed-in account's `CLAUDE_CONFIG_DIR`, `TERM=xterm-256color`, `COLORTERM=truecolor`.
- Run 1 (bridge pid 1073421, claude pid 1073432) = Q1-Q4, Q6 close. Run 2 (bridge 1137986, claude 1137998) = Q5 hooks, ring wrap, kill.
- Prompts: "Reply with the single word OK." and "Use the Bash tool to run: printenv HOME" (+ one `AskUserQuestion` prompt in run 2).
  The owner's real transcripts and files were not read; only existence, `type` counts, `entrypoint` and the field names of the spike's
  own `sessions/<pid>.json` (values printed: `kind`, `entrypoint`, `status`, `pid`, `version`, `procStart`).
- Screenshots (not committed): `.../scratchpad/t130/shots/` (listed per question). They show the owner's account e-mail in the status line.

## Answers

| # | Question | Verdict | Evidence (UTC) |
|---|---|---|---|
| 1 | PTY starts the interactive `claude` and keeps it running | **GO** | `portable-pty` 0.9.0, MIT, last release 2025-02-11; 18 crates in its tree, 5 new to `rust/Cargo.lock` (`downcast-rs`, `filedescriptor`, `serial2`, `shell-words`, and `nix` 0.28 beside the app's 0.29). Started 03:54:58Z: `claude` is its own session leader (`SID` = pid), controlling tty `pts/17`, up 8 min 48 s later; first screen bytes after 655 ms (13 B), the folder-trust dialog drawn by ~700 ms. |
| 2 | Typed input reaches it; it answers; its own permission dialog is answered by keystrokes | **GO** | 03:57:56Z `ArrowDown`+`Enter` answered the folder-trust dialog (`Yes, I trust this folder`); 03:58:51Z typed the test prompt (one `in 1 bytes` log line per key), `Enter`; 03:59:07Z the TUI showed `● OK`. 04:02:41Z the real **Bash permission dialog** (`Do you want to proceed?`, 4 options) appeared in manual mode; 04:03:12Z `Enter` (= option 1) answered it and the command ran. In run 2 the `AskUserQuestion` picker (options + `Chat about this`) was answered with `Enter` at 04:11:16Z. Screenshots `q2-after-trust-1280.png`, `q2-after-ok-1280.png`, `q2-permission-dialog-1280.png`, `q2-run2-askuserquestion-ui-1280.png`. Note: this account starts in *auto mode*, where the same Bash call ran with no dialog; a build must not assume a dialog. |
| 3 | Renders correctly in xterm.js over a local bridge: colours, cursor, resize | **GO** | Colours (24-bit status line, syntax-style highlights), emoji, box lines, inverse selection bars and the block cursor drew correctly (`q3-first-screen-1280.png`, `q2-after-ok-1280.png`, `q3-idle-prompt-1280.png`). Resize: viewport 1280 -> 375 -> 768 -> 1280 px gave xterm 149x48 -> 40x47 -> 88x48 -> 149x48 and `stty size` on the pty read the same each time (SIGWINCH reached `claude`, which reflowed: `q3-permission-dialog-375.png`). Page served with the app's exact CSP (`default-src 'self'; ... style-src 'self' 'unsafe-inline'`): no violation; `ws://127.0.0.1:<port>` is allowed by `'self'`, no `connect-src` needed; `xterm.js` needs no eval or worker. WebSocket echo (key to screen update, 30 samples, in page): median 5.1 ms, p95 8.4, max 11.1. SSE + POST: median 7.5 ms, p95 14.5, max 21.2. **Not run here:** the Tauri/WebKitGTK window and a real phone keyboard. |
| 4 | Saved as an interactive session; `--resume <id> -p` resumes it | **GO** | Project folder `projects/<munged scratch path>/` has `<id>.jsonl`, 83 records of types `mode`, `permission-mode`, `session-owner`, `file-history-snapshot`, `user`, `assistant`, `attachment`, `system`, `ai-title`, `last-prompt`, `queue-operation`, `tools-used` and some internal ones; **every `entrypoint` = `cli`** (36 records), version 2.1.329. So it is *typed work* (FR-039), not *agent work*. 04:06:35Z `claude --resume <id> -p "Reply with the single word OK." --model haiku` (env scrubbed, same cwd) exited 0, printed `OK`, appended to the same file (98 records; the new ones `entrypoint` = `sdk-cli`), same session id. |
| 5 | Live sees it as a terminal session; the answer-bridge hooks fire | **GO** | `<config>/sessions/<pid>.json` appeared once the trust dialog was answered (not before) with these fields: `pid`, `sessionId`, `cwd`, `startedAt`, `procStart` (`'1965201'`), `version`, `peerProtocol`, `peerFeatures`, `kind` = `interactive`, `entrypoint` = `cli`, `pidDomain`, `asksForConvertedSkills`, `messagingSocketPath`, `name`, `nameSource`, `nameSince`, `status` = `idle`, `updatedAt`, `statusUpdatedAt`; a `<pid>.<hash>.key` file beside it (existence only; not opened). `session_procs.rs` reads `pid`, `sessionId`, `procStart`, `status`, `name`, `kind`, `entrypoint`: **all seven present**, so Live shows it exactly like a terminal session and `Stop` works. The file is removed by `claude` on exit. **Hooks fired** (run 2, three throwaway hooks in the scratch repo's `.claude/settings.json`, not the real config; the hook only logged the event name and key names): `PermissionRequest` (Bash) 04:07:13.75Z, `Stop` 04:07:21.19Z (`stop_hook_active=False`), `PreToolUse` (`AskUserQuestion`) 04:09:07.80Z, `PermissionRequest` (`AskUserQuestion`) 04:11:07.94Z, `Stop` 04:11:18.33Z. The `PermissionRequest` input keys: `cwd, effort, hook_event_name, permission_mode, permission_suggestions, prompt_id, scratchpad_dir, session_id, tool_input, tool_name, transcript_path` (no `tool_use_id`, as `answer_bridge.rs` already assumes). Also seen in the TUI spinner line (`running Stop hooks… 3/4`, `running PreToolUse hooks… 2/3`): the account's own user-scope hooks ran for the PTY session, so FR-113's user-scope entries would too. **By reading, not run**: `hook ask` with the real entries; FR-121's hold would be pointless for a PTY session (the owner can type), and its `entrypoint starts with sdk` skip does not apply (`cli`). |
| 6 | Closing the PTY ends `claude` cleanly; nothing left; re-attach from an in-memory buffer | **GO** (conditional) | Run 1, 04:06:10Z `POST /quit` = drop the PTY master: `claude` got the hangup, wrote its last redraw, removed `sessions/<pid>.json` and the key file and was gone within ~6 s (the bridge's 5 s wait ended first; the owner's SessionEnd-style hooks may account for the time). Run 2, 04:12:17Z `kill -9` of the **bridge**: `claude` gone within 1 s, files removed, mid-turn. `pgrep` for the bridge, `claude` of the scratch repo and the Chrome profile: none left. Re-attach: a page reload (8 times, also a headless Chrome kill + new one) replayed the in-memory ring and the screen was identical (`q6-reattached-after-reload-1280.png`); 138 KB replayed in about 1 s. **Sizes**: run 1 produced 165,926 B in 12 min (3 turns, several resizes): ~14 KB/min; while the TUI only showed its spinner the rate was ~370 B/s (3.7 KB in 10 s), i.e. 1.3 MB/h. A 64 KiB ring (run 2, 87,791 B total) **wrapped** and the replay was torn (a stray `5` at the top, header and history gone, a broken status line: `q6-wrapped-ring-replay-1280.png`); a resize nudge (cols-1 then back, SIGWINCH) made `claude` redraw everything and the screen was whole again (`q6-wrapped-ring-after-resize-nudge-1280.png`). So: ring of 1 MiB per session (about 45 min of spinner), and always nudge after replay. |
| 7 | Page cost of `xterm.js` + fit addon | **GO** (conditional) | `@xterm/xterm` 6.0.0, MIT, 488,663 B raw / **120,455 B gzip** (+ `xterm.css` 7,112 / 2,512 gz); `@xterm/addon-fit` 0.11.0, MIT, 1,521 / 657 gz; no runtime dependencies; together ~123.6 KB gzip. `scripts/check_static.py` `check_no_remote` on the raw files **fails**: `sourcemap present` (both `.js` end in `//# sourceMappingURL`) and three URL strings (`github.com/chjj/term.js`, `bellard.org/jslinux` in the CSS; `developer.mozilla.org/.../Secure_Contexts` in `xterm.js`); stripping the source-map comment leaves only the three URLs. They are in comments or docs strings, and a Vite build usually drops comments, but that was not run (`npm ci` needs the owner's OK): a build task must run `check_static.py` and, if needed, add those prefixes to `ALLOWED_URL_PREFIXES` (they are never fetched). The page also needs the lockfile pin (exact versions, `npm ci`). |

## The two routes, measured

| | WebSocket | SSE + POST |
|---|---|---|
| Order of typed bytes | kept (186 single-key sends: 186/186 in order) | **not kept** when sent without waiting: 4 bursts of 186 keys, all reordered (138 of 186 in place in the one that was counted); the page would have to queue its POSTs |
| Echo latency | median 5.1 ms | median 7.5 ms |
| Token | a browser `WebSocket` cannot set headers: the token goes in the **first frame**, never in the URL (matches FR-116 / NFR-008) | `EventSource` needs the query-token carve-out `GET /api/live` already has (T056) and a header token on each POST |
| New crates | `tungstenite`, `tokio-tungstenite`, `data-encoding`, `utf-8` (axum `ws`) | none beyond `base64` |
| Guard tests (04:03Z) | evil `Origin` -> 403; rebinding `Host` -> 403; wrong token -> closed with no data; no auth frame -> no data (closed after 5 s) | no token -> 401; wrong token -> 401; token + evil `Origin` on POST -> 403 |

Recommendation: **WebSocket**, first-frame token, `Host`/`Origin` checked on the upgrade by the existing guard layer. Typed bytes are the most
dangerous write the app would have (any command the owner's `claude` may run), so they should not ride on a URL token or reorder.

## What a build would need

1. **PTY owner** in `app` (new module beside `app_session.rs`): `openpty` + spawn `claude` as session leader with `child_env` and the
   FR-109 form fields (repo, account, model, mode), no free arguments (FR-116); reader thread -> ring buffer + `broadcast`; waiter thread -> exit event.
   Alternative to `portable-pty`: `nix::pty` with the app's nix 0.29 (not tried here).
2. **Lifecycle with the tray**: `PR_SET_PDEATHSIG`-style like app sessions, so quitting the tray closes the PTY (claude exits in <= 6 s, as run 2 showed on kill);
   cap of 4 running (question 41); *Stop* = close the master, then SIGKILL the group after 5 s. A separate daemon that outlives the tray is a bigger decision (not needed for the idea).
3. **Route**: one guarded `GET /api/term/{id}` WebSocket (upgrade passes the Host/Origin guard; first frame = token + size), `POST` to start/stop with `confirmed: true`.
   Binary frames in = stdin bytes, text frames = `{"t":"resize"}`; out = PTY bytes, `{"t":"exit"}`. Per-session byte cap on input per second is cheap insurance.
4. **Buffer**: 1 MiB ring per session in memory only (FR-115 family, a sixth NFR-002 exception: raw screen bytes incl. tool output and secrets, shown never stored); dropped when the session ends and its panel closes, and at exit. After replay, always send a resize nudge. Private-mode settings set at start (bracketed paste, focus reporting) are lost when the ring wraps: keep a small prefix or accept the loss.
5. **Several clients**: one PTY size only; the last `resize` wins (the guard test attached an 80x24 client and shrank the live session until the next resize). Decide: one active attached page, others read-only.
6. **Page**: Live "New session" gets a *Terminal* vs *Chat* choice; `xterm.js` + fit addon in the panel, on-screen keys for Esc, Tab, Ctrl-C, arrows (a phone has none). CSP unchanged. Needs a UX pass at 375 px.
7. **Live and the rest**: the PTY session shows in Live by itself through `sessions/<pid>.json` (all seven fields present) and the transcript route (`entrypoint` `cli`); *Stop* is simpler (the app owns the process). Stream-json app sessions stay (structured cards, `set_model`/`set_permission_mode`/`get_context_usage`/effort from T126, interrupt and queue of T127, Allow/Deny cards): none of that exists for a PTY session, whose controls are the TUI's own slash commands. The answer bridge (FR-113/FR-121) is not needed for sessions the app owns.

## Risks

- **Security surface**: a page fault or a token leak that passes the guard types into a shell-capable process (stronger than FR-116's "no free arguments"). Mitigation = WebSocket first-frame token, Host/Origin check on the upgrade, `confirmed`, no start without a click; the leak suite must cover the stream.
- **Spike side effects on the owner's account** (Claude Code's own writes, cannot be undone by the spike): a project folder with two session files under `projects/<munged scratch path>/`; the folder-trust entry of the scratch repo in the account's `.claude.json`; whatever the owner's own Stop/PreToolUse hooks did for these two sessions (4 Stop and 3 PreToolUse hooks ran; they added about 1.5 min per turn; their names were not read). Suggest the owner deletes the scratch project folder.
- **The TUI runs the owner's whole environment** (their hooks, statusline command, MCP servers) inside the app window, with a ~1.5 min "Stop hooks" delay per turn on this account that the stream-json path does not show (a `-p` resume took ~5 s).
- **Fidelity outside Chrome**: WebKitGTK (the app window) and real phones not tested.
- **Auto mode hides dialogs**: this account runs in auto mode by default; prompts the app might want to surface (FR-111 cards) never appear as structured data in a PTY session.
- **Ring wrap tears a replay** unless a resize nudge follows; spinner traffic fills the ring (~1.3 MB/h).
- **`portable-pty` is unmaintained-ish** (last release 2025-02) and adds a second `nix`.
- **A reversal**: DDR-0006 cut item, PRD "Write sessions" cut list ("embedding the interactive terminal UI (a PTY)"), question 42 ("tmux/terminal input stays out") and Path C of `BRAINSTORMING_LOG_LIVENATIVE.md` ("question 42 already put tmux out") need a DDR before any build.

## Open questions for the owner

1. Go to a build? If yes: a DDR that reverses DDR-0006's cut item and amends FR-116 (typed bytes are free text) and NFR-002 (a sixth exception: raw terminal bytes in memory).
2. Terminal **instead of** or **beside** stream-json app sessions? (Beside keeps T126/T127 features for the Chat choice.)
3. Should sessions outlive the tray (needs a daemon) or end with it (as app sessions do)?
4. Several pages attached to one PTY: allowed, and who controls the size?
5. Is the 1.5 min of owner hooks per turn acceptable inside the app window, (the owner said parity, so no settings are skipped)?
6. May the build task run `npm ci` for `@xterm/xterm` + `@xterm/addon-fit` and extend `ALLOWED_URL_PREFIXES` if the build keeps those URL strings?
