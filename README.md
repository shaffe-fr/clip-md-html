# clip-md-html

[![CI](https://github.com/shaffe-fr/clip-md-html/actions/workflows/ci.yml/badge.svg)](https://github.com/shaffe-fr/clip-md-html/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/shaffe-fr/clip-md-html)](https://github.com/shaffe-fr/clip-md-html/releases/latest)

Tray app for Windows and macOS (CLI on Linux): press a global hotkey and the Markdown in the clipboard is converted to HTML and pasted as rich text. Built for copying raw Markdown from an AI assistant or editor into Thunderbird or any rich-text editor.

The HTML is semantic and unstyled, so the target applies its own defaults. Tables, strikethrough and task lists are supported. The original Markdown stays on the clipboard as the plain-text flavor.

## Install

Download the [latest release](https://github.com/shaffe-fr/clip-md-html/releases/latest): `clip-md-html-<version>-windows-x86_64.zip`, `clip-md-html-<version>-macos-universal.tar.gz`, `clip-md-html-<version>-linux-x86_64.tar.gz` or `clip-md-html-<version>-linux-aarch64.tar.gz`, checksums in `SHA256SUMS.txt`.

From source:

```sh
cargo install --path .
```

## Usage

Copy Markdown, focus the target (e.g. a Thunderbird compose window), press the hotkey.

| OS      | Default hotkey |
|---------|----------------|
| Windows | `Win+Alt+V`    |
| macOS   | `Cmd+Option+V` |

Tray menu: Convert now (convert without pasting), Start with Windows / Start at login, Quit. Only one instance runs at a time.

`clip-md-html --once` converts the clipboard and exits without pasting. Exit code 1 if the clipboard is empty or not text.

On Linux there is no tray icon, hotkey or config file; see [Linux](#linux).

## Configuration

Created on first run:

- Windows: `%APPDATA%\clip-md-html\config.toml`
- macOS: `~/Library/Application Support/clip-md-html/config.toml`

```toml
hotkey = "Win+Alt+V"   # macOS: "Cmd+Alt+V"
```

Modifiers: `Win`/`Cmd`, `Alt`/`Option`, `Ctrl`, `Shift`. Restart the app after editing. `Ctrl+Alt` (AltGr on AZERTY, rejected on Windows) and `Ctrl+Shift+V` (Thunderbird's paste as plain text) are rejected. A notification reports an invalid or already-taken hotkey. The log, `clip-md-html.log`, lives in the same directory.

## Linux

CLI only: no tray icon, no built-in hotkey, no autostart. You bind a command to a keyboard shortcut in your desktop environment.

```sh
clip-md-html --once    # convert the clipboard
clip-md-html --paste   # convert, then send Ctrl+V
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

Use `--once` instead of `--paste` if you prefer to paste yourself. Before Ctrl+V, every tool is told to release Super, Alt, Shift and Ctrl (`xdotool keyup` plus `--clearmodifiers`, `wtype -m`, `ydotool` key-up codes), but on Wayland the compositor still sees the physical keys as held, so prefer release-triggered bindings (`bindsym --release` in Sway, `bindr` in Hyprland). GNOME and KDE have no release-triggered custom shortcuts: if the paste lands as another shortcut, use `--once` and press Ctrl+V yourself.

Requirements: the Linux binaries are built on Ubuntu 22.04 and need glibc 2.35 or newer (Ubuntu 22.04, Debian 12, Fedora 36 and later). CI prints the highest `GLIBC_` symbol version the binary requires and fails above 2.35.

If the background holder fails to start (no clipboard access, for example), its stderr is included in the error message.

### What is tested

CI (Ubuntu 22.04, x86_64 and aarch64) builds, runs clippy and the unit tests, and runs end-to-end jobs against a real clipboard:

- X11 under Xvfb with `xclip`: `--once` returns promptly, `text/html` is offered with the expected tags and no `style=`, the plain text is still the Markdown, the holder outlives the command and exits when another client takes the clipboard.
- Wayland under headless sway with `wl-clipboard`: same assertions.
- `--paste` under Xvfb with `xdotool`: a Tk window receives Ctrl+V and reads the `text/html` flavor.
- `--paste` with a fake failing `wtype` ahead of a fake `ydotool` on `PATH`: it falls through, and reports every attempt when all fail.

Not tested: GNOME and KDE on Wayland (compositor support for the data-control protocol, shortcut binding), `wtype` and `ydotool` on a real compositor, pasting into Thunderbird or another real application, and the `linux-aarch64` build beyond compiling.

## Autostart

Opt-in from the tray menu (Windows and macOS only).

- Windows: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
- macOS: `~/Library/LaunchAgents/fr.shaffe.clip-md-html.plist`, effective at next login

## macOS Accessibility permission

Simulating `Cmd+V` requires System Settings > Privacy & Security > Accessibility. Enable `clip-md-html` (or the terminal launching it) and restart the app. Without it the clipboard is still converted and a notification asks you to paste manually. The app has a menu bar icon and no Dock icon.

## Antivirus

A global hotkey, simulated paste (`SendInput` / `CGEvent`) and optional autostart in an unsigned binary can trigger heuristic false positives. The app registers one hotkey with `RegisterHotKey`, uses no keyboard hook, never reads key state, writes only to its config directory and makes no network requests. The Windows exe embeds version metadata and an `asInvoker` manifest. During development you can exclude `target/`.

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
