#!/usr/bin/env bash
# Builds, tests, scans and packages the Rust beta port into dist/rust-beta/.
set -euo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

# The GNU toolchain ships rust-lld. It links the app without needing a full
# MinGW C runtime. The raw-dylib imports still require dlltool from MinGW
# binutils (w64devkit or WinLibs), and the icon/version resource needs windres.
if ! command -v dlltool >/dev/null 2>&1; then
  echo "error: dlltool.exe not found on PATH; install MinGW-w64 binutils or w64devkit." >&2
  exit 1
fi
if ! command -v windres >/dev/null 2>&1; then
  echo "error: windres.exe not found on PATH; the icon and version resource need it." >&2
  exit 1
fi

if [ -z "${CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER:-}" ]; then
  export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=rust-lld
fi

# Force a fresh build timestamp for the header pill on every packaged build.
touch build.rs

echo "Checking formatting…"
cargo fmt --check

echo "Building release binary…"
cargo build --release

echo "Running unit tests…"
cargo test --release

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
if [ -z "$VERSION" ]; then
  echo "error: could not read the version from Cargo.toml." >&2
  exit 1
fi

OUT="$ROOT/dist/rust-beta"
NAME="Habbo.si Client Beta (Rust)"
EXE="$OUT/$NAME.exe"
ZIP="$OUT/$NAME v$VERSION.zip"
mkdir -p "$OUT"
cp "target/release/habbo-si-client-rust.exe" "$EXE"
cp "$ROOT/lib/webview2/WebView2Loader.dll" "$OUT/"

echo "Packaging $NAME v$VERSION…"
rm -f "$ZIP"
powershell.exe -NoProfile -Command \
  "Compress-Archive -Path '$(cygpath -w "$EXE")','$(cygpath -w "$OUT/WebView2Loader.dll")' -DestinationPath '$(cygpath -w "$ZIP")' -Force"

# Newest Defender platform first, then the legacy per-machine path.
MP=""
for candidate in "/c/ProgramData/Microsoft/Windows Defender/Platform"/*/X86/MpCmdRun.exe \
                 "/c/Program Files/Windows Defender/MpCmdRun.exe"; do
  if [ -f "$candidate" ]; then MP="$candidate"; fi
done
if [ -z "$MP" ]; then
  echo "error: MpCmdRun.exe not found; cannot verify the build with Windows Defender." >&2
  exit 1
fi

scan() {
  local target="$1"
  local status=0
  echo "Scanning $(basename "$target") with Windows Defender…"
  "$MP" -Scan -ScanType 3 -File "$(cygpath -w "$target")" -DisableRemediation || status=$?
  if [ "$status" -ne 0 ]; then
    echo "error: Windows Defender exited with $status for $target; refusing to ship." >&2
    exit 1
  fi
}

scan "$EXE"
scan "$ZIP"

# Only reached after both scans are clean: refresh the local beta folder too.
DESKTOP="$(powershell.exe -NoProfile -Command "[Environment]::GetFolderPath('Desktop')" | tr -d '\r\n')"
DESKTOP="$DESKTOP/Habbo.si Client Beta"
if [ -d "$DESKTOP" ]; then
  cp -f "$EXE" "$OUT/WebView2Loader.dll" "$ZIP" "$DESKTOP/"
  echo "Desktop:  $DESKTOP"
fi

echo
echo "Built:    $EXE"
echo "Package:  $ZIP"
echo "Defender: no threats"
echo "SHA256:"
sha256sum "$EXE" "$ZIP"
