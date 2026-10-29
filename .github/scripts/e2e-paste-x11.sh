#!/usr/bin/env bash
# End-to-end check of `clip-md-html --paste` under Xvfb: a Tk window receives the synthetic
# Ctrl+V from xdotool and records the text/html flavor of the clipboard it sees.
set -euo pipefail

bin=${BIN:-target/release/clip-md-html}
out=$(mktemp -d)

cat > "$out/target.py" <<'PY'
import sys, tkinter

out = sys.argv[1]
root = tkinter.Tk()
root.geometry("200x100+0+0")

def on_paste(_event):
    try:
        html = root.clipboard_get(type="text/html")
        # Tk hands non-text targets back as "0x3c 0x68 ..." hex bytes.
        if html.startswith("0x"):
            html = bytes(int(b, 16) for b in html.split()).decode("utf-8")
    except tkinter.TclError as e:
        html = "ERROR: %s" % e
    with open(out + "/pasted.html", "w") as f:
        f.write(html)
    root.after(50, root.destroy)
    return "break"

root.bind("<Control-v>", on_paste)
root.update()
root.focus_force()
root.update()
open(out + "/ready", "w").close()
root.mainloop()
PY

python3 "$out/target.py" "$out" &
target_pid=$!
for _ in $(seq 1 50); do [ -e "$out/ready" ] && break; sleep 0.1; done
[ -e "$out/ready" ] || { echo "FAIL: target window did not start" >&2; exit 1; }

printf '# Pasted\n\nSome **bold** text.' | xclip -selection clipboard -i
sleep 0.3
"$bin" --paste

for _ in $(seq 1 50); do wait_done=$(kill -0 "$target_pid" 2>/dev/null && echo no || echo yes); [ "$wait_done" = yes ] && break; sleep 0.1; done
[ -e "$out/pasted.html" ] || { echo "FAIL: the window never received Ctrl+V" >&2; exit 1; }
html=$(cat "$out/pasted.html")
echo "pasted: $html"
grep -qF '<h1>Pasted</h1>' <<< "$html" || { echo "FAIL: pasted content is not the converted HTML" >&2; exit 1; }
grep -qF '<strong>bold</strong>' <<< "$html" || { echo "FAIL: missing <strong>" >&2; exit 1; }
echo "OK (paste, x11)"
