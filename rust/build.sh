#!/usr/bin/env bash
# Builds the Rust beta port into dist/rust-beta/.
set -euo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

# The GNU toolchain ships rust-lld. It links the app without needing a full
# MinGW C runtime. The windows crate's raw-dylib imports still require dlltool
# from MinGW binutils (w64devkit or WinLibs) on PATH.
if ! command -v dlltool >/dev/null 2>&1; then
  echo "error: dlltool.exe not found on PATH; install MinGW-w64 binutils or w64devkit." >&2
  exit 1
fi

if [ -z "${CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER:-}" ]; then
  export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=rust-lld
fi

cargo build --release

OUT="$ROOT/dist/rust-beta"
mkdir -p "$OUT"
cp "target/release/habbo-si-client-rust.exe" "$OUT/Habbo.si Client Beta (Rust).exe"
cp "$ROOT/lib/webview2/WebView2Loader.dll" "$OUT/"
echo "Built: $OUT/Habbo.si Client Beta (Rust).exe"
