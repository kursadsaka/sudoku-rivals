# Web kurulum simgeleri: 192/512 PNG, maskable ve apple-touch-icon

15 Kasım 2026. Taban `agent/g16-render-free` / `f5eb6f6`. Yalnız `apps/web` varlıkları, sunucunun statik allowlist'i, paketleme betiği, testler ve belge;
iOS ve Android uygulama dosyaları değişmedi.

## Kaynak ve üretim

Tek vektör kaynak `apps/web/icon.svg` (marka mavisi `#1d4ed8`, beyaz 3×3 ızgara, ortada dolu hücre; Android simgesiyle aynı çizim). Dosya değişmedi.
`scripts/web-icons.sh` (rsvg-convert + ImageMagick; PNG'ler repoya girdiği için derleme bunlara ihtiyaç duymaz) şunları üretir:

| Dosya | Boyut | Kullanım | Not |
| --- | --- | --- | --- |
| `icon-192.png`, `icon-512.png` | 192, 512 | manifest `any` | `icon.svg` olduğu gibi: yuvarlak köşeli kare, saydam köşeler (RGBA) |
| `icon-maskable-512.png` | 512 | manifest `maskable` | tam kanama (köşe yok), opak RGB; çizim 166/192 (%86,4) küçültülür: ızgara kenarı %54, içerik merkez %80'lik güvenli dairenin (yarıçap %40) içinde |
| `apple-touch-icon.png` | 180 | `<link rel="apple-touch-icon">` | tam kanama, opak RGB (iOS köşeyi kendisi yuvarlar) |

## Değişiklikler (en dar kapsam)

- `manifest.webmanifest`: `icons` = 192/512 PNG `any`, 512 PNG `maskable`, `icon.svg` `any`. Önceden `icon.svg` `any maskable` idi; yuvarlak/saydam köşeli dosya maskable için
  uygun değildi. Diğer alanlar (ad, `start_url`, `scope`, `display`, renkler) aynı.
- `index.html`: SVG favicon'a `sizes="any"`, PNG favicon yedeği (192) ve `apple-touch-icon` bağlantıları eklendi; başka değişiklik yok.
- `sw.js`: dört PNG önbelleğe alma listesine eklendi, önbellek `sudoku-rivals-app-v9` (eski sürüm etkinleşmede silinir). Fetch/navigasyon davranışı aynı.
- `server/src/http.rs`: `WEB_ASSETS` allowlist'ine dört `image/png` girdisi. Başka yol açılmadı (istek yolundan dosya yolu üretimi yok; `validate_assets` bunları da denetler).
- `scripts/staging-build.sh`: paket kopyalama listesine dört PNG.

## Doğrulama

- `python3 -m unittest tests/test_web_icons.py` (8 test, tarayıcısız; PNG'ler standart kütüphaneyle çözülür): boyutlar ve kanallar, "any" simgelerin saydam köşesi, maskable ve
  apple-touch'ın dört köşesinin tam marka mavisi olması, **maskable içeriğin %40 yarıçaplı güvenli dairede ve ortalı olması** (negatif kontrol: ölçeklenmemiş çizim 77,9 px ile
  72 px sınırını aşar), manifest simgelerinin var olması/türü/boyutunun eşleşmesi, `any maskable` olmaması, HTML bağlantıları, sunucu allowlist'i + `sw.js` + paketleme listesi
  tutarlılığı.
- `cargo test --locked -p sudoku-server static_tests`: 3 test geçti (yeni: simgeler allowlist'te `image/png`; her allowlist dosyası web kökünde var).
- `sh scripts/staging-build.sh && python3 tests/staging-package.py`: taşınmış release paketi geçti. Yeni denetim: manifestin her simgesi ve `apple-touch-icon.png` doğru `Content-Type`
  ve gerçek PNG boyutuyla sunuluyor; mevcut statik gizlilik, ETag/304, WASM/solo/UI sözleşmesi/web çok oyunculu testleri ve 4 mod×boyut HTTP maçı hâlâ geçiyor.
- `tests/web-install.mjs` (yeni; paketlenmiş sunucuyu loopback'te açar, Chromium CDP ile `Page.getAppManifest` + `Page.getInstallabilityErrors`): manifest hata/uyarısı yok, kurulabilirlik
  hatası yok, dört simge doğru türle sunuluyor, `<head>` bağlantıları beklenen. Not: güncel Chromium eski manifesti (yalnız SVG) de kurulabilir sayıyor; bu denetim manifestin
  geçerli ve kurulabilir olduğunu kanıtlar, eski/yeni farkını değil. PNG'lerin asıl katkısı raster isteyen yüzeyler (Android WebAPK, eski/diğer tarayıcılar, iOS ana ekranı).

## Sınırlar

- Gerçek bir "Ana ekrana ekle"/WebAPK kurulumu, iOS Safari ana ekran simgesi ve gerçek maskeli (daire/squircle) görünüm cihazda denenmedi; maskable güvenli bölge piksel ölçümüyle doğrulandı.
- Canlı Render servisine deploy edilmedi; bu dosyalar yalnızca bir sonraki paketlemeyle gider (SW önbelleği `v9`'a yükselir, tek seferlik yenilenme).
- Yeniden üretim `rsvg-convert` ve `magick` ister; çıktı bayt bayt aynı olmayabilir (testler piksel özelliklerini denetler).

## iOS için kaynak yolu (uygulanmadı)

Aynı vektör: `apps/web/icon.svg`; köşesiz tam kanama hâli `apps/android/branding/ic_launcher_master.svg` (web ikonundan `rx="42"` çıkarılmış hâl, Android testi eşitliği denetler).
Öneri: `rsvg-convert -w 1024 -h 1024 apps/android/branding/ic_launcher_master.svg | magick - -alpha off PNG24:AppIcon-1024.png` ile 1024×1024 **opak** PNG,
`apps/ios/SudokuRivals/Assets.xcassets/AppIcon.appiconset/` (tek boyutlu `Contents.json`, Xcode 14+) ve `project.yml`'da varlık kataloğunu kaynak yapıp
`ASSETCATALOG_COMPILER_APPICON_NAME: AppIcon` ayarı. Tüm platformlar için ana SVG'yi platform-nötr bir konuma (ör. `design/app-icon/`) taşımak ve `scripts/web-icons.sh` ile
Android `branding/README.md` komutlarını tek betikte birleştirmek önerilir.

## Güncelleme: yeni simge çizimi (15 Kasım, web)

`icon.svg` yeniden çizildi: aynı marka mavisi zemin, ortada beyaz 3×3 tahta (ızgara zemin renginde bindirme çubuklarla), arkasında sağ üste kaymış kehribar "rakip" tahtası ve ortada tek kehribar hücre. Tasarım sözleşmesi: [web-app-icon-design.json](web-app-icon-design.json) ([app-icon-skill](https://github.com/xzhih/app-icon-skill) `semantic-icon-design` biçimi, `blue-field-cutout-badge` yönü, 4 katman, maske/çizgi/gölge yok).

- Skill'in üretim ve doğrulama betikleri (`author-icon`, `verify`; `npm ci` ile dört bağımlılık kurulumu ister) **kurulmadı ve çalıştırılmadı**; vektör elle yazıldı. Ücretli servis gerekmedi. Skill deposunda lisans dosyası yok.
- PNG'ler aynı `scripts/web-icons.sh` ile yeniden üretildi; betik ve `tests/test_web_icons.py` değişmedi ve geçiyor (8 test). Maskable sürümde mavi olmayan en uzak piksel merkezden 201,3 px (güvenli yarıçap 204,8 px).
- 256, 64 ve 32 px'te gözle incelendi: tahta ve arkadaki kehribar tahta 32 px'te de ayırt ediliyor.
- Başlıktaki küçük marka işareti (`index.html`) aynı çizimi kullanır. **Android ve iOS simgeleri değişmedi**; platformlar arası simge şimdilik farklıdır.
