# Habbo.si Client — Rust beta port

The native Rust beta client: a borderless Win32 window hosting three WebView2 surfaces
through `wry` — the custom app bar, the hotel and the themed footer — with the same hotel
URL, beta identity and release feed as the C# `Habbo.si Client Beta`. It does **not** use
.NET, WinForms, Tauri or a bundled browser.

The port is deliberately a separate application:

| | C# beta | Rust beta |
| --- | --- | --- |
| Executable | `Habbo.si Client Beta.exe` | `Habbo.si Client Beta (Rust).exe` |
| Profile | `%LOCALAPPDATA%\HabboSiClientBeta` | `%LOCALAPPDATA%\HabboSiClientBetaRust` |
| Engine | WebView2 via WinForms | WebView2 via `wry` |
| Update feed | beta pre-releases by default | beta pre-releases by default |

Keeping a separate profile means the port can be run beside the existing beta without
fighting over the WebView2 user-data folder or changing a signed-in session.

## Build

Requirements:

- Windows 10 1903+ or Windows 11 with the Edge WebView2 Runtime.
- Rust with the GNU Windows host (`x86_64-pc-windows-gnu`). The rustup GNU toolchain
  includes `rust-lld`, so no Visual Studio install is required.
- MinGW-w64 binutils with `dlltool.exe` and `windres.exe` on `PATH`; [w64devkit](https://github.com/skeeto/w64devkit)
  works and does not need to be installed system-wide. A full GCC is not required.
- Git Bash for the build script.

```bash
./rust/build.sh
```

Output:

```
dist/rust-beta/
├─ Habbo.si Client Beta (Rust).exe
└─ WebView2Loader.dll
```

The executable is a normal windowed program. Keep `WebView2Loader.dll` beside it; the
WebView2 runtime itself remains a shared Windows component.

## Feature parity

- **Borderless chrome** — the native frame is replaced by an HTML app bar with the HABBO
  wordmark, `CLIENT`/`BETA` pills, reload, settings, minimise, maximise and
  close. Dragging and all eight resize edges are wired to `tao` window APIs, double-click
  toggles maximise, and the footer is a second surface with room status, FPS and version.
- **Themes** — dark, light and navy palettes shared by the chrome, footer and every sheet,
  selectable in Settings and applied immediately.
- **English / German** — every chrome string, sheet heading and status line follows the
  language switch, matching the C# translations.
- **Discord Rich Presence** — named-pipe IPC with app-ID validation, large artwork,
  client version and the current room, title-cased the same way the C# client does it.
  Disabled automatically for preview runs.
- **Splash screen** — logo, gold rule and version fade over the hotel surface, then the
  hotel takes focus.
- **Cache clearing** — queued as a marker and applied on the next launch, because the
  running WebView2 owns its profile while the client is open.
- **Settings sheet** (`Ctrl+,`) — language, theme, renderer switches, beta/stable update
  channel, FPS readout, developer tools, Discord toggle and application ID, profile folder,
  cache clear and release notes. Stored as JSON in the Rust profile.
- **Update and What's new sheets** — the selected GitHub feed is checked once per launch;
  the release notes for the installed tag are rendered as plain text and shown once per
  profile.
- **Hotkeys** — `F5` reload, `F11` fullscreen, `Esc` leave fullscreen, `F12` devtools (when
  enabled), `Ctrl+,` settings.
- **Window state** — size and maximised state are remembered; preview runs never
  overwrite them.
- **CLI parity** — `--update-check <file>`, `--release-notes <file>`, `--presence-json <file>`,
  `--smoke <file>`, `--channel stable|beta`, `--lang en|de`, `--theme dark|light|navy`,
  `--data <dir>`, `--nosplash`, `--version`, plus the design/QA captures `--shot`,
  `--shot-live`, `--shot-settings`, `--shot-notes` and the DOM diagnostics `--probe <file>`.

## Safety

The executable embeds its icon, version resource and an `asInvoker` PerMonitorV2 manifest
through `rust/build.rs` (`windres`), builds with thin LTO and a normal Rust panic strategy,
and is scanned with Windows Defender before packaging. The binaries are **not code-signed**;
SmartScreen may still ask for confirmation on first run, which is expected for an unsigned
community build.

## Data and privacy

```
%LOCALAPPDATA%\HabboSiClientBetaRust\
├─ settings.json        language, theme, channel, Discord and window state
├─ WebView2\            hotel login, localStorage and asset cache
├─ WebView2-surface\    chrome, footer and splash surfaces
└─ ui\                  throw-away WebView2 profiles for the settings/notice sheets
```

The hotel and the chrome use separate WebView2 user-data folders because WebView2 refuses
two environments over one folder when their browser arguments differ — the hotel surface
needs the renderer switches.

The only network traffic is the hotel itself, the selected GitHub release feed, the notes
lookup after a build update, and Discord when presence is enabled. No account data is sent
to GitHub; failures are silent and the client keeps working offline. `--data <dir>` points
the whole profile somewhere else for testing.

## Verification

```bash
EXE="dist/rust-beta/Habbo.si Client Beta (Rust).exe"

"$EXE" --version
"$EXE" --presence-json .probe/rust-presence.json
"$EXE" --data .probe/rust-profile --update-check .probe/rust-update.txt
"$EXE" --data .probe/rust-profile --release-notes .probe/rust-notes.txt
"$EXE" --data .probe/rust-profile --smoke .probe/rust-smoke.txt
"$EXE" --data .probe/rust-profile --shot .probe/rust-shot.png --nosplash
"$EXE" --data .probe/rust-profile --shot-live .probe/rust-live.png
"$EXE" --data .probe/rust-profile --probe .probe/rust-probe.txt
```

`--smoke`, `--shot`, `--shot-live` and `--probe` skip the update lookup and Discord so a
check is deterministic, and they exit on their own. Unit tests cover the release-tag
ordering, settings validation, room detection and the Discord payload:

```bash
cd rust && cargo test --release
```
