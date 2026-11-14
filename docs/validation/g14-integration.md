# G14 integration — 2026-11-14 (Mac)

## Lineage

- Branch `agent/g14-integration`, created in a fresh clone from the Mac branch `agent/g14-cross-platform` (`d89a577`, parent `872c1f3`).
- Merged GitHub `agent/g14-android-recovery` at `a528e0b` (merge base `b68b695`). The textual merge was clean. Both lines are preserved unchanged in history.
- The previous Mac checkouts and the local `sudoku-rivals-g03-ios` repository were not modified.

## Compatibility review of the shared-core changes on iOS/web

| Core change (Android branch) | Web (`live.js`) | iOS (`LiveSession.swift`) |
|---|---|---|
| `retry_at` cleared on terminal loss/leave; `resume`/`retry` reject lost sessions | `tick` retries only when `retry_at` is due and falls back to a quiet `resume`; both now stay terminal (covered by the WASM regression) | `tick` skips resume when `session_lost`; the retry branch can no longer reopen a lost session |
| `resume` returns `Busy` while create/join is pending | `enter()` resumes only without `pending_http`; online-callback regression passes | resume branch is only taken while the seat is held, which excludes a new setup |
| Create/join clears the old `needs_resume` | no adapter change needed | no adapter change needed |
| Server starts a queued rematch on resume | no client change; rematch stays enabled while the friend is temporarily disconnected | same; new live test below |

**iOS defect found and fixed.** This is the iOS counterpart of the Android fix "continue ticking a pending new setup after voluntary leave". Suppose you leave a match and then create or join a new room, and that response is lost. Before the fix:

- `tick()` returned early because of `voluntarilyLeft`.
- Backgrounding did not mark the in-flight setup failed.

As a result, the setup screen stayed disabled on "waiting" with no retry, including after a cold restore. Now:

- `tick()` runs while a pending setup exists after leaving. The old room is still never resumed or polled.
- Backgrounding marks any pending HTTP request failed.

The new live test failed without the fix (`retryAtMs` stayed nil, with no recovery in 20 s) and passes with it.

The web service-worker cache was bumped to `v4` because the rebuilt WASM asset changed and the cache serves assets cache-first.

## Checks on this Mac

- `cargo test --locked --offline --workspace`: 76 passed.
- WASM rebuilt (`scripts/web-build.sh`, wasm-bindgen 0.2.130). `tests/wasm.mjs`, `tests/web-solo.mjs`, `tests/ui-contract.mjs` and `tests/web-multiplayer.mjs` passed. The last one includes the terminal-loss and pending-setup regressions.
- `tests/web-multiplayer.mjs --http` passed against a freshly built server on owned port 3191, covering race and duel at 6×6 and 9×9, plus lost create/join cold restore.
- iOS: the FFI static libraries for device and simulator, the UniFFI bindings and the xcframework were rebuilt from the merged core, and the app was built.
- `tests/ios-multiplayer.py` ran on the unused, shut-down "Sudoku G14 isolated" (iPhone 17) simulator with owned ports 3101/3102: **39/39 passed** (25 unit/live + 14 XCUITest). The live tests used two real Swift adapters against the merged local server. They include:
  - `newRoomAfterLeaveRecoversLostSetupLiveAndRestored`: an in-flight setup is backgrounded and then retried with the exact same body. Separately, a dropped response is recovered with the exact body after a cold restore. Neither path sends any request to the departed room.
  - `queuedRematchStartsWhenDisconnectedFriendResumes`: the friend requests a rematch and goes to the background. The server marks the friend disconnected (15 s heartbeat) and the host requests a rematch. The friend's foreground resume alone then starts one new match, with exactly one rematch command per side.

## Limits

- No Android SDK, JDK, emulator or device exists on this Mac, so no Android build or test ran here. Android results are those recorded in `g14-android.md` (ASUS emulator, before this merge). The merged Android sources are byte-identical to `a528e0b`.
- Cross-platform matches were checked only as automated local protocol tests: iOS Swift adapter ↔ server, and WASM web adapter ↔ server. No simultaneous iOS ↔ Android or web ↔ Android match ran, and no browser UI automation ran in this pass. The manual iOS ↔ browser harness (`tests/ios-web-local.py`) was not re-run.
- No physical iOS device, VoiceOver or TalkBack testing was done.
- The iOS result screen does not cache a departed friend's final duel score the way Android now does. It shows only your own score, never a fake 0. This difference in presentation is left open.
