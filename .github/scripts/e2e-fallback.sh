#!/usr/bin/env bash
# `--paste` must fall through a present-but-failing tool (a fake wtype) to the next candidate
# (a fake ydotool), and report every attempt when all of them fail.
set -euo pipefail
bin=$(realpath "${BIN:-target/release/clip-md-html}")
fakes=$(mktemp -d)
log="$fakes/calls.log"
printf '#!/bin/sh\necho wtype >> "%s"\necho "virtual keyboard protocol not supported" >&2\nexit 1\n' "$log" > "$fakes/wtype"
printf '#!/bin/sh\necho "ydotool $*" >> "%s"\nexit 0\n' "$log" > "$fakes/ydotool"
chmod +x "$fakes/wtype" "$fakes/ydotool"
printf '# Hi' | xclip -selection clipboard -i 2>/dev/null || true
# Only the session-type hint: the real clipboard stays X11, the fake tools do the "typing".
export XDG_SESSION_TYPE=wayland

PATH="$fakes:$PATH" "$bin" --paste
grep -qx wtype "$log" || { echo "FAIL: wtype was not tried" >&2; exit 1; }
grep -q '^ydotool key' "$log" || { echo "FAIL: did not fall back to ydotool" >&2; cat "$log" >&2; exit 1; }

printf '#!/bin/sh\nexit 1\n' > "$fakes/ydotool"
if out=$(PATH="$fakes:$PATH" "$bin" --paste 2>&1); then echo "FAIL: expected an error" >&2; exit 1; fi
grep -q 'no paste tool worked' <<< "$out" || { echo "FAIL: unclear error: $out" >&2; exit 1; }
grep -q 'wtype' <<< "$out" && grep -q 'ydotool' <<< "$out" || { echo "FAIL: error does not list attempts: $out" >&2; exit 1; }
echo "OK (fallback)"
