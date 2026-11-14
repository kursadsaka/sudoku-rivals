#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
: "${ANDROID_HOME:?Set ANDROID_HOME to your Android SDK}"
: "${JAVA_HOME:?Set JAVA_HOME to JDK 17}"
NDK="$ANDROID_HOME/ndk/28.2.13676358"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android26-clang"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android26-clang"
rustup target add aarch64-linux-android x86_64-linux-android
# Explicit for Rust's cdylib link, independently of the NDK driver defaults.
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="${CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS:-} -C link-arg=-Wl,-z,max-page-size=16384"
export CARGO_TARGET_X86_64_LINUX_ANDROID_RUSTFLAGS="${CARGO_TARGET_X86_64_LINUX_ANDROID_RUSTFLAGS:-} -C link-arg=-Wl,-z,max-page-size=16384"
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
cargo build --locked -p sudoku-ffi
cargo run --locked -p sudoku-ffi --features bindgen --bin uniffi-bindgen -- generate --library "$TARGET_DIR/debug/libsudoku_ffi.so" --language kotlin --config apps/android/uniffi.toml --out-dir apps/android/app/src/main/java --no-format
cargo build --locked -p sudoku-ffi --target aarch64-linux-android --release
cargo build --locked -p sudoku-ffi --target x86_64-linux-android --release
mkdir -p apps/android/app/src/main/jniLibs/arm64-v8a apps/android/app/src/main/jniLibs/x86_64
cp "$TARGET_DIR/aarch64-linux-android/release/libsudoku_ffi.so" apps/android/app/src/main/jniLibs/arm64-v8a/
cp "$TARGET_DIR/x86_64-linux-android/release/libsudoku_ffi.so" apps/android/app/src/main/jniLibs/x86_64/
if [ "$#" -eq 0 ]; then set -- assembleDebug; fi
(cd apps/android && gradle --no-daemon "$@")
