# G16 — Render Free staging

Bu yapılandırma yalnız ayrı, yeni bir staging Web Service içindir. Production,
DNS, ücretli disk/veritabanı, mağaza veya imza anahtarı gerektirmez. Kaynak dal
`agent/g16-render-free`; otomatik deploy kapalıdır. Kullanıcının son bilgisine
göre kartsız Free servis kurulmuş, test sonrası askıya alınmıştır. Bu kod turu
Render servisine veya ayarlarına dokunmaz. `<HOST>` mevcut servisin hostname'i
için bir placeholder'dır.

## Önce hesap kontrolü

Render hesabına erişim ve repo bağlantısı gerekir. Bağlı workspace'te **Free**
compute seçilebildiğini, kalan ücretsiz saat/build/bandwidth kotasını ve ödeme
durumunu kontrol et. Bu uygulama için ücretli plan, kart ekleme veya limit
artırma yapma. Ödeme yöntemi varsa bandwidth/build aşımı ücret doğurabilir;
yalnız `plan: free` yazılması toplam fatura garantisi değildir. Ücretsiz
kullanım ve ücret aşımını engelleyen hesap ayarı doğrulanamıyorsa servis yaratma.
Mevcut aynı adlı bir servisi değiştirme; yeni ve benzersiz bir staging adı seç.

Resmî kaynaklar (14 Kasım 2026 kontrolü):
[Free sınırları](https://render.com/docs/free),
[Web Service / bind ve PORT](https://render.com/docs/web-services),
[Blueprint alanları](https://render.com/docs/blueprint-spec),
[Rust toolchain](https://render.com/docs/rust-toolchain).

## Kurulum

Yeniden kurulum gerekirse hesap kontrolü sonrası `render.yaml` ile yeni servis kur veya aynı
değerleri Web Service formunda kullan:

- Runtime Rust, plan **Free**, Frankfurt, tek instance; disk/DB/autoscaling yok.
- Repo `kursadsaka/sudoku-rivals`, branch `agent/g16-render-free`.
- Build: `sh scripts/staging-build.sh`; start: `./dist/sudoku-server`.
- Health: `/health`; auto deploy **off**. Blueprint otomatik sync'ini de kapat.
- `RUSTUP_TOOLCHAIN=1.100.0`, `SUDOKU_BIND=0.0.0.0`,
  `SUDOKU_WEB_ROOT=./dist/web`. Render'ın `PORT` değerini kullan; hardcode etme.
- Yalnız Render'ın verdiği `https://<HOST>.onrender.com` origin'ini kullan.
  TLS Render'dadır; Axum container içinde HTTP dinler. Geniş CORS veya
  native cleartext istisnası ekleme.

Build script, Cargo.lock ile uyumlu wasm-bindgen CLI 0.2.130 ve WASM target'ını
hazırlar, kilitli Rust/WASM derler ve yalnız allowlist web dosyalarıyla binary'yi
`dist/` içine koyar. Build araçlarının kurulumu yalnız yerelde/Render build
ortamında olur. `dist/` git'e alınmaz. Yeni dependency eklenmedi.

## Ağ ve kota tercihi

Varsayılan hâlâ `127.0.0.1:3000`. `PORT`, eski `SUDOKU_PORT` ayarından önceliklidir;
`SUDOKU_BIND` yalnız gerçek IP adresi kabul eder. Port 0/geçersiz değer ve
eksik/okunamayan public web dosyası başlangıcı durdurur. Yanlış web root için
build checkout'una sessiz fallback yapılmaz.

Loopback dışı bind otomatik olarak **ortak staging kotası** kullanır: tüm
ziyaretçiler toplam 10 dakikada 5 create, 10 join denemesi paylaşır. Aynı 256-bit
guest, ad, mod/boyut ve lobi koltuğuna ait başarılı create/join tekrarları kotayı
tüketmez. Farklı kurulumlar ve normal giriş retleri kotayı tüketir; dolunca 429
döner. Misafir sırrını bilmeden tekrar istisnası kullanılamaz.
`X-Forwarded-For`, `Forwarded`, `X-Real-IP` güvenilmez ve kota anahtarı yapılmaz.
Proxy'nin loopback görünmesi yüksek yerel-test kotası sağlamaz. Bu küçük
staging için bilinçli bir sınırdır; ziyaretçi başına IP adaleti sağlamaz.
Gerçek per-IP kota için Render ingress başlık/erişim sözleşmesi ayrıca
doğrulanmalıdır. Böyle bir doğrulama yapılmadan başlık güveni açılmamalı.

`/r/ABC234` ve küçük harfli karşılığı aynı web girişini açar. Geçersiz alfabe,
query veya ek path reddedilir. Web paylaşımı zaten `location.origin` üzerinden
HTTPS URL üretir; kullanıcı onayı olmadan join veya origin değişimi yapmaz.
API/health/hata yanıtları `no-store`; statik varlıklar revalidation ve
`Referrer-Policy: no-referrer` kullanır. Token/guest URL'ye veya uygulama loguna
eklenmez. İstek yolu dosya yoluna çevrilmez; yalnız sabit allowlist okunur.

Statik varlıklar başlangıçta bir kez belleğe alınır; gövdeler her istekte
yeniden okunup kopyalanmaz. İçerik ETag'i ile eşleşen GET 304 ve boş gövde verir.
Public statik aktarım bütçesi süreç başına 128 MiB'dır; gövde boyutu ve istek
başına muhafazakâr 1 KiB başlık payı atomik rezerve edilir. 304 başlık payını
tüketir; HEAD muhafazakâr olarak gövde bedeliyle sayılır. Bütçe tükenince statik
yollar 503 verir; health/API ve mevcut maçlar etkilenmez. Bütçe restart ile
yenilenir. Bu aylık Render/billing limiti değildir: yeniden başlatmalar, küçük
hata yanıtları, API ve platform trafiği ayrıca sayılır. Statik bütçe de kötüye
kullanılarak yeni web yüklemeleri engellenebilir; test sonrası askıya alma
koşulu sürer.

## RAM durumu ve ücretsiz servis davranışı

Public süreçte kapalı/tutulan odalar dahil üst sınır 32'dir; yerelde 128 kalır.
Mevcut koltuklar ve makbuzlar kapasite için silinmez; dolu kapasitede doğrulanmış
aynı lobi kurulumu tekrar döner. 24 saatlik kapalı oda saklama nedeniyle yeni
oda girişi uzun süre sınırlanabilir; bu bilinçli bellek/erişilebilirlik tercihidir.
32 oda bir RSS garantisi değildir; en büyük makbuz birikimini azaltır.

Free servis 15 dakika trafiksizlikte uyuyabilir; soğuk açılış yaklaşık bir
dakika sürebilir. Native timeout'larında hata/retry görülmesi beklenir.
Restart/deploy/uyku RAM'deki odaları ve sonuçları kaybettirebilir; istemci sahte
sonuç üretmemelidir. Keep-alive servisi, ücretli disk veya DB ekleyerek bu
sınır aşılmadı. Tek replica çalıştır; deploy sırasında eski/yeni süreç
örtüşmesinde de durum paylaşılmaz. Deploy öncesinde staging maçlarını bitir;
yerleşik bir drain/kalıcı kurtarma yok. Free planın uyku davranışı kapatılamaz.

## Kontrol

Yerel, ücret ve dış servis kullanmayan doğrulama:

```sh
sh scripts/staging-build.sh
cargo test --locked --workspace
python3 tests/staging-package.py
```

Paket testi kendi dinamik loopback portunda çalışır ve sürecini kapatır.
`TMPDIR` ile test klasörü seçilebilir. Binary ve web paketini başka klasöre
taşır; gerçek WASM, solo, UI sözleşmesi ve race/duel × 6/9 HTTP maç matrisini
çalıştırır. Eksik WASM ile başlangıcın reddedildiğini de sınar. Public bind
kotası ayrı HTTP regresyon testinde loopback üzerinden doğrulanır.

Gerçek 0.0.0.0 bind'i yalnız izole ağ ad alanında sınanır:

```sh
unshare --user --map-root-user --net sh -c 'ip link set lo up && python3 tests/staging-package.py --public'
```

`--public`, `lo` dışında bir arayüz varsa çalışmayı reddeder; LAN portu açmak için
kullanılmaz. Statik bütçe yerel WASM istekleriyle tüketilir, ardından aynı
sunucuda dört gerçek istemci maç kombinasyonunun geçtiği doğrulanır.

Render servisi kurulduğunda önce plan/build SHA/tek-instance ayarını tekrar
doğrula, ardından HTTPS `/health`, `/`, `/r/ABC234`, JS/WASM MIME ve kaynak
dosyalarının 404'ünü kontrol et. HTTPS yanıtı giriş sayfası/bot challenge
olmamalı. İki tarayıcı bağlamıyla gerçek oda kur/katıl, hazır, sonuç, rövanş ve
ayrılma deneyi yap; secret taşıyan cevapları rapora/loga basma. Geçici hata ve
soğuk açılışta sonuç uydurulmadığını doğrula. Tam Node HTTP suite'i beş farklı
oda ve kayıp yanıt tekrarları oluşturur; yeni kodla boş public kota penceresine
sığar. Mevcut staging'de izinsiz çalıştırma; ortak kotayı tamamen tüketir.

## Android staging build

Android için ayrı `staging` build type vardır: `scripts/android-build.sh assembleStaging`
→ `apps/android/app/build/outputs/apk/staging/app-staging.apk`. Origin
`https://sudoku-rivals-g16-staging.onrender.com` olarak derlemeye sabittir;
`-PsudokuMatchServerUrl` ve `dev_server_url` intent'i bunu değiştiremez, uygulamada
sunucu alanı yoktur ve kayıtlı/gelen başka bir origin reddedilir (kayıt korunur,
üzerine yazılmaz). Release'ten türer: debug değildir, `src/debug` manifest ve
loopback `networkSecurityConfig` yoktur, cleartext kapalıdır, VIEW/autoVerify
filtresi yoktur. Application ID `com.sudokurivals.staging` olduğundan debug/release
kayıtlarına dokunmaz; ikisi yan yana kurulur. Debug hâlâ `10.0.2.2:3000` loopback,
release hâlâ boş `MATCH_SERVER_URL` kullanır.

APK yerel debug keystore ile imzalanır ve yalnız emülatör/test kurulumu içindir:
imzalı yayın, mağaza veya App Links sertifika parmak izi değildir. Render
ayarları/servisi değişmez. Doğrulama: `python3 tests/android-staging.py --aapt2 <aapt2>`
(statik) ve `--adb <adb> --serial <emulator> --live` (emülatör; tek create + leave
gönderir, ortak 5 create/10 dk kotasından birini kullanır; Free servis soğuksa
~1 dk sürebilir).

Universal/App Links için gerçek
imzalı app identifier ve APK sertifika parmak izi gerekir; bu değişiklikler uydurma
association dosyası yayımlamaz. İki `.well-known` yolu halen 404; OS link doğrulaması
tamamlandı sayılmaz. Web daveti kurulum olmadan çalışır. Native otomatik açılış ayrı
G16 devam işidir; Android staging build'i HTTPS VIEW filtresi eklemez.

## iOS staging yapılandırması

`apps/ios/project.yml` içinde Release'ten türeyen `Staging` build configuration ve
`SudokuRivalsStaging` şeması vardır (`xcodebuild -scheme SudokuRivalsStaging ...`).
Derleme koşulu `STAGING`: `LiveSession.releaseOrigin` sabit
`https://sudoku-rivals-g16-staging.onrender.com` olur ve origin sabitlenir; başka
origin'li kayıtlı oturum yüklenmez (`protectedSave`, kayıt bayt bayt korunur). Uygulamada
sunucu alanı yoktur; kurulum ekranı sabit host'u gösterir. Staging DEBUG ile derlenemez
(`#error`), loopback HTTP ve test kancaları yoktur. Info.plist'te hiçbir
`NSAppTransportSecurity` anahtarı yoktur (ATS varsayılanı: yalnız HTTPS). Bundle ID
yer tutucu `com.example.sudokurivals.staging`, ad "Sudoku Rivals Staging": ayrı veri
konteyneri, debug/release kayıtlarına dokunmaz. Debug (`LIVE_TEST_ORIGIN`/loopback) ve
Release (boş origin, "yapılandırılmadı") davranışı değişmedi. İmza yoktur; gerçek bundle
ID/Team sahibin kararıdır. `PrivacyInfo.xcprivacy` eklendi (izleme yok, system boot time
API'si 35F9.1). Statik kontrol: `python3 tests/ios-staging.py`. Ayrıntı:
[iOS staging kanıtı](validation/g16-ios-staging.md). Universal link/association hâlâ
yoktur.

## Geri alma

Yerel commit bağımsız dalda; entegrasyona cherry-pick/revert uygulanabilir.
Kurulacak staging'de yeni deploy'u geri almak RAM maçlarını kurtarmaz. Sorun
varsa yalnız yeni staging servisini durdur/sil; mevcut kaynak, domain ve
production uygulamalarına dokunma.
