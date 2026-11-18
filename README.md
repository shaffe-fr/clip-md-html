# clip-md-html

[![CI](https://github.com/shaffe-fr/clip-md-html/actions/workflows/ci.yml/badge.svg)](https://github.com/shaffe-fr/clip-md-html/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/shaffe-fr/clip-md-html)](https://github.com/shaffe-fr/clip-md-html/releases/latest)

Tray app for Windows and macOS (CLI on Linux): press a global hotkey and the Markdown in the clipboard is converted to HTML and pasted as rich text. Built for copying raw Markdown from an AI assistant or editor into Thunderbird or any rich-text editor.

The HTML is semantic and unstyled, so the target applies its own defaults. Tables, strikethrough and task lists are supported. A single newline is kept as a line break, as in GitHub comments, so one sentence per line stays one sentence per line. The original Markdown stays on the clipboard as the plain-text flavor.

It also works the other way: select text in a mail or a web page, press a second hotkey, and the clipboard holds the selection as Markdown. See [HTML to Markdown](#html-to-markdown).

## Install

Download the [latest release](https://github.com/shaffe-fr/clip-md-html/releases/latest): `clip-md-html-<version>-windows-x86_64.zip`, `clip-md-html-<version>-macos-universal.tar.gz`, `clip-md-html-<version>-linux-x86_64.tar.gz` or `clip-md-html-<version>-linux-aarch64.tar.gz`, checksums in `SHA256SUMS.txt`.

From source:

```sh
cargo install --path .
```

## Usage

Copy Markdown, focus the target (e.g. a Thunderbird compose window), press the hotkey.

| OS      | Markdown to HTML (paste) | HTML to Markdown (copy) |
|---------|--------------------------|-------------------------|
| Windows | `Win+Alt+V`              | `Win+Alt+C`             |
| macOS   | `Cmd+Option+V`           | `Cmd+Option+M`          |

Tray menu: Convert Markdown to HTML now (convert without pasting), Convert HTML to Markdown now (convert what is already on the clipboard), Start with Windows / Start at login, Quit. Only one instance runs at a time.

`clip-md-html --once` converts the clipboard Markdown and exits without pasting. Exit code 1 if the clipboard is empty or not text.

`clip-md-html --to-markdown` replaces the HTML on the clipboard with its Markdown and exits. Exit code 1 if the clipboard has no HTML.

On Linux there is no tray icon, hotkey or required config file; see [Linux](#linux).

## HTML to Markdown

Select text in a mail client, a browser or Word, then press `Win+Alt+C` (`Cmd+Option+M` on macOS). The app sends Ctrl+C (Cmd+C) to the focused window, waits for the copy, and replaces the clipboard with the Markdown of its HTML flavor. A notification confirms it. Paste the result anywhere; pressing the other hotkey turns it back into rich text.

If you copied already, press the hotkey anyway: when the copy changes nothing, the HTML already on the clipboard is converted (after a one second wait). Plain text with no HTML flavor is reported, not converted.

Only text remains on the clipboard afterwards (no HTML flavor). Besides the usual tags (headings, emphasis, links, lists, tables, quotes, code), the conversion handles what Outlook and Word emit:

- Highlighted text (`mso-highlight`, a `background` colour, `<mark>`) is kept as `<mark>...</mark>`; split runs are merged.
- Monospace runs (Courier New, Consolas, ...) become inline code.
- Word list paragraphs become Markdown list items, nested by level.
- Empty spacer paragraphs are dropped and non-breaking spaces become plain spaces.
- Images embedded in the copy are saved to `~/Pictures/clip-md-html` and linked by absolute path, e.g. `![image](C:/Users/you/Pictures/clip-md-html/20261118-161016-1.png)`. That covers Thunderbird and Betterbird (`data:` images) and Outlook/Word (`file:` temp copies, read right after the copy). An agent such as Kiro can open these paths; for a web tool like Jira, drag the file in. Names are the conversion time in UTC plus a number; the same image twice in one copy is saved once. Files named this way are deleted once they are more than 24 hours old, the next time an image is saved; other files in the folder are left alone.
- Images with a web address are kept as `![alt](url)`. References to mail parts that cannot be read (`cid:`, `imap:`, `mailbox:`, ...) become `[image]`.

Linux has no hotkey and does not send Ctrl+C: copy first, then run `clip-md-html --to-markdown`.

## Configuration

Created on first run, on every OS, at `$XDG_CONFIG_HOME/clip-md-html/config.toml` when `XDG_CONFIG_HOME` is set to an absolute path, otherwise `~/.config/clip-md-html/config.toml`:
- Windows: `%USERPROFILE%\.config\clip-md-html\config.toml`
- macOS: `~/.config/clip-md-html/config.toml`
- Linux: same path; the CLI reads nothing from it and needs no config.

Upgrading: if the new file does not exist, the config from the old location (`%APPDATA%\clip-md-html\config.toml` on Windows, `~/Library/Application Support/clip-md-html/config.toml` on macOS) is copied there at startup. The old file is left in place and no longer used; if both exist, the new one wins and the log notes it.

```toml
hotkey = "Win+Alt+V"            # Markdown to HTML and paste. macOS: "Cmd+Alt+V"
markdown_hotkey = "Win+Alt+C"   # copy the selection as Markdown. macOS: "Cmd+Alt+M"
```

`markdown_hotkey` is missing from configs written by earlier versions; the default applies until you add it.

Modifiers: `Win`/`Cmd`, `Alt`/`Option`, `Ctrl`, `Shift`. Restart the app after editing. `Ctrl+Alt` (AltGr on AZERTY, rejected on Windows) and `Ctrl+Shift+V` (Thunderbird's paste as plain text) are rejected. A notification reports an invalid or already-taken hotkey.

The log, `clip-md-html.log`, is state rather than config:
- Windows: `%LOCALAPPDATA%\clip-md-html\clip-md-html.log`
- macOS: `~/Library/Logs/clip-md-html/clip-md-html.log`
- Linux: no log file; errors go to stderr.

## Linux

CLI only: no tray icon, no built-in hotkey, no autostart. You bind a command to a keyboard shortcut in your desktop environment.

```sh
clip-md-html --once          # convert the clipboard Markdown to HTML
clip-md-html --paste         # same, then send Ctrl+V
clip-md-html --to-markdown   # replace the HTML on the clipboard with its Markdown
```

Running it without arguments prints a usage hint and exits with status 2.

The Linux clipboard only lives as long as the program that owns it. `--once` therefore starts a detached background copy of itself that serves the HTML (`text/html`) and the Markdown (plain text) until another application copies something, then exits. The command itself returns right away. On Wayland it uses the `wlr-data-control` protocol (sway, Hyprland, other wlroots compositors; recent KDE Plasma should work too). GNOME's Mutter does not implement it, so there the X11 clipboard through XWayland is used as a fallback, which is untested (see below).

`--paste` then simulates Ctrl+V with the first available tool, chosen from `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY` and `DISPLAY`:

| Session | Tools, in order       |
|---------|-----------------------|
| Wayland | `wtype`, `ydotool`    |
| X11     | `xdotool`, `ydotool`  |

If a tool is installed but fails (for example `wtype` on GNOME Wayland, which lacks the virtual-keyboard protocol), the next one is tried; an error listing every attempt is printed only when all of them fail. In both cases the clipboard is still converted and you paste manually. If none is installed, the error says which to install. `ydotool` needs its `ydotoold` daemon and access to `/dev/uinput`, which this program does not set up. `ydotool` is called with the 1.x syntax (evdev keycodes) first, then with the 0.1.x syntax (`key ctrl+v`) if that fails. Prefer `wtype` on Wayland (wlroots compositors, KDE) and `xdotool` on X11. `wtype` does not work on GNOME Wayland.

Bind a shortcut (use the absolute path if the binary is not in the desktop's `PATH`). Avoid `Ctrl+Shift+V` (Thunderbird's paste as plain text); `Super+Alt+V` is a good default.

- GNOME: Settings > Keyboard > Keyboard Shortcuts > Custom Shortcuts > add, command `clip-md-html --paste`.
- KDE Plasma: System Settings > Keyboard > Shortcuts > Add New > Command or Script, command `clip-md-html --paste`.
- Sway (`~/.config/sway/config`): `bindsym --release $mod+Mod1+v exec clip-md-html --paste`
- Hyprland (`~/.config/hypr/hyprland.conf`): `bindr = SUPER ALT, V, exec, clip-md-html --paste`
- X11 window managers: bind the same command in the WM, or with `xbindkeys` / `sxhkd`.

`--to-markdown` serves plain text the same way (a detached holder until another application copies something). It never sends a key: copy the selection yourself first.

Use `--once` instead of `--paste` if you prefer to paste yourself. Before Ctrl+V, every tool is told to release Super, Alt, Shift and Ctrl (`xdotool keyup` plus `--clearmodifiers`, `wtype -m`, `ydotool` key-up codes), but on Wayland the compositor still sees the physical keys as held, so prefer release-triggered bindings (`bindsym --release` in Sway, `bindr` in Hyprland). GNOME and KDE have no release-triggered custom shortcuts: if the paste lands as another shortcut, use `--once` and press Ctrl+V yourself.

Requirements: the Linux binaries are built on Ubuntu 22.04 and need glibc 2.35 or newer (Ubuntu 22.04, Debian 12, Fedora 36 and later). CI prints the highest `GLIBC_` symbol version the binary requires and fails above 2.35.

If the background holder fails to start (no clipboard access, for example), its stderr is included in the error message.

### What is tested

CI (Ubuntu 22.04, x86_64 and aarch64) builds, runs clippy and the unit tests, and runs end-to-end jobs against a real clipboard:

- X11 under Xvfb with `xclip`: `--once` returns promptly, `text/html` is offered with the expected tags and no `style=`, the plain text is still the Markdown, the holder outlives the command and exits when another client takes the clipboard.
- Wayland under headless sway with `wl-clipboard`: same assertions.
- `--paste` under Xvfb with `xdotool`: a Tk window receives Ctrl+V and reads the `text/html` flavor.
- `--paste` with a fake failing `wtype` ahead of a fake `ydotool` on `PATH`: it falls through, and reports every attempt when all fail.

`--to-markdown` is covered by the unit tests of the converter and compiles on Linux, but has no end-to-end job yet.

Not tested: GNOME and KDE on Wayland (compositor support for the data-control protocol, shortcut binding), `wtype` and `ydotool` on a real compositor, pasting into Thunderbird or another real application, and the `linux-aarch64` build beyond compiling.

## Autostart

Opt-in from the tray menu (Windows and macOS only).

- Windows: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
- macOS: `~/Library/LaunchAgents/fr.shaffe.clip-md-html.plist`, effective at next login

## macOS Accessibility permission

Simulating `Cmd+V` and `Cmd+C` requires System Settings > Privacy & Security > Accessibility. Enable `clip-md-html` (or the terminal launching it) and restart the app. Without it the clipboard is still converted and a notification asks you to paste manually. The app has a menu bar icon and no Dock icon.

## Antivirus

A global hotkey, simulated paste and copy (`SendInput` / `CGEvent`) and optional autostart in an unsigned binary can trigger heuristic false positives. The app registers two hotkeys with `RegisterHotKey`, uses no keyboard hook, never reads key state, writes only to its config and log directories and makes no network requests. The Windows exe embeds version metadata and an `asInvoker` manifest. During development you can exclude `target/`.

## Code signing

- Windows: signing through SignPath Foundation will be requested after the first release. Until then binaries are unsigned and SmartScreen may warn.
- macOS: unsigned. After download, run `xattr -d com.apple.quarantine ./clip-md-html`.

## Development

```sh
cargo build --release
cargo clippy --all-targets -- -D warnings
cargo test
```

CI builds and tests on Windows, macOS and Linux, plus Linux clipboard end-to-end jobs. Pushing a `v*` tag publishes a release.

## Contributing

Issues and pull requests are welcome; run clippy and the tests first. For vulnerabilities, see [SECURITY.md](SECURITY.md).

## License

MIT
