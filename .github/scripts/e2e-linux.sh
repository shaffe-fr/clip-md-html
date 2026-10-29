#!/usr/bin/env bash
# End-to-end check of `clip-md-html --once` against a real clipboard.
# Usage: e2e-linux.sh x11|wayland   (needs a running X11 or Wayland session)
set -euo pipefail

backend=${1:?usage: e2e-linux.sh x11|wayland}
bin=${BIN:-target/release/clip-md-html}

sample=$'# Title\n\nSome **bold** text and a [link](https://example.com).\n\n- one\n- two\n\n| a | b |\n|---|---|\n| 1 | 2 |'

case $backend in
  x11)
    set_clip() { printf '%s' "$1" | xclip -selection clipboard -i; }
    targets() { xclip -selection clipboard -t TARGETS -o; }
    get_html() { xclip -selection clipboard -t text/html -o; }
    get_plain() { xclip -selection clipboard -t UTF8_STRING -o; }
    ;;
  wayland)
    set_clip() { printf '%s' "$1" | wl-copy; }
    targets() { wl-paste -l; }
    get_html() { wl-paste -n -t text/html; }
    get_plain() { wl-paste -n -t text/plain; }
    ;;
  *) echo "unknown backend $backend" >&2; exit 2 ;;
esac

fail() { echo "FAIL: $*" >&2; exit 1; }
holder_running() { pgrep -f -- '--hold-clipboard' > /dev/null; }

set_clip "$sample"
sleep 0.3

start=$(date +%s%N)
"$bin" --once
elapsed_ms=$(( ($(date +%s%N) - start) / 1000000 ))
echo "--once returned after ${elapsed_ms} ms"
[ "$elapsed_ms" -lt 3000 ] || fail "--once took ${elapsed_ms} ms"

# The caller is gone; the detached holder must still own the clipboard.
sleep 1
holder_running || fail "holder process is not running"
echo "targets:"; targets
targets | grep -qx 'text/html' || fail "text/html missing from targets"

html=$(get_html)
echo "html: $html"
for tag in '<h1>' '<strong>' '<a href="https://example.com">' '<li>' '<table>' '<td>'; do
  grep -qF -- "$tag" <<< "$html" || fail "missing $tag"
done
if grep -qF 'style=' <<< "$html"; then fail "unexpected style= attribute"; fi

plain=$(get_plain)
[ "$plain" = "$sample" ] || fail "plain text flavor differs from the Markdown: $plain"

# Another client takes the clipboard: the holder must exit by itself.
set_clip "something else"
for _ in $(seq 1 50); do holder_running || break; sleep 0.1; done
if holder_running; then fail "holder still running after ownership change"; fi

echo "OK ($backend)"
