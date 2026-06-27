#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# 1. Fetch the Syphon framework source.
git submodule update --init --recursive

# 2. Build Syphon.framework (Release) and copy it into vendor/.
cd vendor/syphon-src
# Prerequisites: Full Xcode is required (not just Command Line Tools).
# If xcrun cannot find the SDK, run: sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
# Metal Toolchain must be installed. If build fails with "cannot execute tool 'metal'", run once:
# xcodebuild -downloadComponent MetalToolchain
xcodebuild -project Syphon.xcodeproj -scheme Syphon -configuration Release \
  -derivedDataPath build SYMROOT="$PWD/build"
cd "$ROOT"
rm -rf vendor/Syphon.framework
cp -R vendor/syphon-src/build/Release/Syphon.framework vendor/Syphon.framework
echo "Syphon.framework installed at vendor/Syphon.framework"
