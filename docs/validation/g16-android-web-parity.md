# G16 — Android'in web görsel dili ve akışlarıyla eşitlenmesi

17 Kasım 2026. Taban: `origin/agent/g16-render-free` (`120bd77`, web'in son hâli). Çalışma dalı `agent/g16-android-web-parity`, ayrı
worktree. Native Kotlin/Jetpack Compose korundu; Rust kuralları, UniFFI yüzeyi, protokol ve sunucu değişmedi (beş seviye, teknik ipucu ve
seviye okuma fonksiyonları çekirdekte zaten vardı). iOS karşılığı: PR #3 (`agent/g16-ios-web-parity`); oraya dokunulmadı, kararlar onunla
uyumlu tutuldu.

## Değişenler (yalnız `apps/android` ve Android testleri)

- **Görsel dil** (`Design.kt`): web'in kâğıt/mürekkep belirteçleri (açık/koyu) Compose'a taşındı: kanvas, kâğıt, mürekkep, soluk metin,
  çizgi, ızgara, marka mavisi, seçili/ilişkili/aynı rakam, hata, kehribar rakip. Kartlar, birincil/ikincil düğmeler, segmentli seçim,
  ilerleme çubuğu, web'in SVG yollarından çizilen çizgi simgeler, başlıkta marka işareti. Material bileşenleri (diyalog, metin alanı,
  anahtar) aynı belirteçleri kullanır. Pencere arka planı ilk karede kanvas rengidir (`Theme.SudokuRivals`, gece varyantı); durum/gezinme
  çubuğu simgeleri seçilen temaya uyar (edge-to-edge).
- **Ana sayfa**: üst başlık / başlık / giriş metni, mavi "Tek başına oyna" kartı, arkadaş kartı (sen mavi, rakip kehribar), süren maça dönüş
  kartı, boyut rozetli kayıt kartları (seviye · süre; `solo-resume-6/9`), açılamayan kayıt için hata kartı + tekrar dene, sağ üstte Ayarlar.
- **Solo kurulum**: boyut kartları (**varsayılan 9×9**, son seçim hatırlanır), **beş seviye** segmentli seçim (Başlangıç…Uzman; varsayılan
  Kolay, hatırlanır). Bulmaca `new_rated_solo` ile ana iş parçacığı dışında üretilir; düğme "Bulmaca hazırlanıyor…" der, seviyeye ulaşamayan
  tohum web'deki gibi 3 kez denenir. Kayıt biçimi değişmedi; seviye `solo_difficulty` ile verilen sayılardan okunur, seviyesiz eski kayıtlarda
  görünmez.
- **Oyun ekranı** (solo/yarış/düello; tek ekran yerleşim algoritması aynen korunarak): kare geri/duraklat düğmeleri, "Solo · 9×9 · Kolay" +
  büyük süre, doluluk çubuğu, sabit yükseklikte durum satırı, seçili hücre mavi dolgu, aynı rakam ve satır/sütun/kutu vurgusu, çakışma
  zemini, notlar 3 sütunlu ızgarada, her tuşta kalan adet (biten rakam geri çekilir), not modunda kesik çizgili tuşlar, simgeli araçlar
  (kalem açıkken dolu). Geçici uyarılar başlığın üstünde süzülür, tahtayı oynatmaz.
- **İpucu**: `solo_technique_hint`. Tekli yetmediğinde önce eleme adımları (kilitli aday, çıplak/gizli çift), sonra yerleştirme cümlesi.
  Desen hücreleri tahtada işaretlenir. Kart tuşların üstüne biner (dokunuşlar alttaki tuşlara geçmez), tahta ve tuşlar yerinden oynamaz.
  Kayıtlı oyun açılınca doğrudan tahtaya döner (web gibi); duraklatınca tahta yine gizlenir.
- **Duraklat / sonuç**: ortalanmış simge, başlık, "9×9 · Kolay | süre" kutusu; sonuçta "Yeni oyun" kuruluma götürür.
- **Arkadaş modu**: Oda kur / Katıl sekmeleri (davet bağlantısı Katıl'da, kodu dolu açılır; yeni ziyaret Oda kur'da başlar), mod kartları,
  boyut (**varsayılan 9×9**), **beş seviye** + açıklama. Oda kurulurken `difficulty` gönderilir. Mod, boyut, seviye ve elle yazılan görünen ad
  hatırlanır (dokunulmamış varsayılan "Misafir/Guest" saklanmaz, dili izlemeye devam eder). Lobide seviye satırı, harf kutulu oda kodu,
  kopyala/paylaş, avatarlı oyuncu satırları ve hazır rozetleri. Oyunda aynalı gösterge: yarışta iki doluluk yüzdesi/çubuğu, düelloda
  puan/seri ve kalan süre çubuğu (son 10 sn kırmızı). Sonuçta sonuca göre simge, düelloda skor satırı.
- **Ayarlar** (Android'e uygun): Tema (Sistem/Açık/Koyu), Ses (varsayılan **kapalı**), Titreşim (titreşimli cihazda; varsayılan **açık**, Android
  klavyesi gibi; sistemin "Dokunma geri bildirimi" ayarı yine geçerli). Dil uygulama içinde değil, Android 13+ uygulamaya özel dil ayarında
  (satır o sayfayı açar; daha eskide telefon dili izlenir). Hareket sistemin "Animasyonları kaldır" ayarını izler. Yerel kayıt listesi.
- **Ses/haptik** (`Feedback.kt`): `View.performHapticFeedback` (tuş, seçim, hata=REJECT, başarı=CONFIRM) ve kodla üretilen kısa sinüs tonları
  (`AudioTrack`, `USAGE_GAME`/`SONIFICATION`, dosya yok; zil sessiz veya titreşimdeyken çalmaz, ses odağı almaz). Tetikleyiciler web'deki gibi:
  giriş, not, hata, bitiş, ipucu, maç başlangıcı, rakip geldi, düelloda yanlış hamle, maç sonucu.
- **Uygulama simgesi** (kullanıcı onayıyla): başlatıcı simgesi web'in yeni simgesine eşitlendi (mavi zemin, beyaz kart, mavi kutu çizgileri,
  kehribar orta hücre, arkada eğik kehribar kart). Uyarlanabilir ön plan + Android 13 tek renk katmanı + Play 512 px RGBA PNG ana SVG'den
  mekanik olarak üretildi. Mağazaya yükleme yapılmadı. Ayrıntı: `apps/android/branding/README.md`.
- Korunanlar: Rust'a giden komutlar/kayıt biçimleri, canlı istemci adaptörü (taşıma, kalıcılık, kurtarma), hata metinleri, ekran okuyucu hücre
  etiketleri, tüm mevcut `testTag`'ler (yenileri: `solo-resume-6/9`, `difficulty-*`, `tab-create/join`, `theme-*`, `sound-toggle`,
  `haptics-toggle`, `hint-card`, `hint-apply`, `hint-dismiss`, `replace-confirm`, `game-notice`, `room-difficulty`, `live-resume`).

## Test düzenekleri

Yeni akışa göre güncellendi: `android-solo.py` (boyuta özel devam kartı, doğrudan tahtaya dönüş + duraklatınca gizlilik, ana sayfadaki bozuk
kayıt kartı, ipucu kartı etiketiyle uygulama, arka planda üretim için bekleme), `android-game-layout.py` (devam kartı), `android-locale.py`
(yeni metin anahtarları), `android-multiplayer.py` (oyun başlığında isteğe bağlı seviye eki), `test_android_i18n.py` (bilerek aynı kalan iki
anahtar), `test_android_icon.py` (yeni simgenin renkleri, `L` yol komutu, ortalamayı ön karttan ölçme).

## Doğrulama

ASUS (Linux), bu çalışmaya özel API 36 `google_apis_ps16k` x86_64 emülatörü (16 KB sayfa), son APK ile:

- `scripts/android-build.sh assembleDebug`: Rust FFI (arm64-v8a, x86_64) + Kotlin derlemesi geçti, Kotlin hatası yok.
- `tests/android-solo.py`: geçti (çevrimdışı 6×6/9×9 yeni oyun, döndürme + gece modu, verilen hücre koruması, not/sil/geri al/yinele,
  başarısız kayıt + tekrar dene, soğuk açılış, duraklatınca tahta gizliliği, ayrı boyut kayıtları, bozuk kaydın korunması, teknik ipucu ve
  bitiş).
- `tests/android-game-layout.py`: 12 ölçüm geçti (6×6 ve 9×9; 411×914, 411×731, 360×640, 320×568 dp ve yatay 731×411 dp; debug_ui
  açık/kapalı). Kaydırmasız olması gereken her boyutta tahta tek ekrana sığdı; 320×568 dp'de beklendiği gibi sayfa kayıyor.
- `tests/android-locale.py`: 147 metin/erişilebilirlik beklentisi geçti (tr-TR, en-US, desteklenmeyen de-DE → İngilizce).
- `tests/android-multiplayer.py --layout-check`: geçti. Android ↔ WASM (web istemcisi) yarış/düello × 6×6/9×9: maç, karşılıklı rövanş,
  ayrılma, davet bağlantısı, kaybolan yanıtın aynı gövdeyle tekrarı, 40 çevrimdışı düzenleme/tek eşitleme, uzlaştırma, soğuk yeniden
  başlatma, sunucu yeniden başlaması, 90 sn süre, kilit ve geri sayım metni; 4 dil etiketi + 20 yerleşim ölçümü (5 ekran boyutu × 4 maç).
- `tests/android-g14-lifecycle.py`: geçti (yavaş oda kurma, Home/yeniden açma kurulum tekrarı, kayıp ayrılma + arka plan/süreç ölümü,
  doğrulanmış yarış/düello sonucunun süre dolumu ve soğuk açılışta korunması).
- Statik: `test_android_i18n`, `test_android_icon`, `test_android_layout_check`, `test_android_multiplayer_fixture`,
  `test_android_native_alignment` (26 test) geçti. `cargo test --locked --workspace` (önce `scripts/web-build.sh` ile WASM üretilerek),
  `node tests/wasm.mjs`, `tests/web-solo.mjs`, `tests/ui-contract.mjs` geçti; Rust ve web değişmedi.
- Koşu sırasında bulunup düzeltilenler: 320×568 ve 360×640 dp'de maç başlığının tahtayı sığdırmaması (700 dp altında durum satırı skor
  satırına katılır, yarış çubukları gizlenir; 600 dp altında aralıklar daralır ve araçlar 48 dp olur, web'in kısa ekran kuralı gibi), yeni
  oturum açıldıktan sonra eski davetin Katıl sekmesini geri getirmesi. Bir koşuda `uiautomator dump` aracı emülatörde çöktü (çıkış 139);
  uygulama hatası değil, aynı APK ile tekrar koşu baştan sona geçti.
- Açık temada ve koyu temada (sistem gece modu) ana sayfa, kurulum, oyun, lobi ve arkadaş kurulumu ekran görüntüleriyle gözle kontrol
  edildi (yalnız çalışma alanında; depoya eklenmedi).
- Koşulmayanlar: `tests/android-staging.py` (Render staging askıda, HTTPS staging sunucusu gerekiyor), İngilizce çok oyunculu matris
  (`--locale en-US`), gerçek cihaz.

## Sınırlar

- Gerçek cihaz kullanılmadı; dokunma hissi, ses tonları ve titreşim insan kulağı/eliyle denenmedi.
- Mağaza ekran görüntüsü hazırlanmadı (kullanıcı tasarımı onaylayana kadar bekliyor); doğrulama görüntüleri yalnız çalışma alanında, depoda değil.
- Web'in uzun telefonlardaki 3×3 hesap makinesi tuş düzeni alınmadı: tek ekran düzen testi 9×9 için 5+4 tuş sırası bekliyor (iOS'ta da 5+4).
- Uygulama içi dil seçici yok (Android'in uygulamaya özel dil ayarı kullanılıyor); Android 12 ve altında dil telefonu izler.
- Davet bağlantısıyla uygulamanın kendiliğinden açılması (App Links) hâlâ yok; kod veya bağlantı Katıl sekmesine yapıştırılır.
