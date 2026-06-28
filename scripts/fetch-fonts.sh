#!/usr/bin/env bash
# Fetch LINE Seed JP (SIL OFL) from the official line/seed release into vendor/fonts/.
# Required before building the GUI (cargo build --features gui), like setup-syphon.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/vendor/fonts"
URL="https://github.com/line/seed/releases/download/v20251119/seed-v20251119.zip"

mkdir -p "$DEST"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading LINE Seed JP from $URL"
curl -fsSL "$URL" -o "$tmp/seed.zip"
unzip -q "$tmp/seed.zip" -d "$tmp/x"

# Some releases nest per-family zips; expand any inner zips too.
find "$tmp/x" -name '*.zip' -print0 | while IFS= read -r -d '' z; do
  unzip -qo "$z" -d "${z}.d" || true
done

# Pick a Japanese Regular weight (ttf or otf).
font="$(find "$tmp/x" -type f \
  \( -iname '*JP*Rg*.ttf' -o -iname '*JP*Regular*.ttf' \
     -o -iname '*JP*Rg*.otf' -o -iname '*JP*Regular*.otf' \) -print -quit)"
if [ -z "$font" ]; then
  echo "ERROR: no LINE Seed JP Regular font found in the archive." >&2
  echo "Inspect the archive layout and adjust the find globs in this script." >&2
  exit 1
fi
cp "$font" "$DEST/LINESeedJP-Regular.ttf"

license="$(find "$tmp/x" -type f \( -iname 'OFL*' -o -iname 'LICENSE*' \) -print -quit)"
[ -n "$license" ] && cp "$license" "$DEST/LICENSE" || echo "(note: no LICENSE file found in archive)"

echo "OK: $DEST/LINESeedJP-Regular.ttf ($(wc -c < "$DEST/LINESeedJP-Regular.ttf") bytes)"
