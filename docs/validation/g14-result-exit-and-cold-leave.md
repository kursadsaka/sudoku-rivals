# G14 follow-up: result recovery and durable leave

Base: `agent/g14-integration` at `c6ced3992e2ebb43166a8bfca4ec2df62ed06dbf`.

## Changes

- The iOS verified-result screen now exposes the existing preserve-and-reset
  action when `connection == session_lost`, matching Android. It keeps the same
  confirmation, cancellation and `LiveStore.preserve()` failure handling; normal
  connected results retain rematch without a reset button.
- A pending explicit leave is now optional state in the existing core save.
  Restore validates its exact body/token, authenticated room path, POST method
  and request id; it cannot coexist with setup. Old saves without it still load.
  Restore retries the same descriptor before resume, even if the previous
  connection was terminal. Acknowledgment clears leave and any uncertain command.
- The private native leaving flag is restored from the credential-free
  `pending_leave` view flag, so the acknowledged cold replay returns Home.
  iOS pending setup also correctly owns recovery before resume after cold start.
- Web needs no new adapter logic: its existing durable retry and onLeft callback
  handle restored leave. Its asset cache is bumped to v6 for rebuilt WASM.

## Regression tests

- Rust/real Service, race and duel: process death before and after leave is
  accepted; exact replay, resume remains Busy, one forfeit timestamp, uncertain
  command cleared and no pending leave after acknowledgment. Tampered saved
  methods, ids, room paths, token bodies, missing auth and mixed setup are rejected.
- Web/real rebuilt WASM: dropped leave response, save/restore, same body replay,
  no resume, exactly one Home callback.
- Android real UI/proxy/server: cold death after accepted leave returns Home
  using identical leave hashes without resume. The result-expiry tests now use
  the actual recovery button, verify an exact preserved backup, then create a
  new room. They no longer delete the result save as a shortcut between modes.
- iOS adapter tests: restored leave returns Home; expired-result reset preserves
  exact original bytes before replacing the current record.
- New XCUITests for race and duel: server-verified win → server restart → known
  result/lost connection → cold launch → recovery confirmation cancellation →
  confirmation → create a new room. They fail if the recovery button is absent,
  and check the button is absent on connected results. Server restart exercises
  the same terminal state as room expiry; the previous Service expiry test still
  checks the exact 600,000 ms boundary.

## Mac execution required

This Linux host has no Xcode or iOS simulator. No iOS build, Swift test or UI test
execution is claimed. On Mac rebuild the core/xcframework using
`scripts/ios-build.sh`, then run `python3 tests/ios-multiplayer.py`. The new tests
are `testExpiredRaceResultCanPreserveSaveAndCreateNewRoom`,
`testExpiredDuelResultCanPreserveSaveAndCreateNewRoom` and
`coldRestoreReplaysAcceptedLeaveAndReturnsHome` (race/duel).

## Host issues

Sandbox startup hit the /tmp user's quota; only this implementation's previous
logs were moved into the workspace, and new evidence logs use the workspace.
Loopback/KVM tests used approved execution outside the sandbox. Gradle's Kotlin
compiler daemon failed but its fallback completed the APK build successfully.
The initial Android launch ran before emulator boot was complete; it was rerun
on the booted emulator. A new web fixture initially used a different guest from
its fake Session; the fixture was corrected and tests rerun.

## Validation on this host

- `cargo test --locked --offline --workspace`: **82 passed, 0 failed**.
- Rebuilt WASM: fixtures/generation/numeric, web solo, UI contract and web HTTP
  race/duel × 6/9 passed, including the new cold-leave replay regression.
- Rebuilt Android APK: new API 35 x86_64 real UI regression passed at 320×640.
  Race and duel both preserve the result through closure/cold launch, copy the
  current record exactly via recovery, and create a new room. Background and
  process-death leave replay both return Home without resume.
- Existing full `tests/android-multiplayer.py` on the rebuilt APK at
  1080×1920/480 dpi: **exit 0**. Race/duel × 6/9, lost setup response, offline
  edits/single sync, cold restart, reconciliation, save failure/corrupt record,
  90-second result, rematch, result preservation, explicit leave and server restart
  all passed. No iOS runtime result is implied by these Android checks.
