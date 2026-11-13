#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
# Run on macOS with Xcode, xcodegen, rustup and cargo installed.
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo build --locked -p sudoku-ffi --target aarch64-apple-ios --release
cargo build --locked -p sudoku-ffi --target aarch64-apple-ios-sim --release
rm -rf apps/ios/generated
mkdir -p apps/ios/generated/headers
cargo run --locked -p sudoku-ffi --features bindgen --bin uniffi-bindgen -- generate --library target/aarch64-apple-ios-sim/release/libsudoku_ffi.a --language swift --out-dir apps/ios/generated --no-format
# Headers live in their own dir: an xcframework written inside -headers copies itself recursively.
cp apps/ios/generated/sudoku_ffiFFI.h apps/ios/generated/headers/
cp apps/ios/generated/sudoku_ffiFFI.modulemap apps/ios/generated/headers/module.modulemap
xcodebuild -create-xcframework -library target/aarch64-apple-ios/release/libsudoku_ffi.a -headers apps/ios/generated/headers -library target/aarch64-apple-ios-sim/release/libsudoku_ffi.a -headers apps/ios/generated/headers -output apps/ios/generated/SudokuFFI.xcframework
cd apps/ios
xcodegen generate
xcodebuild -project SudokuRivals.xcodeproj -scheme SudokuRivals -sdk iphonesimulator CODE_SIGNING_ALLOWED=NO build
