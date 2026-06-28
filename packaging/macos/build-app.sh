#!/usr/bin/env bash
# Assemble Bucatini.app from already-built universal binaries + Syphon.framework.
#
# Usage:
#   build-app.sh <version> <gui-bin> <cli-bin> <framework-dir> <icns> <out-dir>
#
# Produces "<out-dir>/Bucatini.app". The GUI is the bundle executable; the CLI
# rides along in Contents/MacOS so terminal users can run it from the bundle.
# Both binaries get an rpath into Contents/Frameworks so they find Syphon at
# runtime. libndi is NOT bundled — users install the NDI runtime separately.
set -euo pipefail

version="${1:?version}"
gui_bin="${2:?gui binary path}"
cli_bin="${3:?cli binary path}"
framework="${4:?Syphon.framework path}"
icns="${5:?icns path}"
out_dir="${6:?output dir}"

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
app="$out_dir/Bucatini.app"

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Frameworks" "$app/Contents/Resources"

# Info.plist with the version substituted in.
sed "s/__VERSION__/$version/g" "$here/Info.plist" > "$app/Contents/Info.plist"

cp "$gui_bin" "$app/Contents/MacOS/bucatini-gui"
cp "$cli_bin" "$app/Contents/MacOS/bucatini"
chmod +x "$app/Contents/MacOS/bucatini-gui" "$app/Contents/MacOS/bucatini"
cp -R "$framework" "$app/Contents/Frameworks/"
cp "$icns" "$app/Contents/Resources/bucatini.icns"

# Point both binaries at the bundled framework. The release binaries already
# carry an @loader_path rpath (for the flat tarball layout); add the .app one.
for bin in bucatini-gui bucatini; do
  install_name_tool -add_rpath @executable_path/../Frameworks "$app/Contents/MacOS/$bin" 2>/dev/null || true
done

echo "built $app"
