# Display Switcher

Fast display group switching for Windows with global hotkeys, gamepad chords, system tray, and optional post-actions (Steam Big Picture, custom programs).

## Features

- **Display groups** — assign monitors via dropdown, save atomic layout snapshots
- **Triggers** — global hotkeys, system tray, gamepad chords (XInput), config UI
- **Post-actions** — exit/launch Steam Big Picture, run programs or shell commands
- **Telemetry** — on-device switch timing (median/p95 display apply latency)
- **Themes** — light, dark, or system

## Requirements

- Windows 10/11
- [Bun](https://bun.sh/) (package manager + script runner)
- [Rust](https://rustup.rs/)
- Visual Studio 2022 Build Tools (C++ workload) for native compilation

## Development

```bash
cd display-switcher
bun install
bun run tauri dev
```

## Build installer

```bash
bun run tauri build
```

## Regenerate app icons

Source: `icon-concepts/concept-4-single-refresh.svg`

```bash
bun run icon
```

Output: `src-tauri/target/release/bundle/nsis/`

## Default test setup

After onboarding, two groups are created:

| Group | Hotkey | Post-action |
|-------|--------|-------------|
| Desktop Mode | Ctrl+Alt+D | Exit Steam Big Picture |
| TV Mode | Ctrl+Alt+T | Launch Steam Big Picture |

Assign your monitors, arrange layouts in Windows Settings, then click **Save layout** for each group.

## Config location

`%APPDATA%/display-switcher/`

- `config.json` — groups, hotkeys, settings
- `profiles/` — serialized display paths
- `telemetry.jsonl` — switch timing records

## License

MIT