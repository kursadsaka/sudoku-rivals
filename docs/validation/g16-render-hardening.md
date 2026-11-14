# G16 — Fable bulguları sonrası yerel düzeltmeler

15 Kasım 2026. Taban: `agent/g16-render-free` / `31ecc78`; GitHub head'i çalışma
öncesinde doğrulandı. Bu tur yalnız kod ve testtir: kullanıcının bildirdiği
kartsız, testten sonra askıya alınmış Render Free servisi ve auto-deploy-off
ayarı değiştirilmedi. Yeni deployment, ajan, Notes yazısı veya main merge yok.

İnceleme kaynağı: [Fable bağımsız incelemesi](t3-thread://v2/0e2952d4-664e-4d95-9b5f-7ad0803aaea5/thread:delegated-task:command%3Amcp%3A6bf54baa-56df-4591-8123-ad3f52b199c3%3Adelegate-task%3A780b2243-f457-4412-99c8-b9835868ee24).

## Değişiklikler

- Public ingress'te kapalı/tutulan odalar dahil en fazla **32 oda**. Yerel
  sınır 128 ve koltuk başına makbuz sınırı korunur; mevcut oyun/veri/sonuç
  kapasite açmak için atılmaz. Sınırda aynı doğrulanmış lobi kurulumu tekrar
  dönebilir. Bu kontrol HTTP'nin kullandığı sabit public ingress anahtarında
  yapılır; istemci başlıkları bu anahtarı seçemez.
- Statik allowlist başlangıçta bir kere yüklenir. Paylaşılan Bytes gövdeleri
  kopyalanmaz; ETag içeriğe bağlıdır. `If-None-Match` eşleşmesinde **304, boş
  gövde** döner. Route/path/MIME ve API gizlilik davranışı korunur.
- Public statik gövde + muhafazakâr başlık bütçesi **128 MiB/süreç**. Eşzamanlı
  rezervasyon atomiktir; yetersiz bütçede statik 503 döner. Health/API bunun
  dışında kalır. 304 de 1 KiB başlık payı tüketir. Yerel geliştirme kotası bu
  public bütçeyle sınırlanmaz.
- Kaybolan create/join yanıtı aynı 256-bit guest ve aynı lobi kurulumu ile
  tekrarlandığında kota ve kapasite denetiminden önce mevcut koltuk döner.
  Farklı istek veya yeni guest tekrar istisnasını kullanamaz. Yeni kurulumların
  ortak 5 create/10 join/10 dakika kotası sürer. API DTO veya istemci protokolü
  değişmedi; native istemcilerin süre/transport kodlarına dokunulmadı.

## Doğrulama

- `cargo test --locked --offline --workspace`: **89 geçti, 0 başarısız**.
  Yeni testler: public oda sınırı ve kapasitede replay/koltuk korunması;
  20 create/join tekrarı ve dolu kotada replay; ETag'in güçlü/zayıf eşleşmesiyle
  boş 304; bütçe dolunca 503/health korunması; eşzamanlı bütçe aşımının reddi.
- `sh scripts/staging-build.sh`: final release/WASM paketi yerelde üretildi,
  yeni dependency yok. Rust 1.100.0 derlemesi uyarısız tamamlandı.
- Paket, başka klasör/CWD'ye taşınarak **gerçek 0.0.0.0 bind** ile denendi:
  `unshare --user --map-root-user --net`, yalnız lo arayüzü. LAN veya internet
  portu açılmadı. `tests/staging-package.py --public` başka arayüz varsa reddeder.
- WASM MIME, gerçek ETag 304 ve farklı tag'de 200; kaynak/dotfile 404'leri;
  public statik bütçenin tükenmesi ve sonrasında health 200 doğrulandı.
- Aynı bütçesi tükenmiş sunucuda gerçek WASM adapteriyle **race/duel × 6/9**
  hazır/geri sayım/sonuç/rövanş/resume/leave matrisi geçti. Kayıp create/join
  yanıtında soğuk restore aynı oda/koltuğu korudu. Beş farklı oda tamamlandı,
  altıncı create sahte forwarded başlığına rağmen 429 aldı: main'in public
  bind→public kota bağlantısı kalıcı uçtan uca testte doğrulandı.
- WASM fixture/solo/üretim/sınır testleri, web solo, UI contract ve mevcut web
  terminal kayıp/pending setup/sonuç/cold-leave regresyonları geçti. Eksik
  paket WASM'ında başlangıç reddedildi. Test kendi sürecini kapattı.

Çalışma alanındaki ham loglar: `g16-hardening-evidence/cargo-test.log`,
`staging-build.log`, `public-package.log`. `render.yaml` ve entegrasyon dalı
bu turda değiştirilmedi. Bu sonuçlar gerçek askıdaki Render servisine yeni
kodun deploy edildiğini kanıtlamaz; final gönderim incelemesi Fable'a ayrıldı.

## Kalan sınırlamalar

32 oda RSS için matematiksel üst sınır değildir; makbuz ve RAM-only çalışma
riski azalır. Kapalı odalar 24 saat tutulduğu için 32 oda dolarsa yeni giriş
uzun süre kapanabilir. Ortak kotanın tek ziyaretçi tarafından tüketilmesi ve
kimliksiz API isteklerinin yük oluşturması tamamen çözülmedi. Gerçek IP
başlıkları doğrulanmadan bunlara güven açılmadı.

128 MiB bütçe aylık Render trafik/fatura tavanı değildir: restart ile sıfırlanır,
API/küçük hata/platform trafiğini sınırlamaz. HEAD muhafazakâr gövde bedeliyle
sayılır; bütçe de erişilebilirlik açısından tüketilebilir. Bu nedenle gözetimli
test ve test sonrası askıya alma koşulu sürer. Sıkıştırma veya keep-alive servisi
eklenmedi. Bu tur fiziksel cihaz veya gerçek internet gecikmesi denemesi yapmadı.

## Native davetlerde ayrı kalan işler

1. Mevcut Render HTTPS origin'ini native staging build ayarlarına bağlamak:
   iOS releaseOrigin halen boş; Android debug başlangıcı hâlâ emülatör
   loopback adresini seçiyor. URL'den origin değiştirmek yerine build allowlist'i
   kullanılmalı; yerel kayıtlar ve aktif maç korunmalı.
2. İmzalı iOS `application-identifier` prefix/bundle ID ile Associated Domains
   entitlement, `project.yml` bağlantısı ve idempotent `.onOpenURL`/user-activity
   alıcısı. Aktif maçı otomatik terk etmeden daveti kullanıcıya sunmalı.
3. Android HTTPS VIEW/BROWSABLE `autoVerify` intent-filter ve aynı host
   doğrulaması; staging APK'sının gerçek imza sertifikası SHA-256'sı. Mevcut
   onCreate/onNewIntent ve kayıt koruma davranışı korunmalı.
4. Bu gerçek kimliklerle iki sabit HTTPS association route'u: redirect/auth
   olmadan JSON 200 AASA/assetlinks; şu an 404. Sonra Apple CDN ve Android
   verification, simülatör/emülatörde cold/warm açılış, yanlış host/path,
   mevcut oturum ve web↔native eşleşmesi. Gerçek telefon testi kullanıcı
   kararıyla kabul koşulu değildir. Domain/DNS değişikliği gerekmiyor.
