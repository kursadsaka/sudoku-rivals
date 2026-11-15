#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

rustup target add wasm32-unknown-unknown
# Must match wasm-bindgen in Cargo.lock; never install a floating CLI version.
if [ "$(wasm-bindgen --version 2>/dev/null || true)" != "wasm-bindgen 0.2.130" ]; then
    cargo install wasm-bindgen-cli --version 0.2.130 --locked
fi
sh scripts/web-build.sh
cargo build --locked --release -p sudoku-server

mkdir -p dist/web/pkg
cp "${CARGO_TARGET_DIR:-target}/release/sudoku-server" dist/sudoku-server
for file in index.html main.js session.js live.js sound.js cuelume.js styles.css sw.js manifest.webmanifest icon.svg icon-192.png icon-512.png icon-maskable-512.png apple-touch-icon.png; do
    cp "apps/web/$file" "dist/web/$file"
done
cp apps/web/pkg/sudoku_wasm.js apps/web/pkg/sudoku_wasm_bg.wasm dist/web/pkg/
