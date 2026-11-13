#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
: "${ANDROID_HOME:?Set ANDROID_HOME to your Android SDK}"
: "${JAVA_HOME:?Set JAVA_HOME to JDK 17}"
NDK="$ANDROID_HOME/ndk/28.2.13676358"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android26-clang"
rustup target add x86_64-linux-android
cargo build --locked -p sudoku-ffi
cargo run --locked -p sudoku-ffi --features bindgen --bin uniffi-bindgen -- generate --library target/debug/libsudoku_ffi.so --language kotlin --out-dir apps/android/app/src/main/java --no-format
cargo build --locked -p sudoku-ffi --target x86_64-linux-android --release
mkdir -p apps/android/app/src/main/jniLibs/x86_64
cp target/x86_64-linux-android/release/libsudoku_ffi.so apps/android/app/src/main/jniLibs/x86_64/
(cd apps/android && gradle --no-daemon assembleDebug)
