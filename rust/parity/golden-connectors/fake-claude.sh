#!/bin/sh
# The fake `claude` of the T050 oracle: `gen_connectors.py` (Python) and `connector_parity.rs` (Rust) put this same file
# first on PATH as `claude`. The real CLI is never run.
#   FAKE_CLAUDE_LOG    each call appends `CCD=<CLAUDE_CONFIG_DIR, or <unset>>`, one line per argument, then `--`
#   FAKE_CLAUDE_STATE  a folder: `<state>/<key>/<name>` is one user-scope MCP entry of that account, two lines
#                      `Command: <command>` and `Args: <args joined by a space>` (what `claude mcp get` prints); `<key>` is the
#                      CCD value with every byte outside `[A-Za-z0-9._-]` as `_`, and `<state>/<key>/.ccd` holds the value itself
#   FAKE_CLAUDE_FAIL   `<subcommand> <name>` (e.g. `add-json acme-slack`) fails: exit 1, and `boom <FAKE_CLAUDE_STDERR>` on
#                      stderr when FAKE_CLAUDE_STDERR is not empty
ccd="${CLAUDE_CONFIG_DIR-<unset>}"
record="CCD=$ccd"
for a in "$@"; do record="$record
$a"; done
# one write per call: calls that run at once (legacy detection asks in parallel) never interleave their lines
printf '%s\n--\n' "$record" >> "$FAKE_CLAUDE_LOG"
key=$(printf '%s' "$ccd" | LC_ALL=C tr -c 'A-Za-z0-9._-' '_')
dir="$FAKE_CLAUDE_STATE/$key"
[ "$1" = mcp ] || exit 2
sub=$2
name=$3
if [ "$sub" = get ]; then
  [ -f "$dir/$name" ] || exit 1
  printf '%s:\n  Scope: User config\n  Type: stdio\n' "$name"
  sed 's/^/  /' "$dir/$name"
  exit 0
fi
if [ "${FAKE_CLAUDE_FAIL-}" = "$sub $name" ]; then
  # stderr only when asked: Python shows it as a line, Rust never shows it (the leak gate plants a canary here)
  [ -n "${FAKE_CLAUDE_STDERR-}" ] && printf 'boom %s\n' "$FAKE_CLAUDE_STDERR" >&2
  exit 1
fi
case $sub in
  add-json)
    [ -f "$dir/$name" ] && exit 1
    mkdir -p "$dir"
    printf '%s' "$ccd" > "$dir/.ccd"
    cmd=$(printf '%s' "$4" | sed -n 's/.*"command": "\([^"]*\)".*/\1/p')
    args=$(printf '%s' "$4" | sed -n 's/.*"args": \["\([^"]*\)", "\([^"]*\)"\].*/\1 \2/p')
    printf 'Command: %s\nArgs: %s\n' "$cmd" "$args" > "$dir/$name"
    ;;
  remove)
    [ -f "$dir/$name" ] || exit 1
    rm "$dir/$name"
    ;;
  *) exit 2 ;;
esac
