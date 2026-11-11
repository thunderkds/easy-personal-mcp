#!/bin/sh
# The fake `systemctl` of T034 (the real one is never run). It records each call's arguments, one line per call, to the
# file named by FAKE_SYSTEMCTL_LOG (and does nothing at all when that is unset), and prints a marker on both streams so a
# test can prove the tool's output never reaches the CLI's output.
#   FAKE_SYSTEMCTL_NO_SESSION=1   `--user show-environment` fails: no user session
#   FAKE_SYSTEMCTL_FAIL=<verb>    a call whose verb (the word after --user) is <verb> fails
[ -n "$FAKE_SYSTEMCTL_LOG" ] || exit 99
printf '%s\n' "$*" >> "$FAKE_SYSTEMCTL_LOG"
echo "FAKE-TOOL-OUTPUT"
echo "FAKE-TOOL-OUTPUT" >&2
if [ -n "$FAKE_SYSTEMCTL_NO_SESSION" ] && [ "$*" = "--user show-environment" ]; then exit 1; fi
if [ -n "$FAKE_SYSTEMCTL_FAIL" ] && [ "$2" = "$FAKE_SYSTEMCTL_FAIL" ]; then exit 1; fi
exit 0
