#!/usr/bin/env bash
# Package Bucatini.app into a distributable .dmg using only hdiutil (no extra
# deps). The volume contains the app + an /Applications symlink so users can
# drag-install, plus README files and an "unsigned app" first-run note.
#
# Usage: build-dmg.sh <version> <app-path> <out-dir> [extra-file ...]
set -euo pipefail

version="${1:?version}"
app="${2:?Bucatini.app path}"
out_dir="${3:?output dir}"
shift 3
extras=("$@")

vol="Bucatini $version"
dmg="$out_dir/Bucatini-$version-macos-universal.dmg"
stage="$(mktemp -d)/dmg"
mkdir -p "$stage"

cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
for f in "${extras[@]}"; do
  [ -e "$f" ] && cp "$f" "$stage/"
done

# Unsigned-build first-run instructions (we don't ship an Apple Developer ID).
cat > "$stage/READ ME FIRST.txt" <<'EOF'
Bucatini is distributed UNSIGNED (no Apple Developer ID).

First launch:
  1. Drag Bucatini.app into the Applications folder.
  2. Right-click Bucatini.app -> Open -> Open (only needed the first time).

If macOS still blocks it ("damaged / cannot be opened"), clear the quarantine
flag in Terminal:
  xattr -dr com.apple.quarantine /Applications/Bucatini.app

Requirements:
  - The NDI runtime must be installed (e.g. NDI Tools, or `brew install libndi`).
EOF

rm -f "$dmg"
hdiutil create -volname "$vol" -srcfolder "$stage" -ov -format UDZO "$dmg"
rm -rf "$(dirname "$stage")"
echo "built $dmg"
