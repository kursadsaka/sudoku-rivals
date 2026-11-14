# G14 independent review fixes

Base: `agent/g14-integration` at `10db2a307afc8e378087da5f4d9116188de43609`.

- Native Home navigation marks the cancelled setup as failed so reopening retries
  the same request and guest rather than leaving setup permanently pending.
- While leaving, native lifecycle recovery retries leave before resume. The core
  rejects a resume that would replace pending leave; normal move recovery is unchanged.
- Refused setup clears its retry deadline/backoff. A departed seat returns to
  terminal state; web tick and online events do not resume it. Web cache is v5.
- A validated closed snapshot for the same match cannot erase an already verified
  result. It ends transport recovery and retains the finished snapshot, including
  on save/restore. iOS/web render verified results before session-loss screens;
  Android already had that ordering. Rematch remains disabled after room loss.
  An unknown result still uses the lost-session screen.

## Regression coverage

`server/tests/client.rs` uses the real authoritative Service with an injected
monotonic clock, not a sleep: race/duel leave replay, rejected setup replay after
leave, and exact result expiry at 600,000 ms followed by poll/resume/cold restore.
Existing rematch tests verify that a genuinely new match replaces the old result.

`tests/web-multiplayer.mjs` uses rebuilt WASM and the actual transport, evaluates
main.js's online listener, checks no old-room resume after rejected setup retry,
and invokes the actual screen selection with known/unknown results. Its HTTP
matrix covers race/duel × 6/9 plus cold restore of lost setup responses.

`tests/android-g14-lifecycle.py` is for a **disposable emulator only** (it resets
its live save). It uses real UI, HTTP and a real local server, holds a successful
create response during Back, tests both fresh and previously departed clients,
and drops an acknowledged leave response before background/foreground. It checks
identical request hashes, the same seat/room, Home completion and no resume.
Race/duel expiry uses a protocol-valid closed response at server time +600,000 ms,
then checks the saved snapshot, result heading, disabled rematch and cold restore.
The real expiry boundary itself is covered by the Service test above.

## Commands and host constraints

Use an unused pair of local ports and an emulator created for this run:

```sh
cargo test --locked --offline --workspace
scripts/web-build.sh
node tests/wasm.mjs
node tests/web-solo.mjs
node tests/ui-contract.mjs
SUDOKU_SERVER_URL=http://127.0.0.1:3081 node tests/web-multiplayer.mjs --http
scripts/android-build.sh
python3 tests/android-g14-lifecycle.py --adb "$ANDROID_HOME/platform-tools/adb" \
  --serial emulator-5580 --server-bin "$PWD/target/debug/sudoku-server" \
  --server-port 3082 --proxy-port 3083
python3 tests/android-multiplayer.py --adb "$ANDROID_HOME/platform-tools/adb" \
  --serial emulator-5580 --server-bin "$PWD/target/debug/sudoku-server" \
  --server-port 3082 --proxy-port 3083
```

The web command requires a separately started, owned local server. The Android
scripts own their temporary server/proxy. Rust HTTP, Gradle, emulator and adb
needed execution outside the filesystem/network sandbox (loopback/KVM denied
inside); reruns used that access. Initial test harness corrections: closed
snapshots must hide cells/givens, comparisons exclude normal poll timestamps,
and expiry cases use independent clients because retained results reopen.

## Mac handoff (not executed on this Linux host)

Rebuild FFI/xcframework with `scripts/ios-build.sh`, then run
`python3 tests/ios-multiplayer.py` on a clean simulator. New Swift Testing cases:

- `homeCancelsSetupAndReopenRetriesIdenticalBody`: create and join, manual retry
  clock, cancelled callback cannot replace the current attempt.
- `lostLeaveThenBackgroundRetriesLeaveWithoutResume`: race and duel, forwarded/dropped
  leave followed by foreground; identical leave requests, Home, opponent result.
- `verifiedResultSurvivesExpiryPollAndResume`: race and duel, controlled expiry
  poll/410 resume, retained result and restored record. Expiry fixture deliberately
  empties hidden board fields, as required by the closed-snapshot protocol.

The existing iOS after-leave setup/background/cold restore suite remains intact.
Xcode, iOS compilation, Swift test execution, final result UI rendering on iOS,
physical devices and cross-platform simultaneous matches were not verified here.

## Linux validation (14 November 2026)

- Final `cargo test --locked --offline --workspace`: **80 passed, 0 failed**.
- Rebuilt WASM; fixture/generation/numeric, web solo and UI contract checks passed.
  Web multiplayer HTTP race/duel × 6/9 and all new regressions passed.
- Final Android debug APK built successfully. New lifecycle regressions passed on
  isolated API 35 x86_64 emulator at 320×640: fresh/after-leave Home cancellation,
  dropped leave/background, race/duel result expiry and cold restore. The expiry
  UI checks use controlled protocol responses, not ten minutes of wall-clock time.
- No iOS execution or cross-platform simultaneous match is claimed for this run.
- Existing `tests/android-multiplayer.py` on the final APK at 1080×1920/480 dpi:
  **exit 0**, race/duel × 6/9, deep links, exact lost-response retry, 40 offline
  edits/single sync, cold restart, reconciliation, failed-save retry, corrupt-save
  preservation, 90-second timer, mutual rematch, missing-opponent score/rematch,
  verified-result preservation, mode-specific failure copy, leave/server restart.
