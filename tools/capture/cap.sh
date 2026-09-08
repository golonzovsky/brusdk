#!/bin/sh
# cap.sh <session-dir> <label> <drive.mjs args...>
# e.g. tools/capture/cap.sh captures/2026-09-08-s1 dot-r0c0-64 print --image probes/dot-r0c0-64.png
set -eu
here=$(cd "$(dirname "$0")" && pwd)
session=$1; label=$2; shift 2
mkdir -p "$session"
BRADY_CAPTURE_MODULES="$HOME/.claude/skills/brady-m511/scripts" BRADY_CAPTURE="$session" BRADY_CAPTURE_LABEL="$label" exec node --import "$here/hook.mjs" "$here/drive.mjs" "$@" 2>&1 | tee -a "$session/$label.log"
