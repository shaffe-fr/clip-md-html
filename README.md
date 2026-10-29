# clip-md-html

[![CI](https://github.com/shaffe-fr/clip-md-html/actions/workflows/ci.yml/badge.svg)](https://github.com/shaffe-fr/clip-md-html/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/shaffe-fr/clip-md-html)](https://github.com/shaffe-fr/clip-md-html/releases/latest)

Tray app for Windows and macOS: press a global hotkey and the Markdown in the clipboard is converted to HTML and pasted as rich text. Built for copying raw Markdown from an AI assistant or editor into Thunderbird or any rich-text editor.

The HTML is semantic and unstyled, so the target applies its own defaults. Tables, strikethrough and task lists are supported. The original Markdown stays on the clipboard as the plain-text flavor.

## Install

Download the [latest release](https://github.com/shaffe-fr/clip-md-html/releases/latest): `clip-md-html-<version>-windows-x86_64.zip` or `clip-md-html-<version>-macos-universal.tar.gz`, checksums in `SHA256SUMS.txt`.

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

## Configuration

Created on first run:

- Windows: `%APPDATA%\clip-md-html\config.toml`
- macOS: `~/Library/Application Support/clip-md-html/config.toml`

```toml
hotkey = "Win+Alt+V"   # macOS: "Cmd+Alt+V"
```

Modifiers: `Win`/`Cmd`, `Alt`/`Option`, `Ctrl`, `Shift`. Restart the app after editing. `Ctrl+Alt` (AltGr on AZERTY, rejected on Windows) and `Ctrl+Shift+V` (Thunderbird's paste as plain text) are rejected. A notification reports an invalid or already-taken hotkey. The log, `clip-md-html.log`, lives in the same directory.

## Autostart

Opt-in from the tray menu.

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

CI builds and tests on Windows and macOS. Pushing a `v*` tag publishes a release.

## Contributing

Issues and pull requests are welcome; run clippy and the tests first. For vulnerabilities, see [SECURITY.md](SECURITY.md).

## License

MIT
