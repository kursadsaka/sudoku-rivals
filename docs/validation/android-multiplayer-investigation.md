# Android multiplayer acceptance investigation

Branch: `agent/android-api36-prep`. Production comparison: G14 `c6930b0` versus API36 preparation `a3690d1`. No production app/client/server behavior was changed in this follow-up; changes are acceptance fixtures, lifecycle/viewport synchronization, diagnostics and regression tests.

## What is established

The old failure log recorded only `reconcile=false`, race/playing/generation=2 and successful polls. It did not retain the foreign command receipt, the pending sync or both boards. It cannot establish the exact historical event order. We do not claim a proven API36 product regression or silently label the old failure a pass.

Both actual APKs pass the original focused divergent-board scenario on the **same API36 / 16 KB x86_64 AVD**, with clean app data, the same backend/WASM bot, ports and viewport. The foreign command is accepted (`receipt.code=ok`, board_version 0 -> 1), the local value 2 survives cold restore, the distinct server value 1 triggers explicit reconciliation and keeping local sends one sync. The original scenario is intermittent rather than a deterministic API36 regression.

A deterministic test-data ambiguity is verified independently: a pending sync with value 1, followed by 39 more local edits ending at 2, can cold-resume with the server acknowledging value 1. The correct behavior is **no conflict**: acknowledge the uncertain sync, retain all 40 undo entries and send the newer local board. A foreign value 3 instead requires explicit reconciliation. `core/src/multiplayer.rs::accept_board` already distinguishes these; its production source is identical in both heads (SHA-256 `1a869c52fb29b894c05a17b2bf779c32ec6ae04ccf2d73a313475f8bda3f19d3`). The new Rust test exercises both outcomes in 6x6 and 9x9 against the real Service and passes on both heads.

The old Android fixture hard-coded foreign value 1 and only checked HTTP 200. It neither excluded an uncertain sync ACK nor verified that the server accepted/applied a new board; HTTP 200 also carries rejected/no-change command receipts. The fixed fixture chooses a value different from acknowledged, final-local **and pending-sync** values, and asserts `code=ok`, exact applied cells and version +1 before reconnecting. The Python regression covers both board sizes with no pending sync, pending=1 and pending=2. Substituting the old constant fixture fails exactly the two pending=1 cases; the fixed fixture passes.

Two independent harness assumptions were also exposed:

- Scroll coordinates (540,1620) were outside the solo-test viewport 320x640. They now use current display dimensions.
- The diagnostic full API36 run passed both races and duel 6x6, but failed the duel 9x9 generation assertion after HOME + a fixed 1-second sleep. That sleep did not prove an Activity stop. Both lifecycle probes now wait for the app's actual `state=STOPPED` before foregrounding, so the test cannot claim to exercise resume without stopping the Activity.

The comparison AVD also initially showed a **System UI ANR dialog** covering the application. This run was not counted as an app failure or pass. Only this owned AVD's dialog was dismissed. Old test app data was archived before clean comparison starts; no user device, running user app or staging checkout was used.

## Comparison and commands

G14 was extracted with `git archive c6930b0` into ignored `.tools/baseline-src`; only its Gradle heap was increased to 2 GiB to avoid the previously observed build OOM. Its original build script/AGP/target35/native sources were otherwise used. The new Rust regression test was subsequently copied into that isolated tree for the old-production-source test. The acceptance driver and unchanged backend/WASM bot were shared, so the tested variable is the installed APK.

APK hashes:

```text
2bf10565a8c7530897bd40203eade1277aa563d11f0cadaa88df31ce3fca8581  G14 c6930b0 debug APK, target35
c303fb3a5601e94fee52f1f7618d839b6ae4a03551b19590d584917b82f50bf6  API36 a3690d1 debug APK, target36
```

Commands from the repo root, with local Rust/JDK/Gradle/Android paths configured as in `android-api36-prep.md`:

```sh
python3 tests/test_android_multiplayer_fixture.py
cargo test --locked -p sudoku-server --test client cold_resume_distinguishes_uncertain_sync_ack_from_divergence
# The same new test against the isolated G14 production sources:
(cd .tools/baseline-src && cargo test --locked -p sudoku-server --test client cold_resume_distinguishes_uncertain_sync_ack_from_divergence)
# Install each APK on owned emulator-5582, then use the same driver:
python3 -u tests/android-multiplayer.py --adb "$ANDROID_HOME/platform-tools/adb" --serial emulator-5582 --server-bin "$PWD/target/debug/sudoku-server" --server-port 34902 --proxy-port 34903 --reconcile-only
python3 -u tests/android-multiplayer.py --adb "$ANDROID_HOME/platform-tools/adb" --serial emulator-5582 --server-bin "$PWD/target/debug/sudoku-server" --server-port 34902 --proxy-port 34903 --trace
```

The AVD is `api36-16k-prep`, API36, PAGE_SIZE=16384, 320x640/density160. App data resets are restricted to this owned test AVD between APK comparisons. The test itself backs up/restores its private records and airplane mode. Server binds only unused loopback ports; no existing service was stopped. A new owned web-test server used port 34904 and was terminated by its own shell trap.

## Results

- Original focused scenario: G14 PASS, API36 PASS (`reconcile-{g14,api36}-original.log`).
- Deterministic cold-restore ACK/conflict regression: G14 PASS, API36 PASS, four size/remote-value cases per head.
- Fixed Python fixture: PASS, six size/pending-value cases. Old-fixture negative control: two expected failures.
- `cargo test --locked --workspace`: PASS, 83 tests plus doctests.
- Rebuilt WASM; wasm, web solo, UI contract and full HTTP multiplayer tests: PASS.
- Corrected full Android matrix, G14 target35 APK on API36/16KB: PASS (exit 0), all race/duel x 6/9 combinations plus final loss/corruption checks, approximately 449 s.
- Corrected full Android matrix, API36 APK: PASS (exit 0), all four combinations plus final loss/corruption checks, approximately 449 s on the same AVD with SDK=36 and PAGE_SIZE=16384 verified before launch.
- Both full runs include explicit reconcile, 40 offline edits / one sync, deep links, durable setup replay, failed-save retry, authoritative 90-second duel, mutual rematch, missing-opponent score/rematch, verified-result preservation, mode-specific failure copy, explicit leave, server restart and corrupt-record preservation.
- Restored the owned AVD's archived pre-comparison app files and byte-verified them before shutting down only that AVD. No user service or data was replaced.
- No current acceptance blocker remains in these automated suites; the historical-log limitation below remains explicit.

Local ignored evidence under `.tools/`: `baseline-build.log`, `baseline-install.log`, `reconcile-setup.log`, `reconcile-{g14,api36}-original.log`, `reconcile-{g14-core,core}-regression.log`, `reconcile-fixture-{tests,red}.log`, `reconcile-comparison-status.log`, `matrix-{g14,api36}-final.log`, `matrix-api36-diagnostic.log`, `multiplayer-workspace-tests.log`, `multiplayer-web.log`. Early System UI/dirty-start attempts are not passing evidence. No credentials are printed in acceptance diagnostics.

## Limits

The precise cause of the single historical reconcile timeout cannot be reconstructed from the old log. The test ambiguity is independently reproducible; the new acceptance preconditions make any recurrence distinguishable from it. No product workaround was introduced without evidence. Physical ARM64, TalkBack, staging, accounts, signing, merge and publication are outside this follow-up.
