# G14 Android validation — 2026-11-14

## Isolation

- Repository: `sudoku-rivals-repo`; initial tree clean, base `b68b695`.
- Independent local branch: `agent/g14-android-recovery`.
- No merge, push, deployment, production access, or iOS/web source edits.
- Mac-reported `872c1f3` and `d89a577` on `agent/g14-cross-platform` are not available in this checkout. They were not fetched, reset, or overwritten. Their contents and runtime behavior remain unverified here.

## Changes

- Disconnect and unrecoverable-session copy distinguishes race drafts from server-authoritative duel moves/results; room-loss banners do not contradict an existing verified result.
- Terminal `session_lost` stops the Android tick before retry/resume. A stale retry deadline previously reopened the terminal state and made the recovery screen unstable. The emulator regression requires stable loss and no new HTTP calls for three seconds.
- A missing opponent score is explicitly unavailable rather than zero.
- Rematch requires a present, connected opponent and an active connected session.
- An already verified finished result survives subsequent session loss. A lost finished session offers an explicit reset that archives the existing record.

## Checks

- Offline Gradle `assembleDebug`: passed; debug APK installed on Android API 35 x86_64 AVD `sudoku-g03` (emulator 37.2.13).
- `cargo test --locked --offline -p sudoku-core -p sudoku-server`: 70 tests passed, including serialized multiplayer recovery and real local HTTP tests.
- `node tests/ui-contract.mjs`: passed.
- Extended `tests/android-multiplayer.py`: final complete rerun passed on the final APK (exit 0), using ports 3101/3102. Both race sizes, both duel sizes, both failure-copy scenarios, stable terminal loss/no further HTTP, missing-opponent score/rematch, verified-result preservation, and corrupt-record recovery passed. Original app records and airplane mode were restored by test cleanup.
- Initial full emulator run completed all four match/rematch/departure scenarios but failed during subsequent recovery. A focused replay exposed the stale-retry bug above; neither incomplete run is counted as a complete pass.

The Android suite owns disposable HTTP/proxy ports 3101/3102 and uses the web WASM transport as its opponent. It restores pre-existing app records and airplane mode in `finally`. It covers both modes and sizes, deep links, exact-body retry, offline race edits, cold restore, reconciliation, save failures, corrupt-record preservation, authoritative duel time, mutual rematch, departure, mode-specific failure copy, and preserving known results after server restart.

## Limits

Agent device access is disabled in this environment; local SDK/ADB was usable outside the socket-restricted sandbox. The first full-workspace Rust build hit the sandbox disk quota; the narrower relevant suite passed using a separate workspace target directory.

No physical Android device, TalkBack, Mac iOS simulator, or simultaneous iOS/web/Android UI run was verified. A reachable Mac environment alone does not establish cross-device validation. The WASM opponent is an actual web client transport, not a browser UI and not the unpublished Mac web branch.
