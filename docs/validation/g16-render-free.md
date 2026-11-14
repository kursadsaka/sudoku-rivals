# G16 Render Free — inceleme paketi

15 Kasım 2026, ASUS. Başlangıç `agent/g14-integration` uzak/yerel head'i
`c6930b0d8b4ec6fd74a7aa4d4e5e493041f25b6f`; ayrı worktree/dal
`agent/g16-render-free`. Entegrasyon çalışma ağacına dokunulmadı.

## Değişiklik

Render'ın PORT ve açık 0.0.0.0 bind ihtiyacını, yalnız public allowlist web
paketini ve proxy arkasındaki kota davranışını hazırlar. Varsayılan yerel
loopback korunur; hatalı port veya eksik web varlığı başlangıcı durdurur.
Forwarded başlıkları güvenilmez; public bind'de 5 create/10 join denemesi/10
dakika tüm ziyaretçilere ortak uygulanır. Bu kota bilinçli sınırlı staging
çözümüdür, per-IP fairness veya production DDoS önlemi değildir.

Yeni Render YAML yalnız bir Free Web Service ister; otomatik deploy, disk,
DB, özel domain ve autoscaling yok. Pinli Rust/wasm-bindgen build'i public
dosyaları `dist/` içine paketler. Native uygulamalar/core/protokol DTO'ları
değişmedi. OS link association dosyaları gerçek kimlikler olmadan eklenmedi.

## Geçen kontroller

- `cargo check --locked --offline --workspace`.
- `cargo test --locked --offline --workspace`: **85 passed, 0 failed**;
  main port/bind regresyonu, 5 gerçek HTTP testi ve mevcut çekirdek/istemci
  testleri dahil. Yeni HTTP testleri sahte forwarded başlıklarıyla public
  kota aşımının reddini ve açık web root için fallback olmadığını doğrular.
- `sh scripts/staging-build.sh`: yerelde offline dependency cache'i ile
  gerçek release binary ve yeniden üretilmiş WASM paketi oluşturuldu.
- `python3 tests/staging-package.py`: `dist/` geçici başka klasöre taşındı;
  farklı CWD ve işaretlenmiş HTML ile seçilen root doğrulandı. PORT önceliği,
  health/no-store/referrer policy, WASM MIME, kaynak yolu 404'leri geçti.
  Gerçek WASM/solo/UI sözleşmesi testleri ve HTTP race/duel × 6/9 matrisi,
  kayıp create/join cevabında cold retry, rövanş/resume/leave geçti.
  Paket WASM'ı silinince başlangıç reddedildi. Yalnız testin kendi dinamik
  loopback sunucusu açıldı ve kapatıldı.
- YAML yerelde parse edildi: bir servis, `plan: free`, `numInstances: 1`,
  `autoDeployTrigger: off`, disk/domain/scaling/previews yok. Alanlar Render'ın
  resmî Blueprint belgesiyle karşılaştırıldı; hesap gerektiren Render API
  Blueprint validation yapılmadı.
- `git diff --check` temiz. Testte sistem `/tmp` kullanıcı kotası doldu;
  yalnız kendi workspace'indeki `TMPDIR` ile tekrar başarıyla tamamlandı.

## İnceleyenin odaklanacağı yerler

1. `server/src/main.rs`: PORT önceliği, loopback varsayılanı, public bind'in
   otomatik ortak kota seçmesi ve başlangıçta varlık doğrulaması.
2. `server/src/http.rs`: allowlist dosya seçimi, yalnız yapılandırılmış root,
   forwarded başlığına dayanmayan public kota, API cache ve davet doğrulaması.
3. `scripts/staging-build.sh`, `render.yaml`: aynı sürüm WASM/CLI, yalnız Free
   tek-instance kaynak ve manuel deploy. `tests/staging-package.py` taşınmış
   paketi gerçek HTTP ile sınar; source checkout fallback'i başarı sayılmaz.

Bu paket bağımsız ajan incelemesi almadı; kullanıcı başka ajan açılmamasını
istedi. İncelemeye hazır diff: `git diff c6930b0..agent/g16-render-free`.

## Dış kurulumun gerçek engeli ve sınırlar

Render eklentisi katalogda bulundu fakat kurulmuş/bağlı değil. Çalışma
ortamında Render CLI veya Render credential ortam değişkeni bulunmadı;
bilinen CLI config yolları da yok. Secret içerikleri aranmadı veya okunmadı.
Eklenti bağlantısı önerildi; hesap erişimi doğrulanmadı. Dolayısıyla workspace,
ücretsiz kalan kota ve ödeme/aşım durumu doğrulanamadı; servis oluşturulmadı,
deploy URL'si yok. Kart veya ücretli kaynak talep edilmedi.

Hesap bağlandığında mevcut YAML ile yalnız yeni bir Free staging servisi
oluşturulabilir; hesap kontrolü ve HTTPS smoke testi `docs/staging-render.md`
içinde. Render Free uyku/soğuk başlangıç ve RAM-only restart kayıpları sürer;
native Universal/App Links ve gerçek internet çapraz platform maçı henüz
doğrulanmadı. Production/domain/DNS işlemi yapılmadı. Notes'a yazılmadı.
