#!/usr/bin/env bash
# T097: the real-machine smoke check. One command, one PASS / FAIL / SKIP line per step, exit 1 on any FAIL.
#
#   rust/scripts/smoke_real.sh                       build, tests, notification, window; the real `claude` run is skipped
#   rust/scripts/smoke_real.sh --claude <account>    also run `agent-run now --account <account>` (spends plan limits)
#   rust/scripts/smoke_real.sh --only notification,window   run only the named steps (build tests notification window claude)
#
# Steps:
#   build         cargo build --locked -p easy-personal-rs --features window
#   tests         cargo test --workspace --no-fail-fast (passed / failed counts, names of failed tests)
#   notification  a throw-away HOME with one session waiting on AskUserQuestion, `serve --no-ingest --alerts`, and
#                 `dbus-monitor` on the session bus: PASS when the Notify call names the app icon and the desktop entry
#   window        `app` under XWayland (GDK_BACKEND=x11): PASS when `xprop` shows _NET_WM_ICON and a WM_CLASS equal to the
#                 StartupWMClass of rust/packaging/easy-personal.desktop
#   claude        only with --claude: remembers the agent run setting, turns it on, runs `agent-run now --account <account>`,
#                 puts the setting back (also on failure or Ctrl-C); PASS when the outcome line says `ok`
#
# A missing optional tool (dbus-monitor, xprop), no session bus or no display makes that step SKIP, never FAIL.
# Only the claude step touches the owner's store (that is its point). Every temp folder and every process this script
# starts is removed or stopped on exit.
#
# Environment: EP_SMOKE_BIN (the binary to check; default rust/target/debug/easy-personal-rs), EP_STATIC_DIR (the built
# dashboard page; default <repo>/src/easy_personal/dashboard/static).
#
# `cargo test` rebuilds target/debug/easy-personal-rs without the window feature, so the build runs again, quietly,
# before the notification and window steps (when the tests ran in this same run).
set -u

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
rust_dir=$(dirname "$here")
repo=$(dirname "$rust_dir")
if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi

claude_account=""
only=""
while [ $# -gt 0 ]; do
    case $1 in
    --claude)
        [ $# -ge 2 ] && [ -n "$2" ] || { echo "smoke_real: --claude needs an account id or label" >&2; exit 2; }
        claude_account=$2
        shift 2
        ;;
    --only)
        [ $# -ge 2 ] || { echo "smoke_real: --only needs a list of steps" >&2; exit 2; }
        only=$2
        shift 2
        ;;
    *)
        echo "usage: smoke_real.sh [--claude <account id or label>] [--only build,tests,notification,window,claude]" >&2
        exit 2
        ;;
    esac
done

bin=${EP_SMOKE_BIN:-$rust_dir/target/debug/easy-personal-rs}
static_dir=${EP_STATIC_DIR:-$repo/src/easy_personal/dashboard/static}
work=$(mktemp -d "${TMPDIR:-/tmp}/smoke-real-XXXXXX")
pids=""            # processes this script started (never anything else)
agent_restore=""   # "off" while the agent run setting must be put back to off
fails=0
tests_ran=0

stop_pids() {
    local p
    for p in $pids; do
        kill "$p" 2>/dev/null || true
    done
    for p in $pids; do
        wait "$p" 2>/dev/null || true
    done
    pids=""
}

# shellcheck disable=SC2329 # runs through the trap
cleanup() {
    stop_pids
    if [ "$agent_restore" = off ]; then
        "$bin" agent-run off >/dev/null 2>&1 || echo "smoke_real: could not turn the agent run back off; run: $bin agent-run off" >&2
        agent_restore=""
    fi
    rm -rf "$work"
}
trap cleanup EXIT
trap 'echo "interrupted" >&2; exit 130' INT TERM

pass() { echo "PASS $1: $2"; }
skip() { echo "SKIP $1 ($2)"; }
fail() {
    echo "FAIL $1: $2"
    fails=$((fails + 1))
}

wanted() {
    [ -z "$only" ] && return 0
    case ",$only," in *",$1,"*) return 0 ;; esac
    return 1
}

build_window() {
    (cd "$rust_dir" && cargo build --locked -p easy-personal-rs --features window)
}

step_build() {
    if build_window >"$work/build.log" 2>&1; then
        pass build "cargo build --locked -p easy-personal-rs --features window"
    else
        fail build "cargo build failed; last lines: $(tail -n 5 "$work/build.log" | tr '\n' ' ')"
    fi
}

step_tests() {
    tests_ran=1
    (cd "$rust_dir" && cargo test --workspace --no-fail-fast) >"$work/tests.log" 2>&1
    local code=$?
    local counts passed failed names
    counts=$(awk '/^test result:/ { p += $4; f += $6 } END { print p + 0, f + 0 }' "$work/tests.log")
    passed=${counts% *}
    failed=${counts#* }
    names=$(sed -n 's/^test \(.*\) \.\.\. FAILED$/\1/p' "$work/tests.log" | sort -u | tr '\n' ',' | sed 's/,$//; s/,/, /g')
    if [ "$code" -eq 0 ] && [ "$failed" -eq 0 ] && [ "$passed" -gt 0 ]; then
        pass tests "$passed passed, 0 failed"
    elif [ "$failed" -gt 0 ]; then
        fail tests "$passed passed, $failed failed: $names"
    else
        fail tests "cargo test exited $code with $passed passed, $failed failed; last lines: $(tail -n 5 "$work/tests.log" | tr '\n' ' ')"
    fi
}

# What the Notify call of a session that waits on a question carried: `icon=<app icon> hint=<desktop-entry hint>`, or `none`.
notify_fields() {
    python3 -I - "$1" <<'PY'
import re, sys
text = open(sys.argv[1], errors="replace").read()
for call in re.split(r"(?m)^(?=method call )", text):
    if "member=Notify" not in call or "project smokereal" not in call:
        continue
    strings = re.findall(r'(?m)^   string "(.*)"$', call)  # top level only: name, icon, summary, body
    hint = re.search(r'string "desktop-entry"\s+variant\s+string "(.*)"', call)
    print("icon=%s hint=%s" % (strings[1] if len(strings) > 1 else "?", hint.group(1) if hint else ""))
    sys.exit(0)
print("none")
PY
}

step_notification() {
    if ! command -v dbus-monitor >/dev/null 2>&1; then
        skip notification "dbus-monitor is not installed"
        return
    fi
    if [ -z "${DBUS_SESSION_BUS_ADDRESS:-}" ] && [ ! -S "${XDG_RUNTIME_DIR:-/nonexistent}/bus" ]; then
        skip notification "no session bus"
        return
    fi
    if [ ! -x "$bin" ]; then
        fail notification "no binary at $bin (run the build step first)"
        return
    fi
    local h=$work/notify
    mkdir -p "$h/.claude/projects/p" "$h/config"
    cp "$rust_dir/parity/store-v11.sqlite" "$h/store.sqlite"
    python3 -I - "$h" <<'PY'
import sqlite3, sys
h = sys.argv[1]
c = sqlite3.connect(f"{h}/store.sqlite")
c.execute("INSERT INTO agent_accounts (id, provider, folder, label, status) VALUES (1,'claude',?,'Work','confirmed')", (f"{h}/.claude",))
c.commit()
PY
    dbus-monitor --session "type='method_call',interface='org.freedesktop.Notifications',member='Notify'" >"$h/dbus.txt" 2>&1 &
    local mon=$!
    pids="$pids $mon"
    sleep 1
    HOME=$h XDG_CONFIG_HOME=$h/config EASY_PERSONAL_HOME=$h EASY_PERSONAL_AGENT_ROOT=$h "$bin" serve --no-ingest --alerts >"$h/serve.out" 2>"$h/serve.err" &
    local srv=$!
    pids="$pids $srv"
    sleep 2
    local stamp
    stamp=$(date -u +%Y-%m-%dT%H:%M:%S.000Z)
    printf '%s\n' "{\"type\":\"assistant\",\"sessionId\":\"smoke-real-0001\",\"timestamp\":\"$stamp\",\"cwd\":\"/tmp/smokereal\",\"entrypoint\":\"cli\",\"message\":{\"id\":\"m1\",\"model\":\"x\",\"content\":[{\"type\":\"tool_use\",\"id\":\"b1\",\"name\":\"AskUserQuestion\",\"input\":{\"questions\":[{\"question\":\"smoke\"}]}}]}}" >>"$h/.claude/projects/p/s.jsonl"
    local fields=none
    for _ in $(seq 1 30); do
        fields=$(notify_fields "$h/dbus.txt")
        [ "$fields" != none ] && break
        kill -0 "$srv" 2>/dev/null || break
        sleep 1
    done
    stop_pids
    if [ "$fields" = none ]; then
        fail notification "no Notify call for the waiting session within 30 s; serve said: $(tail -n 2 "$h/serve.err" | tr '\n' ' ')"
    elif [ "$fields" = "icon=easy-personal hint=easy-personal" ]; then
        pass notification "Notify call carried app icon easy-personal and desktop-entry easy-personal"
    else
        fail notification "Notify call carried ${fields} (want icon=easy-personal hint=easy-personal)"
    fi
}

# The window ids of process $1 on the X display, one per line.
windows_of() {
    local id pid
    for id in $(xprop -root _NET_CLIENT_LIST 2>/dev/null | tr -d ',' | grep -o '0x[0-9a-f]*'); do
        pid=$(xprop -id "$id" _NET_WM_PID 2>/dev/null | grep -o '[0-9]*$')
        [ "$pid" = "$1" ] && echo "$id"
    done
}

step_window() {
    if ! command -v xprop >/dev/null 2>&1; then
        skip window "xprop is not installed"
        return
    fi
    if [ -z "${DISPLAY:-}" ]; then
        skip window "no display (DISPLAY is not set)"
        return
    fi
    if [ ! -f "$static_dir/index.html" ]; then
        skip window "no built dashboard page at $static_dir; build it or set EP_STATIC_DIR"
        return
    fi
    if [ ! -x "$bin" ]; then
        fail window "no binary at $bin (run the build step first)"
        return
    fi
    local want
    want=$(sed -n 's/^StartupWMClass=//p' "$rust_dir/packaging/easy-personal.desktop" | head -n 1)
    if [ -z "$want" ]; then
        fail window "rust/packaging/easy-personal.desktop has no StartupWMClass line"
        return
    fi
    local h=$work/window
    mkdir -p "$h/config"
    GDK_BACKEND=x11 HOME=$h XDG_CONFIG_HOME=$h/config EASY_PERSONAL_HOME=$h EASY_PERSONAL_AGENT_ROOT=$h \
        "$bin" app --no-ingest --static-dir "$static_dir" >"$h/app.out" 2>"$h/app.err" &
    local app=$!
    pids="$pids $app"
    local id=""
    for _ in $(seq 1 25); do
        id=$(windows_of "$app" | head -n 1)
        [ -n "$id" ] && break
        kill -0 "$app" 2>/dev/null || break
        sleep 1
    done
    if [ -z "$id" ]; then
        local code=running
        kill -0 "$app" 2>/dev/null || { wait "$app" 2>/dev/null; code="exited $?"; }
        stop_pids
        fail window "no window within 25 s ($code); app said: $(tail -n 2 "$h/app.err" | tr '\n' ' ')"
        return
    fi
    local klass icon
    klass=$(xprop -id "$id" WM_CLASS 2>/dev/null)
    if xprop -id "$id" _NET_WM_ICON 2>/dev/null | grep -q 'Icon ('; then icon=yes; else icon=no; fi
    stop_pids
    # WM_CLASS is `"instance", "Class"`; the shell tries both against StartupWMClass.
    if [ "$icon" != yes ]; then
        fail window "_NET_WM_ICON not set; $klass"
    elif ! echo "$klass" | grep -Eq "\"$want\"( |,|$)"; then
        fail window "$klass does not hold StartupWMClass \"$want\"; _NET_WM_ICON set"
    else
        pass window "_NET_WM_ICON set; $klass equals StartupWMClass $want"
    fi
}

step_claude() {
    if [ -z "$claude_account" ]; then
        skip claude "spends plan limits; pass --claude <account>"
        return
    fi
    if [ ! -x "$bin" ]; then
        fail claude "no binary at $bin (run the build step first)"
        return
    fi
    local before
    before=$("$bin" agent-run status 2>&1 | sed -n 's/^agent run: //p')
    if [ "$before" != on ] && [ "$before" != off ]; then
        fail claude "cannot read the agent run setting"
        return
    fi
    # The trap puts the setting back, also on Ctrl-C.
    [ "$before" = off ] && agent_restore=off
    if ! "$bin" agent-run on >/dev/null 2>&1; then
        fail claude "cannot turn the agent run on"
        return
    fi
    local line code
    line=$("$bin" agent-run now --account "$claude_account" 2>&1)
    code=$?
    if [ "$agent_restore" = off ]; then
        "$bin" agent-run off >/dev/null 2>&1 || echo "smoke_real: could not turn the agent run back off" >&2
        agent_restore=""
    fi
    line=$(echo "$line" | tail -n 1)
    if [ "$code" -eq 0 ] && echo "$line" | grep -q '^investigate: ok'; then
        pass claude "$line (setting back to $before)"
    else
        fail claude "exit $code: $line (setting back to $before)"
    fi
}

wanted build && step_build
wanted tests && step_tests
if [ "$tests_ran" -eq 1 ] && { wanted notification || wanted window; }; then
    build_window >"$work/rebuild.log" 2>&1 || fail build "the quiet rebuild after the tests failed: $(tail -n 3 "$work/rebuild.log" | tr '\n' ' ')"
fi
wanted notification && step_notification
wanted window && step_window
wanted claude && step_claude

if [ "$fails" -eq 0 ]; then
    echo "smoke_real: no FAIL"
    exit 0
fi
echo "smoke_real: $fails FAIL"
exit 1
