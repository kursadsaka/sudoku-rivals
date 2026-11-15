# G16 — Android staging bağlantısı

15 Kasım 2026. Taban `agent/g16-render-free` / `6e93373`. Render servisi, ayarları,
Notes, iOS/web ve association dosyaları değiştirilmedi; deploy, push, imzalı yayın yok.

## Değişiklik

- `apps/android/app/build.gradle.kts`: `staging` build type (`release`'ten türer).
  `MATCH_SERVER_URL` sabit `https://sudoku-rivals-g16-staging.onrender.com`;
  `-PsudokuMatchServerUrl` bunu değiştiremez. `STAGING=true`. `applicationIdSuffix
  .staging` → debug/release kayıt ve verisine dokunmaz. Yerel debug keystore ile imzalanır;
  yalnız emülatör/test içindir, yayın imzası değildir. Debug ve release varsayılanları
  (loopback / boş URL) aynı.
- `LiveState.normalizeOrigin`: staging'de yalnız bu tek origin kabul edilir. Başka
  origin'li kayıtlı oturum `protectedSave` ile reddedilir; kayıt üzerine yazılmaz.
  `dev_server_url` zaten `BuildConfig.DEBUG` ile kapalı; staging debug değildir.
- `LiveScreens`: staging'de düzenlenebilir sunucu alanı yok, sabit host gösterilir.
- Manifest etiketi `${appLabel}` (debug/release "Sudoku Rivals", staging "Sudoku Rivals Staging").
- Cleartext, `networkSecurityConfig`, loopback ve `autoVerify` staging APK'sında yok.

## Kanıt

- `scripts/android-build.sh assembleDebug assembleStaging assembleRelease`: başarılı.
- `python3 tests/android-staging.py --aapt2 <aapt2>`: 15 statik kontrol (BuildConfig,
  paket kimliği, debuggable değil, cleartext kapalı, ağ yapılandırması/VIEW filtresi yok,
  debug loopback korunuyor, release manifest değişmedi).
- Emülatör API 36 / 16 KB (`getconf PAGE_SIZE=16384`), `--live`: sabit host görünür,
  sunucu alanı yok, `dev_server_url` etkisiz; başka origin'li üç kayıt (loopback,
  `example.invalid`, `<staging host>.evil.example`) reddedildi ve bayt bayt aynı kaldı;
  tek HTTPS create → lobi ("Bağlandı"), soğuk yeniden açılış ve aynı paketin
  `install -r` güncellemesi sonrası aynı oda sürdü; leave onaylandı, ana ekrana dönüldü.
- Staging istekleri: `/health` GET 1; uygulamadan 4 başarılı create (3'ü test betiği
  geliştirilirken) + 1 reddedilen create ve her odada leave. Reddedilen create "Sunucu
  şu anda yeni işlem kabul etmiyor" mesajı aldı (ortak 10 dk kotası/kapasite; istemci
  hata gösterdi, sonuç uydurmadı). Kota penceresi dolunca son tam koşu geçti. Hiçbir
  oda açık bırakılmadı.

## Açık / bulgular

- Düello 9×9 `generation >= 2` (satır 446) hatası **test kırılganlığıydı, ürün kusuru
  değil.** `background()` yalnız `dumpsys activity activities` çıktısında herhangi bir
  `state=STOPPED` arıyordu; çıktı komşu kayıtları da listeler (uygulama `RESUMED` iken
  launcher `STOPPED`) ve koşul uygulama durmadan doğru çıkabiliyordu. Ana ekran tuşu
  işlenmeden `launch()` gelirse `onStop` → `setForeground(false)` → `resume` hiç çalışmaz,
  generation 1'de kalır ve `/state` yoklamaları sürer. Düzeltme: `activity_state()` yalnız
  uygulamanın kendi `MainActivity` kaydının durumunu okur (`RESUMED→PAUSED→STOPPED`
  emülatörde izlendi). Gerçek `dumpsys` örnekleriyle 4 birim testi eklendi.
- Kanıt: beklemesiz `background()` ile aynı hata imzası iki koşuda da yeniden üretildi
  (düello, generation 1, aynı onaylama); gerçek durma beklenince `/resume` launch'tan
  ~0,2 sn sonra gidiyor. Düzeltilmiş tam süit API 36 emülatörde geçti. Sınır: eski
  harness'in hata veren üç koşusunda koşulun erken doğru çıktığı anı yakalamadım; sonraki
  yedi koşu (taze emülatör, yük altında üçü dahil) eski haliyle de geçti. Bağlantı, aynı
  imza + kanıtlanmış yanlış pozitiften çıkarımdır. `tests/android-g14-lifecycle.py` bu
  koşulu kullanmaz (sabit 1,5 sn bekler) ve incelenmedi.
- Gerçek cihaz, imzalı yayın, App Links/`assetlinks.json`, production, iOS yok.
- Free servis uykudayken ilk istek ~1 dk sürebilir; staging bellek içi olduğundan
  restart/uyku odaları kaybettirir (bkz. staging-render.md).
