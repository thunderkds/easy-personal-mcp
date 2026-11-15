# Cutover: the Rust app becomes the default (Ubuntu)

**Task**: T062. **Date**: 2026-11-15. **Who runs it**: you, the owner. No agent runs these steps.

After this document, `easy-personal-rs` does the work that `easy-personal` did: the MCP server, the daily ingest timer, the connectors and the dashboard. The Python app stays installed for one week, so you can go back at any time. T063 removes it later.

This covers Ubuntu only. macOS and Windows are not planned.

## How to use this document

- Run the steps in order. Run each command block as written.
- After each block, compare what you see with the "You should see" lines. If it does not match, stop. Do not go on to the next step. Go to "Rollback" if you want to undo.
- Run every command from the folder of this repo, unless the step says otherwise.
- If `cargo` is not found, run this first: `. "$HOME/.cargo/env"`.
- Step 0 stops the ingest timer, the one writer that starts by itself. One program ingests into the store at a time. You can keep your Claude Code sessions open.

## Step 0: checks before anything changes

Nothing is installed in this step. It records what you have now, stops every writer, makes a backup and runs two checks.

**0.1 Record what you have now.** The rollback uses these notes to put things back exactly as they were. Run the `claude` line once for each Claude account. The default account is `~/.claude`. For every other account, put `CLAUDE_CONFIG_DIR="$HOME/.claude-work"` (its folder) in front of the command, and change the file name.

```bash
mkdir -p ~/easy-personal-cutover-notes
grep ExecStart ~/.config/systemd/user/easy-personal-ingest.service | tee ~/easy-personal-cutover-notes/timer.txt
claude mcp get easy-personal | tee ~/easy-personal-cutover-notes/mcp-default.txt
```

You should see: for the timer, one `ExecStart=` line. If it ends with `--retention-days N`, you set a retention. Remember `N`: Step 3 and the rollback use it again. If the file is missing, you had no timer. For `claude`, the registration: `Command:`, `Args:` and any `Environment:` lines. If `claude` says there is no such server, that account had no registration.

**0.2 Stop the ingest timer.**

```bash
systemctl --user stop easy-personal-ingest.timer easy-personal-ingest.service
systemctl --user is-active easy-personal-ingest.timer easy-personal-ingest.service
```

The timer stops because an ingest run that starts during Step 0 would write new events while you back up and check the store, and Step 3 replaces the timer anyway. You do not need to close your Claude Code sessions or the Python dashboard. Step 0.3 uses the SQLite backup call, and that call is safe while other programs read or write the store.

You should see: two lines, `inactive` and `inactive`.

**0.3 Back up the store.** This uses the SQLite backup call, so the backup holds the data that is still in the `-wal` file. A plain `cp` of `store.sqlite` does not.

```bash
python3 -c "import sqlite3,os;p=os.path.expanduser('~/.easy-personal/store.sqlite');s=sqlite3.connect(p);d=sqlite3.connect(p+'.before-rust');s.backup(d);d.close();s.close()"
```

You should see: no output. The file `~/.easy-personal/store.sqlite.before-rust` exists.

**0.4 Build the Rust program and check the store.**

```bash
. "$HOME/.cargo/env"
cargo build --locked --manifest-path rust/Cargo.toml -p easy-personal-rs
rust/target/debug/easy-personal-rs store-check --full
```

`--full` has no time limit. On a large store it can take some seconds, and more on a slow machine. Wait for it.

You should see: the last line is `verdict: healthy`, and the exit code is 0 (`echo $?` prints `0`). The line `schema: user_version 11, code has 11, migration pending: no` shows that Rust reads your store as it is and will not migrate it. If it says the store is newer than the program, stop.

The plain `store-check` (without `--full`) is a quick check. It stops after 1.5 seconds. On a large store it cannot finish in that time, so it prints `verdict: not fully checked (... not finished in time)` and the exit code is 3. This does not mean the store is damaged. It means the check ran out of time. Where you need the verdict, run `store-check --full`.

**0.5 Run the keychain interop test.**

The Rust launcher must read the secrets that Python stored in the keychain. This test proves it. It writes two throwaway items in your real Secret Service (`easy-personal/t050-crosscheck-...`), reads each one with the other program, and deletes both. Unlock the keyring first.

```bash
cd rust
EASY_PERSONAL_PYTHON=~/.local/share/uv/tools/easy-personal-mcp/bin/python cargo test -p easy-personal-rs --lib real_keychain_items_cross_check -- --ignored
cd ..
```

You should see: `test result: ok. 1 passed`. If it says `0 passed`, the test did not run: stop and tell the Supervisor.

**If this test fails, the cutover stops here.** Do not run Step 1. Without it, Rust cannot read your connector secrets. Tell the Supervisor what the test printed. Start the timer again with `systemctl --user start easy-personal-ingest.timer`, and reopen your Claude Code sessions.

## Step 1: install

**1.1 Build the dashboard page.** Vite writes it to `src/easy_personal/dashboard/static`.

```bash
npm --prefix web ci && npm --prefix web run build
```

You should see: `built in ...` and no error.

**1.2 Build and install the Rust program.** This is a release build with the window. It takes several minutes. `--target-dir rust/target` reuses the build folder of this repo and keeps big files out of `/tmp`.

```bash
cargo install --locked --path rust/crates/app --features window --target-dir rust/target --root ~/.local/share/easy-personal-rs
```

You should see: `Installed package`, then two programs: `easy-personal-launch` and `easy-personal-rs`. Both are now in `~/.local/share/easy-personal-rs/bin/`.

The `window` feature needs the Ubuntu packages `libwebkit2gtk-4.1-0` and `libgtk-3-0t64` at run time, even for `mcp` and `serve`. You installed them for the window checks.

**1.3 Copy the dashboard page next to the program.**

```bash
mkdir -p ~/.local/share/easy-personal-rs/static
cp -r src/easy_personal/dashboard/static/. ~/.local/share/easy-personal-rs/static/
```

You should see: no output. `~/.local/share/easy-personal-rs/static/index.html` exists.

**1.4 Link the program into `~/.local/bin`.**

```bash
ln -sf ~/.local/share/easy-personal-rs/bin/easy-personal-rs ~/.local/bin/easy-personal-rs
easy-personal-rs --version
```

You should see: `easy-personal-rs 0.3.0`.

This replaces the old file `~/.local/bin/easy-personal-rs` (version 0.1.0) with a link. Link only this one program. Do **not** link `easy-personal-launch`: that name in `~/.local/bin` stays Python's, so a rollback keeps working.

The Rust program still finds its own launcher. It looks for `easy-personal-launch` next to its real file, and Linux gives it the real path, not the link. So it uses `~/.local/share/easy-personal-rs/bin/easy-personal-launch`.

## Step 2: the MCP server

Repeat this step for each Claude account you confirmed. List them with `easy-personal-rs account list`. The default account is `~/.claude`. For every other account, put `CLAUDE_CONFIG_DIR="$HOME/.claude-work"` (its folder) in front of each `claude` command. You saved the old registration of each account in Step 0.1.

```bash
claude mcp remove easy-personal --scope user
claude mcp add --scope user easy-personal -- ~/.local/bin/easy-personal-rs mcp
claude mcp get easy-personal
```

You should see: for `get`, the lines `Type: stdio`, `Command: /home/<you>/.local/bin/easy-personal-rs` and `Args: mcp`.

If `remove` says the server does not exist, go on with `add`. Restart Claude Code after you change an account.

## Step 3: the daily ingest timer

The Rust timer uses the same unit names as the Python timer. It replaces the Python timer. You keep one timer, not two. Step 0.2 stopped the timer. The install enables it and starts it again.

If the `ExecStart=` line that you saved in Step 0.1 ended with `--retention-days N`, add the same `--retention-days N` to both `schedule install` commands. Otherwise leave it out.

```bash
easy-personal-rs schedule install --dry-run
easy-personal-rs schedule install
systemctl --user list-timers | grep easy-personal
```

You should see:

- The dry run prints two unit files and writes nothing. The line `ExecStart=` ends with `easy-personal-rs ingest`, and its path starts with `/home/<you>/.local/share/easy-personal-rs/bin/`.
- The install prints `installed easy-personal-ingest.timer in /home/<you>/.config/systemd/user`.
- `list-timers` shows one row for `easy-personal-ingest.timer`, with a next run time. If the row is missing, run `systemctl --user start easy-personal-ingest.timer`.
- If you passed `--retention-days N`, the `ExecStart=` line ends with `ingest --retention-days N`.

Do not move or delete `~/.local/share/easy-personal-rs/bin/`. The timer runs the program from there.

If the machine has no systemd user session, the install writes nothing, prints a cron line and exits with 1. Add that line yourself with `crontab -e`.

## Step 4: connectors

```bash
easy-personal-rs connector sync
easy-personal-rs doctor
```

You should see:

- `connector sync` prints the plan for each confirmed account. Most often it prints `nothing to change.` Read any plan it prints. If you agree, answer `y`.
- `doctor` prints `ok keychain: SecretService`, `ok claude: on PATH` and `ok launcher: /home/<you>/.local/share/easy-personal-rs/bin/easy-personal-launch`, then one `ok` line for each connector. Its exit code is 0.

Your existing connector entries in Claude Code run `easy-personal-launch exec <id>`. `connector sync` accepts an entry when the program name is `easy-personal-launch`, whatever its folder. So the entries keep using `~/.local/bin/easy-personal-launch`, which is Python's launcher. This is on purpose for the week: both launchers read the same keychain items, and a rollback needs no connector change. T063 must point the entries at the Rust launcher before it removes Python.

New connectors that you add with `easy-personal-rs connector add` are registered with the Rust launcher.

## Step 5: the dashboard

```bash
easy-personal-rs dashboard --static-dir ~/.local/share/easy-personal-rs/static
```

You should see: the line `ingesting before start (waits if another ingest runs)...`, then two lines, `dashboard: http://127.0.0.1:<port>/#<token>` and `Ctrl-C to stop.`

Rust does not open the browser. Copy the whole URL, with the part after `#`, into your browser. Press Ctrl-C to stop.

To use the window instead of the browser (it needs a display):

```bash
easy-personal-rs app --static-dir ~/.local/share/easy-personal-rs/static
```

You should see: a window titled `easy-personal` with the same page. Close the window to stop.

## The week

Use the Rust commands for one week. Do not run Python's `easy-personal ingest` or `easy-personal dashboard` in this time.

Each day, check these things:

- **Ingest ran.** `systemctl --user list-timers | grep easy-personal` shows a last run from the last 24 hours. `easy-personal-rs store-check --full` ends with `verdict: healthy`. (The plain `store-check` can print `not fully checked` on a large store. That is not a problem, see Step 0.4.)
- **MCP tools answer.** In Claude Code, `/mcp` lists `easy-personal` as connected. Ask Claude for your usage summary and get numbers.
- **The dashboard opens.** Step 5 gives a page with data in Usage, Warnings & plan and Kit.
- **Connectors start.** One tool call of each connector works. `easy-personal-rs doctor` exits 0.

Write down each problem: the day, the command or tool, and the exact message. Give the notes to the Supervisor. A problem that stops your work is a reason to roll back (next section).

After a week with no blocking problem, tell the Supervisor. That is the gate for T063, which removes the Python app. T063 does not start before you say so.

## Rollback

The Python app is still installed, so a rollback takes a few minutes. Run only the parts you need. Your notes from Step 0.1 are in `~/easy-personal-cutover-notes/`.

```bash
uv tool install --reinstall --no-cache .
easy-personal schedule install
claude mcp remove easy-personal --scope user
claude mcp add --scope user easy-personal -- <Command> <Args>
easy-personal connector sync
```

- The first line is only needed if the Python tool is gone. It installs Python's program from this repo.
- `easy-personal schedule install` writes the Python timer over the Rust one (same unit names), enables it and starts it. If your saved `ExecStart=` line ended with `--retention-days N`, add the same `--retention-days N`. Check with `systemctl --user list-timers | grep easy-personal`.
- Run the two `claude mcp` lines for each account. In the `add` line, use the `Command:` and `Args:` from the note of that account (`mcp-default.txt` or the file you saved for it), not the default from the README. For each `Environment:` line in the note, add `-e KEY=value` before the name `easy-personal`. If the note says that account had no registration, run only the `remove` line.
- `easy-personal connector sync` checks the connector entries. They did not change in Step 4, so it should print `nothing to change.`
- Restore the store backup only if a migration happened. None is expected, because Python and Rust both read schema 11. A restore also drops everything the store gained since the backup. The next ingest reads it again from your transcripts, if Claude Code has not deleted them. If you need the restore, do these in order:

```bash
systemctl --user stop easy-personal-ingest.timer easy-personal-ingest.service
pgrep -af easy-personal
rm -f ~/.easy-personal/store.sqlite-wal ~/.easy-personal/store.sqlite-shm
cp -p ~/.easy-personal/store.sqlite.before-rust ~/.easy-personal/store.sqlite
systemctl --user start easy-personal-ingest.timer
```

Before the `rm` line, close every process that `pgrep` prints, and every Claude Code session that uses easy-personal. Delete the `-wal` and `-shm` files only when no process has the store open: a stale `-wal` file left from the Rust week would be applied to the restored store.

You can leave `~/.local/bin/easy-personal-rs` and `~/.local/share/easy-personal-rs` in place.

## Differences you will see

The full list is in [`PARITY_T061.md`](PARITY_T061.md), the T061 parity report. These are the ones that you meet in daily use:

- The command is `easy-personal-rs`, not `easy-personal`. `easy-personal-rs --version` prints `easy-personal-rs 0.3.0`.
- `dashboard` does not open the browser. You open the printed URL.
- The dashboard Connectors page and the MCP tool `connector_status` show every connector as not configured. Use `easy-personal-rs doctor` to check connectors.
- `exec` and `headers` are not commands of `easy-personal-rs`. They belong to the separate program `easy-personal-launch`.
