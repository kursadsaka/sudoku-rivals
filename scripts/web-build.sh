#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --locked -p sudoku-wasm --target wasm32-unknown-unknown --release
wasm-bindgen "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/sudoku_wasm.wasm" --target web --out-dir apps/web/pkg
