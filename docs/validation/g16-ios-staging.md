# G16 — iOS staging hazırlığı

15 Kasım 2026. Taban `agent/g16-render-free` / `8dc89cc`; çalışma dalı `agent/g16-ios-store-prep`
(ayrı temiz klon). Render servisi/ayarları, hesap, imza, ödeme, App Store, production,
Notes ve mevcut checkout'lara dokunulmadı; push yok. Staging servisine **hiç istek
atılmadı** (askıda olduğu için; Android turundaki canlı create/leave iOS'ta yapılmadı).

## Değişiklik

- `project.yml`: `Staging` configuration (Release'ten türer), `SudokuRivalsStaging` şeması.
  Staging'de bundle ID `com.example.sudokurivals.staging` (yer tutucu), ad "Sudoku Rivals Staging",
  `SWIFT_ACTIVE_COMPILATION_CONDITIONS=STAGING`.
- `LiveSession`: `stagingOrigin` sabiti; `STAGING`'de `releaseOrigin` bu olur ve `pinsOrigin`
  true'dur. `init(pinOrigin:)`: sabitlenmişse kayıtlı origin sabitle eşit değilse kayıt yüklenmez
  (`protectedSave`; üzerine yazılmaz, reset kopyasını alır). `AppModel`: `DEBUG && STAGING` `#error`.
- `LiveSetupView`: sabit host satırı (`live-server-fixed`), TR/EN metin.
- `PrivacyInfo.xcprivacy`: `NSPrivacyTracking=false`, alan adı yok, `SystemBootTime` / `35F9.1`
  (`clock_gettime_nsec_np(CLOCK_MONOTONIC_RAW)` kullanımı için tedbir; Apple listesinde bu
  çağrının tam yeri belirsiz, fazla beyan zararsız). `NSPrivacyCollectedDataTypes` bilerek
  yok (aşağıda karar listesi).
- `tests/ios-staging.py`: statik .app kontrolü.

## Kanıt (Mac, Xcode 27.2, iOS 27.2 simülatörü, özel simülatör `g16-ios-store-prep`)

- `scripts/ios-build.sh` (Debug) BUILD SUCCEEDED.
- Birim testleri (`-only-testing:SudokuRivalsTests`): 32 test / 3 suite geçti; yeni 3 test:
  sabit HTTPS staging host, staging oturumu yalnız sabit https origin'e istek atar,
  eşleşen kayıt korunur / başka origin'li kayıt yüklenmez ve değişmez.
- `python3 tests/ios-staging.py`: 8/8 PASS (bundle ID, ad, ATS anahtarı yok, URL şeması yok,
  gizlilik manifesti, sabit origin derlenmiş, ikilide loopback/test kancası yok).
- Release configuration, genel iOS SDK, imzasız derleme: BUILD SUCCEEDED; ikilide
  `http://127.0.0.1` yok.
- Simülatör: staging uygulaması kuruldu/açıldı; "Play with a friend" ekranı
  "Test server: sudoku-rivals-g16-staging.onrender.com" gösterir (ağ isteği yok).
- `python3 tests/ios-multiplayer.py --skip-build` (Debug, yerel sunucu :3101, hata proxy'si :3102,
  simülatör `g16-ios-store-prep`): TEST SUCCEEDED — 32 birim/canlı adaptör + 16 XCUITest, 0 başarısız
  (UI testleri 813 sn). Ortam notu: WASM paketi (`apps/web/pkg`) bu klonda derlenmediği için sunucu
  `SUDOKU_WEB_ROOT` ile geçici bir taslak web köküne yönlendirildi (iOS testleri statik varlık
  kullanmaz); wasm32 hedefi kurulmadı, web/WASM testleri bu turda çalıştırılmadı.

## Kullanıcı kararı gerektirenler / yapılmayanlar

- Gerçek bundle ID, Team, imza/provisioning, App Store Connect kaydı; Universal Links
  (alan adı + `apple-app-site-association` + Associated Domains) — yer tutucu ID uydurma dosya yayımlamaz.
- App Store gizlilik cevapları ve `NSPrivacyCollectedDataTypes`: sunucuya görünen ad ve
  rastgele misafir kimliği gidiyor; "toplanan veri" beyanı ürün/hukuk kararıdır.
- `ITSAppUsesNonExemptEncryption` (ihracat uyumu beyanı) eklenmedi.
- Uygulama ikonu (1024 px) ve asset catalog yok; tasarım kararı. Ekran görüntüleri,
  sürüm/marketing numarası, yaş derecelendirmesi, destek/gizlilik URL'si.
- Staging'e gerçek create/join denemesi (ortak 5 create/10 dk kotasını kullanır; servis
  askıda). Gerçek cihaz, VoiceOver.
