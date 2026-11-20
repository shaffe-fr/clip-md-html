# clip-md-html

[![CI](https://github.com/shaffe-fr/clip-md-html/actions/workflows/ci.yml/badge.svg)](https://github.com/shaffe-fr/clip-md-html/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/shaffe-fr/clip-md-html)](https://github.com/shaffe-fr/clip-md-html/releases/latest)

Markdown to rich text and back, from a hotkey.

| Hotkey (Windows / macOS)     | You have                                   | You get                                                   |
|------------------------------|--------------------------------------------|-----------------------------------------------------------|
| `Win+Alt+V` / `Cmd+Option+V` | Markdown on the clipboard, e.g. from an AI | It is pasted as rich text into your mail or editor        |
| `Win+Alt+C` / `Cmd+Option+M` | A mail or web page selected                | Its Markdown on the clipboard, images included            |

Ask an AI to draft a reply, copy its Markdown, press `Win+Alt+V` in Thunderbird: headings, lists, tables and bold arrive as formatting, not as `**stars**`. The other way round, select a mail thread, press `Win+Alt+C`, paste into Kiro or Jira: highlights, code and screenshots come along.

A small tray app for Windows and macOS. On Linux it is a CLI you bind to a shortcut.

## Quick start

1. Download the [latest release](https://github.com/shaffe-fr/clip-md-html/releases/latest) for your OS and run it. An icon appears in the tray (menu bar on macOS).
2. Copy some Markdown, click into a compose window, press `Win+Alt+V`.
3. Select part of a mail, press `Win+Alt+C`, paste anywhere.

On macOS, allow the app in System Settings > Privacy & Security > Accessibility so it can press `Cmd+V` / `Cmd+C` for you ([details](#macos)).

## Markdown to rich text

- The HTML is semantic and unstyled, so the mail client applies its own fonts.
- Tables, strikethrough and task lists are supported.
- One sentence per line stays one sentence per line: a single newline is a line break, as in GitHub comments.
- The Markdown stays on the clipboard as plain text, for apps that only take text.

## Rich text to Markdown

The hotkey copies the selection for you (it sends `Ctrl+C`), then replaces the clipboard with Markdown and shows a notification. If you already copied, press it anyway: the HTML already on the clipboard is used.

Besides the usual headings, emphasis, links, lists, tables, quotes and code, it understands what mail clients and Word produce:

| In the mail                                  | In the Markdown                                  |
|----------------------------------------------|--------------------------------------------------|
| Yellow highlight                             | `<mark>Reply in yellow</mark>`                   |
| Text in Courier New or Consolas              | `` `CDE.TXT` ``                                  |
| Word bullet and numbered paragraphs          | `-` and `1.` list items, nested by level         |
| Line break                                   | A plain newline (`<br>` inside table cells)      |
| Pasted screenshot                            | `![image](C:/Users/you/Pictures/clip-md-html/20261118-161016-1.png)` |
| Image from the web                           | `![alt](https://…)`                              |
| Image the app cannot read (`cid:`, `imap:`)  | `[image]`                                        |

Empty spacer paragraphs and non-breaking spaces are cleaned up. Only text is left on the clipboard.

### Images

Screenshots embedded in the selection are saved to `~/Pictures/clip-md-html` and linked by absolute path. This works with Thunderbird and Betterbird, which embed images in the copy, and with Outlook and Word, whose temporary copies are read right away.

- Kiro and other local agents open these paths directly. For a web tool such as Jira, drag the file in.
- Files are named after the conversion time (UTC). The same image twice in one selection is saved once.
- Files the app created are deleted after 24 hours, the next time an image is saved. Anything else in the folder is left alone.

## Tray menu

- **Convert Markdown to HTML now**: converts the clipboard without pasting.
- **Convert HTML to Markdown now**: converts what is already on the clipboard, without copying.
- **Start with Windows** / **Start at login**: opt-in autostart.
- **Quit**.

Only one instance runs at a time.

## Configuration

`~/.config/clip-md-html/config.toml` (or `$XDG_CONFIG_HOME/clip-md-html/config.toml`), created on first run:

```toml
hotkey = "Win+Alt+V"            # Markdown to rich text, then paste. macOS: "Cmd+Alt+V"
markdown_hotkey = "Win+Alt+C"   # copy the selection as Markdown. macOS: "Cmd+Alt+M"
```

Modifiers: `Win`/`Cmd`, `Alt`/`Option`, `Ctrl`, `Shift`. Restart the app after editing. A notification reports an invalid or already-taken hotkey.

<details>
<summary>Rejected hotkeys, upgrades, log location</summary>

- `Ctrl+Alt` is rejected on Windows (it is AltGr on AZERTY), and `Ctrl+Shift+V` everywhere (Thunderbird's paste as plain text).
- On Windows the file is `%USERPROFILE%\.config\clip-md-html\config.toml`. The Linux CLI reads nothing from it.
- Configs from earlier versions without `markdown_hotkey` use its default.
- Upgrading from before 0.3: if the new file does not exist, the old one (`%APPDATA%\clip-md-html\config.toml` on Windows, `~/Library/Application Support/clip-md-html/config.toml` on macOS) is copied over at startup and left in place. If both exist, the new one wins and the log says so.
- Log: `%LOCALAPPDATA%\clip-md-html\clip-md-html.log` on Windows, `~/Library/Logs/clip-md-html/clip-md-html.log` on macOS, stderr on Linux.

</details>

## Command line

```sh
clip-md-html --once          # Markdown on the clipboard to rich text, without pasting
clip-md-html --to-markdown   # HTML on the clipboard to Markdown, without copying
```

Both exit with code 1 when there is nothing to convert.

## Install

Releases ship `clip-md-html-<version>-windows-x86_64.zip`, `-macos-universal.tar.gz`, `-linux-x86_64.tar.gz` and `-linux-aarch64.tar.gz`, with checksums in `SHA256SUMS.txt`.

From source:

```sh
cargo install --path .
```

## macOS

- Pressing `Cmd+V` and `Cmd+C` for you requires System Settings > Privacy & Security > Accessibility. Enable `clip-md-html` (or the terminal launching it) and restart the app. Without it the clipboard is still converted and a notification asks you to paste yourself.
- The app has a menu bar icon and no Dock icon.
- Autostart uses `~/Library/LaunchAgents/fr.shaffe.clip-md-html.plist`, effective at next login.
- The binary is unsigned. After download, run `xattr -d com.apple.quarantine ./clip-md-html`.

## Linux

No tray icon and no built-in hotkey: bind a command to a shortcut in your desktop environment.

```sh
clip-md-html --once          # Markdown to rich text
clip-md-html --paste         # same, then press Ctrl+V
clip-md-html --to-markdown   # HTML to Markdown (copy the selection yourself first)
```

Use the absolute path if the binary is not in the desktop's `PATH`. `Super+Alt+V` is a good choice; avoid `Ctrl+Shift+V`.

- GNOME: Settings > Keyboard > Keyboard Shortcuts > Custom Shortcuts, command `clip-md-html --paste`.
- KDE Plasma: System Settings > Keyboard > Shortcuts > Add New > Command or Script.
- Sway: `bindsym --release $mod+Mod1+v exec clip-md-html --paste`
- Hyprland: `bindr = SUPER ALT, V, exec, clip-md-html --paste`
- X11 window managers: the WM's own bindings, `xbindkeys` or `sxhkd`.

Needs glibc 2.35 or newer (Ubuntu 22.04, Debian 12, Fedora 36 and later).

<details>
<summary>How the clipboard and the paste work on Linux</summary>

The Linux clipboard only lives as long as the program that owns it. Each command therefore starts a detached copy of itself that serves the result until another application copies something, then exits; the command itself returns right away. On Wayland it uses the `wlr-data-control` protocol (sway, Hyprland, other wlroots compositors, recent KDE Plasma). GNOME's Mutter does not implement it, so there the X11 clipboard through XWayland is used, which is untested.

`--paste` presses Ctrl+V with the first tool that works, picked from `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY` and `DISPLAY`:

| Session | Tools, in order      |
|---------|----------------------|
| Wayland | `wtype`, `ydotool`   |
| X11     | `xdotool`, `ydotool` |

- A tool that fails (`wtype` on GNOME Wayland, for example) falls through to the next. An error listing every attempt is shown only if all fail, and if none is installed the error says what to install. The clipboard is converted either way.
- `ydotool` needs its `ydotoold` daemon and access to `/dev/uinput`, which this program does not set up. The 1.x syntax is tried first, then 0.1.x.
- Every tool is told to release Super, Alt, Shift and Ctrl before Ctrl+V, but on Wayland the compositor still sees the physical keys as held. Prefer release-triggered bindings (`bindsym --release`, `bindr`). GNOME and KDE have none: if the paste lands as another shortcut, use `--once` and press Ctrl+V yourself.
- If the background holder fails to start, its stderr is included in the error.
- Running without arguments prints a usage hint and exits with status 2.

</details>

<details>
<summary>What is tested on Linux</summary>

CI (Ubuntu 22.04, x86_64 and aarch64) builds, runs clippy and the unit tests, then end-to-end jobs against a real clipboard:

- X11 under Xvfb with `xclip`, and Wayland under headless sway with `wl-clipboard`: `--once` returns promptly, `text/html` is offered with the expected tags and no `style=`, the plain text is still the Markdown, the holder outlives the command and exits when another client takes the clipboard.
- `--paste` under Xvfb with `xdotool`: a Tk window receives Ctrl+V and reads the `text/html` flavor.
- `--paste` with a failing fake `wtype` ahead of a fake `ydotool`: it falls through, and reports every attempt when all fail.

`--to-markdown` is covered by the converter's unit tests only. Not tested: GNOME and KDE on Wayland, `wtype` and `ydotool` on a real compositor, pasting into a real application, and the aarch64 build beyond compiling.

</details>

## Antivirus and signing

Global hotkeys, simulated keys (`SendInput` / `CGEvent`) and autostart in an unsigned binary can trigger heuristic false positives. The app registers its two hotkeys with `RegisterHotKey`, uses no keyboard hook and never reads key state. It writes only to its config, log and images folders and makes no network requests. The Windows exe embeds version metadata and an `asInvoker` manifest.

Windows binaries are unsigned for now, so SmartScreen may warn; signing through SignPath Foundation is planned. Autostart on Windows uses `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`. During development you can exclude `target/` from scanning.

## Development

```sh
cargo build --release
cargo clippy --all-targets -- -D warnings
cargo test
```

CI builds and tests on Windows, macOS and Linux, plus the Linux clipboard end-to-end jobs. Pushing a `v*` tag publishes a release.

Issues and pull requests are welcome; run clippy and the tests first. For vulnerabilities, see [SECURITY.md](SECURITY.md).

## License

MIT
