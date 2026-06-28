#!/usr/bin/env bash
# Convert a square PNG (>=1024x1024) into a macOS .icns via an .iconset.
# Usage: make-icns.sh <input.png> <output.icns>
set -euo pipefail

png="${1:?usage: make-icns.sh <input.png> <output.icns>}"
out="${2:?usage: make-icns.sh <input.png> <output.icns>}"

work="$(mktemp -d)"
iconset="$work/icon.iconset"
mkdir -p "$iconset"

# Apple expects these size/scale pairs in the iconset.
for s in 16 32 128 256 512; do
  sips -z "$s" "$s"           "$png" --out "$iconset/icon_${s}x${s}.png"    >/dev/null
  sips -z "$((s * 2))" "$((s * 2))" "$png" --out "$iconset/icon_${s}x${s}@2x.png" >/dev/null
done

iconutil -c icns "$iconset" -o "$out"
rm -rf "$work"
echo "wrote $out"
