# G16 — iOS'un web görsel dili ve akışlarıyla eşitlenmesi

17 Kasım 2026. Taban: `origin/agent/g16-render-free` (`120bd77`, web'in son hâli) + `origin/agent/g16-ios-store-prep`
(`edd55b2`, iOS tek ekran oyun yerleşimi ve uygulama simgesi; daha önce bu dala alınmamıştı). Çalışma dalı
`agent/g16-ios-web-parity`, ayrı worktree. Native SwiftUI korundu; Rust kuralları, protokol ve sunucu değişmedi.

## Değişenler (yalnız `apps/ios`)

- **Görsel dil** (`Design.swift`): web'in kâğıt/mürekkep belirteçleri (açık/koyu): kanvas, kâğıt, mürekkep, soluk metin, çizgi,
  ızgara, marka mavisi, seçili/ilişkili/aynı rakam, hata, kehribar rakip. Başlık, rakam ve süreler yuvarlatılmış sistem
  fontuyla. Kartlar, birincil/ikincil düğmeler ve basınca hafif küçülme (Hareketi Azalt açıkken kapalı). Başlıkta web'deki
  simgenin SwiftUI çizimiyle "Sudoku **Rivals**" yazısı.
- **Ana sayfa**: üst başlık / başlık / giriş metni, mavi "Tek başına oyna" kartı, arkadaş kartı (sen mavi, rakip kehribar),
  süren maça dönüş, boyut rozetli kayıt kartları (seviye · süre), sağ üstte Ayarlar.
- **Solo kurulum**: tahta boyutu kartları (**varsayılan 9×9**, son seçim hatırlanır), **beş gerçek zorluk** (Başlangıç, Kolay,
  Orta, Zor, Uzman; varsayılan Kolay, hatırlanır). Bulmaca ortak çekirdeğin `new_rated_solo` fonksiyonuyla ana iş parçacığı
  dışında üretilir. Üretim sürerken düğme "Bulmaca hazırlanıyor…" der. Seviyeye ulaşamayan tohum web'deki gibi 3 kez denenir.
  Kayıt biçimi değişmedi; seviye, verilen sayılardan `solo_difficulty` ile okunur. Eski, seviyesiz kayıtlarda seviye görünmez.
- **Oyun ekranı** (solo/yarış/düello, tek ekran yerleşimi korunarak): "SOLO · 9×9 · KOLAY" + büyük süre, doluluk çubuğu,
  seçili hücre mavi dolgu, aynı rakam ve satır/sütun/kutu vurgusu, çakışma zemini, her tuşta kalan adet (biten rakam geri
  çekilir), not modunda kesik çizgili tuşlar, sade araç düğmeleri (kalem açıkken dolu).
- **İpucu**: `solo_technique_hint`. Tekli yetmediğinde önce eleme adımları (kilitli aday, çıplak/gizli çift), sonra yerleştirme
  cümlesi gösterilir. Desen hücreleri tahtada işaretlenir. Kart tuşların üstüne biner, tahta ve tuşlar yerinden oynamaz.
  Geçici uyarılar başlığın üstünde süzülür.
- **Duraklat / sonuç**: ortalanmış simge, başlık, "9×9 · Kolay | süre" kutusu.
- **Arkadaş modu**: Oda kur / Katıl sekmeleri, mod kartları, boyut (**varsayılan 9×9**), **beş seviye** + açıklama
  ("iki oyuncu aynı bulmacayı alır…"). Oda kurulurken `difficulty` gönderilir. Seçimler ve görünen ad hatırlanır. Lobide
  seviye satırı, harf kutulu oda kodu, kopyala/paylaş, avatarlı oyuncu satırları, hazır rozetleri var. Oyunda rakip aynalı
  gösterge var: yarışta doluluk yüzdeleri, düelloda puan/seri ve kalan süre çubuğu (son 10 sn kırmızı). Sonuç ekranında
  sonuca göre simge, düelloda skor satırı.
- **Ayarlar** (iOS'a uygun): Tema (Sistem/Açık/Koyu), Ses (varsayılan kapalı), Titreşim (iPhone'da, varsayılan **açık**: iOS
  alışkanlığı; sistemin titreşim ayarı yine geçerli). Dil uygulama içinde değil, iOS Ayarlar'ındaki uygulamaya özel dil
  seçimiyle değişir (`en.lproj`/`tr.lproj` bu seçimi açar). Hareket, iOS'un Hareketi Azalt ayarını izler. Yerel kayıt listesi.
- **Ses/haptik**: Taptic Engine (hafif dokunuş, seçim, hata/başarı bildirimi) ve kodla üretilen kısa sinüs tonları
  (`AVAudioSession` `.ambient`: sessiz anahtarına uyar, başka sesi kesmez; dosya yok). Tetikleyiciler web'deki gibidir:
  giriş, not, hata, bitiş, ipucu, maç başlangıcı, rakip geldi, düelloda yanlış hamle, maç sonucu.
- Korunan şeyler: tüm `accessibilityIdentifier`'lar korundu, yenileri `size-6/9`, `difficulty-*`, `mode-race/duel`,
  `live-tabs`, `settings-open`. TR/EN metin tabloları birlikte güncellendi. Canlı oturum varsayılanları (`LiveSession`)
  testler için değişmedi; 9×9/Kolay uygulama açılırken tercihlerden atanır.

## Testler

- **Linux (ASUS, bu çalışmada)**: `cargo test --locked --workspace`: tüm paketler geçti (Rust değişmedi). İlk koşuda iki
  sunucu testi ortam yüzünden düştü: worktree'de üretilmiş `apps/web/pkg` yoktu ve `/tmp` kotası dolu (EDQUOT). WASM
  `scripts/web-build.sh` ile üretilip `TMPDIR` ev diskine alınınca hepsi geçti. `node tests/wasm.mjs`, `tests/web-solo.mjs`,
  `tests/ui-contract.mjs` geçti.
- Yeni iOS testlerinin beklentileri aynı çekirdek üzerinde WASM ile ayrıca denendi: 6×6 ve 9×9'da beş seviye istenen seviyede
  derecelendi, 9×9 Zor/Uzman (tohum 7) teknik ipuçlarıyla eleme adımı içerek bitti.
- Swift derleyicisi Linux'ta yok. Swift dosyaları elle gözden geçirildi. Ayraç dengesi, TR/EN anahtar eşliği, kopya anahtar
  (çalışma anında çökme) ve kullanılan her `L10n` anahtarının varlığı bir betikle denetlendi.
- **Yapılmadı, Mac gerekiyor**: `scripts/ios-build.sh` (XCFramework + XcodeGen + derleme), `SudokuRivalsTests` (yeni
  `WebParityTests` dahil), `SoloUITests`, `python3 tests/ios-multiplayer.py`, `tests/ios-staging.py`. Güncellenen UI
  testleri: boyut kartları, mod kartları, Katıl sekmesi, puzzle başına değişen boş hücre sayısı.

## Sınırlar

- iOS derlemesi ve simülatör/cihaz çalıştırması doğrulanmadı. İlk Mac derlemesinde küçük derleme düzeltmeleri gerekebilir.
- Ekran görüntüsü alınmadı; mağaza görselleri kullanıcı tasarımı onaylayana kadar bekliyor. Mağaza veya dağıtım işlemi yapılmadı.
- Sesler insan kulağıyla, haptikler cihazda denenmedi. Web'in uzun telefonlardaki 3×3 hesap makinesi tuş düzeni iOS'a
  alınmadı (iOS'ta 5+4 / 6 veya 3×2 kaldı).
- Davet bağlantısıyla uygulama açılışı (universal link) hâlâ yok; kod veya bağlantı Katıl sekmesine yapıştırılır.

## 18 Kasım takibi

- Uzun iPhone ekranlarında 9×9 için üç sütunlu tuş takımı, kısa ekranlarda 48 pt tuşlar ve daha dar aralıklar kullanılır. 6×6'da tuşlar iki sıradır. Büyük erişilebilirlik yazılarında seviye seçimi ve oyun araçları dikey dizilir. Tahta ile kontroller birlikte sığmazsa ekran kaydırılabilir.
- İpucu açıkken sayı tuşları ve araçlar VoiceOver odağından çıkar ve klavye kısayolları devre dışı kalır. Canlı oyun, tahta gerçekten göründüğünde ilk boş hücreyi seçer. Solo kurulumdan geri dönülürse bekleyen üretim kayıt yazmaz.
- ASUS'ta WASM derlemesi, Rust çalışma alanı testleri, WASM/solo/UI sözleşmesi ve yerel HTTP üzerinden yarış/düello × 6×6/9×9 geçti. UniFFI Swift bağları Linux'ta üretildi ve Swift kaynakları sözdizimi ayrıştırıcısından geçti. Xcode derlemesi, yeni UI testleri, simülatör ve fiziksel cihaz doğrulaması bekliyor.
