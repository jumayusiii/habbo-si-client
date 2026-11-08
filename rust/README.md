# Habbo.si Client — Rust beta port

This is the first native Rust milestone of the beta client: a small Win32 window hosting
WebView2 through `wry`, with the same hotel URL, beta identity and release feed as the C#
`Habbo.si Client Beta`. It does **not** use .NET, WinForms, Tauri or a bundled browser.

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
- MinGW-w64 binutils with `dlltool.exe` on `PATH`; [w64devkit](https://github.com/skeeto/w64devkit)
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

## What works in this milestone

- Beta title and version (`1.2.0-beta.2`), separate profile, dark native window and app icon.
- Persistent WebView2 profile with GPU rasterisation, zero-copy, unlocked frame pacing and
  background-throttling switches matching the C# beta defaults.
- Settings sheet (`Ctrl+,`) for the beta/stable update channel, renderer switches, FPS title
  readout and developer tools. Settings are stored as JSON in the Rust profile.
- Startup update check against the selected GitHub feed; a themed notice opens the release
  page when a newer build exists. Nothing is downloaded or installed.
- What's new sheet for the installed beta tag, shown once per Rust profile. Reopen it from
  **Settings → Updates → Release notes**. The GitHub release body is converted to plain text
  and rendered in a scrollable themed webview.
- Hotkeys: `F5` reload, `F11` fullscreen, `Esc` leave fullscreen, `F12` devtools (when
  enabled), `Ctrl` `+`/`-`/`0` zoom and `Ctrl+,` settings.
- Window size/maximised state and zoom are remembered.
- CLI parity for the release checks:
  `--update-check <file>`, `--channel stable|beta`, `--release-notes <file>`, `--data <dir>`
  and `--smoke <file>`.

## Not ported yet

This is intentionally a starting point, not a claim of full parity. The custom borderless
chrome and app bar, themed footer, Discord Rich Presence, English/German switching, splash,
cache clearing, deterministic `--shot`/`--probe` tools and the release packaging still live
only in the C# beta. The current Rust window uses the native Windows title bar. Those pieces
can move over incrementally without changing the profile or update model.

## Data and privacy

```
%LOCALAPPDATA%\HabboSiClientBetaRust\
├─ settings.json      channel, renderer switches, zoom and window state
├─ WebView2\          hotel login, localStorage and asset cache
└─ ui\                throw-away WebView2 profiles for the settings/notice sheets
```

The only network traffic is the hotel itself, the selected GitHub release feed and the
notes lookup after a build update. No account data is sent to GitHub; failures are silent
and the client keeps working offline. `--data <dir>` points the whole profile somewhere
else for testing.

## Verification

```bash
# feed lookup (writes the same fields as the C# tool)
"dist/rust-beta/Habbo.si Client Beta (Rust).exe" --data .probe/rust-profile --update-check .probe/rust-update.txt
cat .probe/rust-update.txt

# notes for the installed tag
"dist/rust-beta/Habbo.si Client Beta (Rust).exe" --release-notes .probe/rust-notes.txt

# headless-ish startup check: opens the window, then exits after the first paint
"dist/rust-beta/Habbo.si Client Beta (Rust).exe" --data .probe/rust-profile --smoke .probe/rust-smoke.txt
cat .probe/rust-smoke.txt
```

`--smoke` skips the update lookup so the check is deterministic.
