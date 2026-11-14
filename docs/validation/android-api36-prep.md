# Android API 36 / 16 KB release preparation

Follow-up: the full Android multiplayer matrix now passes on both the G14 and API36 APKs in the same API36/16KB emulator after test-fixture/lifecycle hardening. The failed attempts below are historical. See [comparison, regression proof and final results](android-multiplayer-investigation.md).

Base: `origin/agent/g14-integration` at `c6930b0d8b4ec6fd74a7aa4d4e5e493041f25b6f`, fetched 2026-11-15. Isolated branch `agent/android-api36-prep`; no G16 checkout edits.

## Toolchain and policy

- [Google Play target API policy](https://support.google.com/googleplay/android-developer/answer/11926878?hl=en): new phone apps/updates must target API 36 from August 31, 2026; extension deadline November 1, 2026 has passed. This is build preparation, not a Play submission.
- [Android 16 setup](https://developer.android.com/about/versions/16/setup-sdk): compile/target 36. Minimum SDK remains 26.
- [AGP 8.10 compatibility](https://developer.android.com/build/releases/agp-8-10-0-release-notes): API 36 supported, Gradle 8.11.1 and JDK 17. Use patch 8.10.1, existing Kotlin/Compose 2.0.21, Build Tools 36.0.0 and existing NDK 28.2.13676358.
- [16 KB guidance](https://developer.android.com/guide/practices/page-sizes): AGP >=8.5.1 handles uncompressed native ZIP alignment; NDK r28 supports 16 KB ELF alignment. Rust cdylibs additionally receive explicit `-Wl,-z,max-page-size=16384`. Validate **all** packaged libraries, including JNA, rather than only our own library.

SDK platform/build-tools 36 and Rust arm64 standard library installed only under the existing workspace-local `.tools`. No system package install, application ID change, account, billing, production, Play Console or release key operation. Gradle heap raised from default 512 MiB to 2 GiB after observed heap exhaustion; validation uses `--max-workers=2` to bound host load.

## Reproduction

Set `ANDROID_HOME` to a local SDK with platform 36, Build Tools 36.0.0 and NDK 28.2.13676358; `JAVA_HOME` to JDK 17; put Rust and Gradle 8.11.1 on PATH. An existing accepted SDK license was reused. Use a writable `TMPDIR`; on this host the sandbox/temp mount intermittently returned quota errors despite free disk space. `JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=$TMPDIR` was used for the retry.

```sh
scripts/android-build.sh --max-workers=2 assembleDebug assembleRelease bundleRelease lintDebug
cargo test --locked --workspace
scripts/web-build.sh
node tests/wasm.mjs
node tests/web-solo.mjs
node tests/ui-contract.mjs
# Start a separate loopback sudoku-server with SUDOKU_PORT=34891 first:
SUDOKU_SERVER_URL=http://127.0.0.1:34891 node tests/web-multiplayer.mjs --http
python3 tests/test_android_native_alignment.py
python3 tests/android-native-alignment.py \
  apps/android/app/build/outputs/apk/debug/app-debug.apk \
  apps/android/app/build/outputs/apk/release/app-release-unsigned.apk \
  apps/android/app/build/outputs/bundle/release/app-release.aab
# For each APK, using SDK Build Tools 36.0.0:
zipalign -c -P 16 -v 4 app.apk
# Official Google bundletool 1.18.3:
java -jar bundletool.jar dump config --bundle=apps/android/app/build/outputs/bundle/release/app-release.aab
```

The build script defaults to `assembleDebug` and always rebuilds both arm64-v8a and x86_64 Rust libraries plus Kotlin bindings. Generated files remain ignored. Release artifacts have no release signing configuration. Debug uses the standard local development signing configuration.

## Validation results

- `scripts/android-build.sh --max-workers=2 assembleDebug assembleRelease bundleRelease lintDebug lintRelease`: passed (111 tasks; final Gradle run 1m27s). Debug lint: 0 errors / 8 warnings / 5 hints; release lint: 0 errors / 4 warnings / 5 hints.
- Resolved lint errors without a blanket baseline: use UniFFI 0.32's `[defaults.bindings.kotlin] android = true` config (API >=34 SystemCleaner, older versions JNA fallback); explicit debug activity export and loopback-only `includeSubdomains=false`; narrow RestrictedApi suppression on the public Activity key-dispatch override, whose intermediate AndroidX implementation carries that annotation. No keyboard behavior change.
- `cargo test --locked --workspace`: passed 82 tests plus doctests. Initial run's doctest failed writing `/tmp` (quota); rerun with checkout-local TMPDIR passed.
- Rebuilt WASM; fixture/numeric/generation checks, web solo, UI contract and HTTP multiplayer all passed. HTTP used an owned server on 34891, not an existing service. First HTTP attempt lacked this server and timed out; not counted as a pass.
- Alignment checker regression: 2 tests (2 valid / 5 invalid ELF fixtures). Final debug APK, unsigned release APK and AAB each contain 6 native libraries: Rust, JNA, AndroidX graphics, each in arm64-v8a and x86_64. All 18 ELF LOAD checks and ABI machine checks passed; both APKs pass `zipalign -c -P 16 -v 4`. Bundletool reports `PAGE_ALIGNMENT_16K`.
- APK badging confirms package ID unchanged, compile/target SDK 36, min SDK 26. `apksigner verify` rejects unsigned release APK; `jarsigner -verify` reports AAB unsigned. No release key was provided or created.
- Isolated API 35 / 4096-byte emulator: solo and full G14 lifecycle suite passed, including verified race/duel results surviving expiry/cold restore. These ran the first API36-targeted APK before the lint/config-only follow-up.
- First API35 multiplayer attempt: post-leave replay and race 6x6 passed, then missing `room-join` at line 324. Investigation found the harness swipes at (540,1620) on an emulator still configured to 320x640 for solo; those coordinates are outside the display. Do not count this attempt as a full pass. The final APK was rerun at the harness's intended 1080x1920 / 420dpi; application invite logic is unchanged.
- Final APK API35 multiplayer retry also FAILED, at `tests/android-multiplayer.py:368`: `Divergent race board did not require explicit choice`. The 40-edit draft/cold-restore assertions passed and the external seat sync returned HTTP 200, but persisted `reconcile` did not become true within 20 s. Diagnostics showed race/playing/generation=2 and successful state polls. This was the initial unresolved acceptance blocker (superseded by the passing follow-up linked above); no full Android multiplayer matrix pass is claimed. No unrelated match/client behavior was changed to make the test pass. See `.tools/android-multiplayer-retry.log` and `.tools/multiplayer-retry-status.log`.
- Final API 36 / 16 KB x86_64 emulator: `ro.build.version.sdk=36`, `getconf PAGE_SIZE=16384`; final APK installed successfully. Full 320dp solo suite passed. Full G14 lifecycle suite also passed: slow create, Home/reopen exact setup replay, lost leave with background and process death, verified race/duel results surviving expiry/cold restore, atomic backup and new-room creation. Final status: `boot=0 solo=0 lifecycle=0`; lifecycle duration 216 s.

## Remaining limits

The release build retains the existing empty `MATCH_SERVER_URL` default; online play needs a separately approved endpoint via `-PsudokuMatchServerUrl=...`. No G16 endpoint or manifest change is included.

Release lint warnings remain: newer Activity/Compose dependencies available, dataExtractionRules recommendation, missing application icon. Debug adds development-link manifest warnings. No unrelated dependency refresh, icon design, staging/domain work, release signing or store submission was attempted. A successful unsigned build is not store-release acceptance. Physical ARM64 execution, API26 runtime and TalkBack were not tested.

## Final artifact SHA-256

```text
c303fb3a5601e94fee52f1f7618d839b6ae4a03551b19590d584917b82f50bf6  app-debug.apk
ffa22700c0cd352a23e4aa39a8efad92f4bf8f43e3b550c568e8f5a40f7fd237  app-release-unsigned.apk
09dd94d77fcf6894bfb16cce73a34da2298f75a811299ae14ce1da4d58818fde  app-release.aab
```

Android emulator commands (debug APK already installed; solo viewport 320x640 / density 160, multiplayer viewport 1080x1920 / density 420):

```sh
python3 tests/android-solo.py --adb "$ANDROID_HOME/platform-tools/adb" --serial emulator-5582
python3 tests/android-g14-lifecycle.py --adb "$ANDROID_HOME/platform-tools/adb" --serial emulator-5582 --server-bin "$PWD/target/debug/sudoku-server" --server-port 34894 --proxy-port 34895
python3 tests/android-multiplayer.py --adb "$ANDROID_HOME/platform-tools/adb" --serial emulator-5580 --server-bin "$PWD/target/debug/sudoku-server" --server-port 34892 --proxy-port 34893
```

Full local logs and tool artifacts are intentionally ignored under `.tools/`: `final-build.log`, `rust-tests.log`, `wasm-web.log`, `web-http.log`, `alignment.log`, `zipalign-{debug,release}.log`, `bundle-config.json`, `android-{solo,lifecycle,multiplayer}.log`, `api36-16k-runtime.log`, `android16k-*.log`, `artifact-sha256.txt`.
