# Changelog

## 1.2.0-beta.4 — 2026-11-09 (hotfix)

- Dark theme rebuilt around `#1E1E1E`, with neutral grey surfaces, borders and muted text.
- Splash screen now appears with the first window paint instead of after WebView2 initialises.
- Live theme preview in Settings: clicking a palette restyles the client immediately, and Cancel, Esc or closing the sheet restores the saved one.
- Header restored to the CLIENT pill, with version and build date/time in its hover text; the BETA chip is gone.
- Settings sheet is borderless, owned by the client, centred, keeps out of the taskbar and has a draggable header.
- `rust/build.sh` now runs `cargo fmt --check` and refreshes the Desktop beta folder only after the Defender scans come back clean.

## 1.2.0-beta.3 — 2026-11-09

- C# beta bumped to 1.2.0-beta.3 and published alongside the Rust build so the shared beta feed offers an installable asset for both clients.
- Rust port reached feature parity with the C# client: borderless chrome, themes, language switch, Discord presence, splash, settings, update and notes sheets, hotkeys and window state.
