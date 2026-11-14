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

## Independent review follow-up

Scope: follow-up to `a9c5413` and the independent Claude review, on the same isolated branch. No iOS/web application source was edited.

- Shared Rust now clears retry deadlines when a room/session is terminally lost or leave is acknowledged. `resume` rejects lost sessions; `retry` also rejects a lost session without a new explicitly requested setup. Android's existing tick guard remains an extra guard.
- Serialized Service regression covers both modes, room missing/closed/unauthorized, repeated retry/resume/poll, network-failed callbacks, cold restore, and creating a new session after loss. A real rebuilt WASM adapter regression verifies no further HTTP calls through tick/online resume/cold restore.
- Android remembers only a server-verified **finished duel** opponent score, keyed by match id, in a separate atomic presentation-cache file. A missing current score uses that cache only for the same finished match; new matches and games never inherit it. If the final score was never received it remains unavailable. Cache write failure retains the in-memory value, but cannot guarantee it survives process death.
- Optional score reads cannot throw on a missing score; unavailable scores are explicitly labelled in game and result views. Although the wire struct's field is optional across modes, the shared validator already requires it for a present duel opponent. A regression confirms absent/null duel scores are rejected atomically before presentation.
- A temporarily disconnected opponent remains eligible for a queued rematch; the server still requires both players connected/requesting before starting. The UI explains the wait. A departed seat cannot receive a rematch; the copy says so directly. Lost own sessions require a new session, and a lost-session invite explains the reset path rather than an impossible leave.
- Offline race-draft copy now applies only while playing, not in lobby or after a finished result.

Compatibility: protocol v1, opaque Rust save v1, and FFI/WASM API shapes are unchanged. Existing Mac iOS/web commits `872c1f3` and `d89a577` remain separate and unmodified. Their native/WASM artifacts must be rebuilt with this common-core fix to receive it; an old binary does not gain it automatically. The Android-only presentation cache is not a protocol migration or shared credential store. Mac UI wording/result routing and equivalent final-score caching still require checking against those unpublished sources; cross-device UI parity is not claimed.

Validation for this follow-up: 72 core/server tests passed; rebuilt WASM fixture/solo/boundary tests, web multiplayer adapter including terminal-loss regression, and UI contract passed. Updated Android matrix passed completely (exit 0), including final opponent score 10 surviving post-result departure and cold restore, no score inheritance into a new match, and queued rematch during temporary disconnect. The passing run is retained in `g14-review-android.log`. The first Android attempt was blocked by an emulator System UI ANR, not an application crash, and is not counted as a pass.

The debug APK native-library SHA-256 matches the rebuilt shared Rust library: `a77b4f3b2160349f18604dde8fc14ef8198de0daedf69c574acda90239cf619b`. Android test cleanup restored the prior session/solo/score-cache records and airplane mode. Mac simulator, real hardware and TalkBack remain unverified.

## Second review follow-up (base `ecbbf07`)

The three new findings from the Claude re-review are addressed without changing protocol v1, opaque save v1, FFI signatures, or iOS/web application sources:

1. Server rematch eligibility is shared between rematch-command processing and resume. If both requests accumulate while a player is disconnected, the last resume starts exactly one new countdown. The regression covers both game modes, a new match id, unchanged countdown on repeated resume, and replay of the old rematch receipt.
2. Shared client `resume` returns `Busy` while create/join is pending, including after network failure. It leaves the pending descriptor and save untouched. The serialized Service regression covers create/join, prior explicit leave/room loss, in-memory/cold-restored state, unchanged replay bodies, and reclaiming the same server-created room/seat. The rebuilt WASM adapter regression also exercises an online callback before/after failure.
3. Android continues ticking a pending new setup after voluntary leave; it does not resume/poll the old departed room. A cold-restored setup opens directly and retries; lifecycle cancellation marks pending HTTP failed even after leave. Explicit create/join clears the old resume requirement. The existing “Yeniden bağlanılacak” copy now matches automatic retry. The emulator fault proxy accepts a new room then drops its response, keeps the network offline, and checks background/foreground, cold restart, exact-body replay and reclaiming the same room with no extra create-button press.

The Android rematch scenario now uses the formerly deadlocking order: opponent requests, disconnects, Android requests, opponent resumes. No extra rematch command is sent after resume. Tests use the freshly rebuilt local server binary explicitly, not the older `/tmp` server artifact.

Validation: 74 core/server tests passed; rebuilt WASM fixture/solo/numeric tests, web adapter regressions and UI contract passed. The full Android matrix passed (exit 0), including post-leave automatic setup recovery and queued rematches starting on resume alone. The direct test-process stdout/trace is retained in `g14-rereview-android.log`. One initial run hit the existing 20-second race-reconciliation wait; the same application build completed the full rerun successfully. No crash/ANR was recorded for that timed-out run. The APK's embedded native library matches the rebuilt library SHA-256 `73e36031944f94bce366defa1ad779c42fffdcf7904a5365dccdc705f743fd02`.

Mac `872c1f3`/`d89a577` and previous commits are preserved, with no merge, push or publication. Mac clients need the rebuilt common Rust outputs; the rematch fix also requires this server build. No Mac simulator, physical arm64 device or TalkBack validation is claimed.
