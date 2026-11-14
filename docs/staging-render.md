# G16 — Render Free staging

Bu yapılandırma yalnız ayrı, yeni bir staging Web Service içindir. Production,
DNS, ücretli disk/veritabanı, mağaza veya imza anahtarı gerektirmez. Kaynak dal
`agent/g16-render-free`; otomatik deploy kapalıdır. Gerçek servis URL'si henüz
oluşturulmadı; aşağıdaki `<HOST>` bir placeholder'dır.

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

Hesap kontrolü geçtikten sonra `render.yaml` ile yeni servis kur veya aynı
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
ziyaretçiler toplam 10 dakikada 5 create, 10 join denemesi paylaşır. Mevcut
protokolde tekrarlar da bu giriş kotasını tüketir; kota dolunca 429 döner.
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

## RAM durumu ve ücretsiz servis davranışı

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

Render servisi kurulduğunda önce plan/build SHA/tek-instance ayarını tekrar
doğrula, ardından HTTPS `/health`, `/`, `/r/ABC234`, JS/WASM MIME ve kaynak
dosyalarının 404'ünü kontrol et. HTTPS yanıtı giriş sayfası/bot challenge
olmamalı. İki tarayıcı bağlamıyla gerçek oda kur/katıl, hazır, sonuç, rövanş ve
ayrılma deneyi yap; secret taşıyan cevapları rapora/loga basma. Geçici hata ve
soğuk açılışta sonuç uydurulmadığını doğrula. Tam Node HTTP suite'i altı create
denemesi yapar; 5/10 ortak public kotasını aşacağından public origin'de körlemesine
çalıştırma. Dört mod/boyutu gerektiğinde kota pencerelerine bölerek doğrula.

iOS/Android API origin'i ancak gerçek HTTPS servis URL'si belli olduğunda
ayarlanabilir. Universal/App Links için gerçek imzalı app identifier ve APK
sertifika fingerprint'i gerekir; bu değişiklikler uydurma association dosyası
yayımlamaz. İki `.well-known` yolu halen 404; OS link doğrulaması tamamlandı
sayılmaz. Web daveti kurulum olmadan çalışır. Native otomatik açılış ayrı
G16 devam işidir; bu commit native uygulamaları değiştirmez.

## Geri alma

Yerel commit bağımsız dalda; entegrasyona cherry-pick/revert uygulanabilir.
Kurulacak staging'de yeni deploy'u geri almak RAM maçlarını kurtarmaz. Sorun
varsa yalnız yeni staging servisini durdur/sil; mevcut kaynak, domain ve
production uygulamalarına dokunma.
