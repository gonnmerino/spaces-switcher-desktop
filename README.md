# Spaces

Switch between virtual desktops on Windows with a key + mouse wheel.

Hold the key, scroll, and move between desktops. That's it.

![Spaces](assets/screenshot.png)

## Features

- **Any key as trigger** — Win, Alt, Shift, Caps Lock, or F1–F12
- **Animated transitions** — slide (macOS-style), scale, or native
- **Double-tap the trigger** to open Windows Task View
- **12 languages**
- Starts with Windows, sits in the tray
- Your settings are remembered

## How to use

1. Hold the trigger key (default **Win**)
2. Scroll the wheel to move between desktops
3. Double-tap the key to open Task View

Click the tray icon to open **Settings** — change the key, animation style, language, or turn animations and the Task View shortcut off.

## Download

Get the installer from [Releases](../../releases) and run it.

> Windows SmartScreen may warn since the app isn't signed. Click _More info → Run anyway_.

**Requires** Windows 10 / 11 (x64).

## Build from source

```bash
bun install
bun run tauri dev     # development
bun run tauri build   # installer → src-tauri/target/release/bundle/nsis/
```

Requires [Bun](https://bun.sh) and [Rust](https://rust-lang.org).

## License

[MIT](LICENSE) — © gonnmerino
