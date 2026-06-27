#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# 1. Fetch the Syphon framework source.
git submodule update --init --recursive

# 2. Build Syphon.framework (Release) and copy it into vendor/.
cd vendor/syphon-src
xcodebuild -project Syphon.xcodeproj -scheme Syphon -configuration Release \
  -derivedDataPath build SYMROOT="$PWD/build"
cd "$ROOT"
rm -rf vendor/Syphon.framework
cp -R vendor/syphon-src/build/Release/Syphon.framework vendor/Syphon.framework
echo "Syphon.framework installed at vendor/Syphon.framework"
