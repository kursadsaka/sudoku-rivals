# Sudoku Rivals

- Read `docs/PROGRESS.md`, `docs/rules.md`, `docs/match-protocol.md` and `docs/ui-contract.md` before changing the relevant behavior.
- Rust workspace: `core`, `core-ffi`, `core-wasm`, `server`; platform UIs: `apps/web`, `apps/android`, `apps/ios`.
- Run `cargo test --locked --workspace`. Build WASM with `scripts/web-build.sh`, then run `node tests/wasm.mjs`, `node tests/web-solo.mjs`, `node tests/ui-contract.mjs` and `node tests/web-multiplayer.mjs --http`.
- iOS setup/build: `scripts/ios-build.sh`; existing integration suite: `python3 tests/ios-multiplayer.py`. Tool paths are machine-specific; `.tools` and generated bindings are ignored.
- Preserve concurrent edits and existing dev servers. Use separate checkout/build directories and bind only unused loopback ports for acceptance tests.
- Shared decisions and continuation context: Notes MCP, Sudoku Rivals note `d12f3995-0db4-4a1b-8488-74e8d30593f5`; consult Dev Best Practices there.
