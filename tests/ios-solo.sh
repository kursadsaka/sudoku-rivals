#!/bin/sh
# Builds the Rust core + iOS app, then runs the solo logic tests (real core via
# UniFFI) and XCUITests on a simulator. macOS + Xcode only.
# Usage: sh tests/ios-solo.sh [xcodebuild -only-testing:... filters]
set -eu
cd "$(dirname "$0")/.."
sh scripts/ios-build.sh
DEST="${IOS_SIM_DESTINATION:-platform=iOS Simulator,name=iPhone 18 Pro}"
RESULT="target/ios-solo.xcresult"
rm -rf "$RESULT"
cd apps/ios
xcodebuild -project SudokuRivals.xcodeproj -scheme SudokuRivals -destination "$DEST" \
  -resultBundlePath "../../$RESULT" CODE_SIGNING_ALLOWED=NO test "$@"
