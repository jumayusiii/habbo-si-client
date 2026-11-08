#!/usr/bin/env bash
# Builds the stable client (dist/) or the beta channel (dist/beta/).
set -euo pipefail
cd "$(dirname "$0")"
CHANNEL="${1:-stable}"
CSC="/c/Windows/Microsoft.NET/Framework64/v4.0.30319/csc.exe"
[ -f "$CSC" ] || CSC="/c/Windows/Microsoft.NET/Framework/v4.0.30319/csc.exe"
W=$(cygpath -w "$(pwd)")

if [ "$CHANNEL" = "beta" ]; then
  OUT_UNIX="dist/beta"
  OUT_WIN="$W\\dist\\beta"
  NAME="Habbo.si Client Beta"
  DEFINE="-define:BETA"
elif [ "$CHANNEL" = "stable" ]; then
  OUT_UNIX="dist"
  OUT_WIN="$W\\dist"
  NAME="Habbo.si Client"
  DEFINE=""
else
  echo "usage: ./build.sh [stable|beta]" >&2
  exit 1
fi

mkdir -p "$OUT_UNIX"
rm -f "$OUT_UNIX/$NAME.exe"
MSYS2_ARG_CONV_EXCL='*' "$CSC" -nologo -target:winexe -platform:anycpu -optimize+ $DEFINE \
  "-win32icon:$W\\assets\\habbo.ico" \
  "-win32manifest:$W\\app.manifest" \
  "-resource:$W\\assets\\logo.png,logo.png" \
  "-resource:$W\\assets\\habbo-wordmark.png,habbo-wordmark.png" \
  "-out:$OUT_WIN\\$NAME.exe" \
  -r:System.dll -r:System.Core.dll -r:System.Drawing.dll -r:System.Windows.Forms.dll \
  "-r:$W\\lib\\webview2\\Microsoft.Web.WebView2.Core.dll" \
  "-r:$W\\lib\\webview2\\Microsoft.Web.WebView2.WinForms.dll" \
  "$W\\src\\*.cs"

cp lib/webview2/Microsoft.Web.WebView2.Core.dll \
   lib/webview2/Microsoft.Web.WebView2.WinForms.dll \
   lib/webview2/WebView2Loader.dll "$OUT_UNIX/"
echo "Built: $OUT_UNIX/$NAME.exe"
