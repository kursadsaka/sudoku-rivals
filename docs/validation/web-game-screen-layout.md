# Web oyun ekranı: tek ekran yerleşimi ve debug bayrağı

Kapsam: yalnız `apps/web` (solo, klasik yarış, düello; 6×6 ve 9×9). iOS ve Android dosyalarına dokunulmadı; aşağıdaki "Native için öneri" bölümü sonraki iş içindir. Bu not `docs/ui-contract.md` içindeki dar ekran sırasını (gezinme → durum → tahta → sayılar → araçlar) korur; iki noktada ondan ayrılır: oyun ekranında uygulama başlığı/alt bilgi gösterilmez, durum satırı yalnız anlamlı bilgiyi gösterir.

## Ortak oyun ekranı yerleşimi (öneri)

Sıra, üç mod ve iki boyutta aynıdır:

1. **Gezinme satırı**: geri/ayrıl · `Mod · N×N` + süre · sağ slot (solo: duraklat; canlı: rakip adı).
2. **Durum satırı** (tek satır, sabit yükseklik): bağlantı noktası (sağlıklıyken yalnız nokta; sorun varsa metin), moda özgü değer (yarış: rakibin doluluğu · düello: puan/rakip/seri), çakışma uyarısı, düello giriş kilidi.
3. **Tahta**: kalan yüksekliğe sığan en büyük kare.
4. **Sayı tuşları**: 6×6 tek sıra (≤339 px genişlikte 3×2) · 9×9 `5+4` (ikinci sıra ortalı). Hedef ≥48.
5. **Araçlar**: solo 5 (kalem, sil, geri al, yinele, ipucu) · yarış 4 · düello yok.
6. **Geçici iletiler** (yalnız gerektiğinde): kayıt hatası, uzlaştırma, ipucu açıklaması, yanlış hamle/ hız sınırı.

Ekranda **yok** (varsayılan): uygulama adı/ayarlar başlığı, çevrimdışı/"6×6 & 9×9" alt bilgisi, "oyun devam ediyor" ve "kaydedildi" metinleri, seçili hücre satırı ("Satır 1 · Sütun 2"), klavye ipucu (yalnız masaüstü işaretçi + ≥700 px'te görünür), düello "not yok" açıklaması. Bunların hepsi ekran okuyucuda korunur: durum ve kayıt metinleri `sr-only`, klavye yardımı ızgaranın `aria-describedby` açıklaması, hücre etiketi konum/değer/not/çakışmayı zaten söyler.

Geniş yatay ekranda (`orientation:landscape` ve ≥560 px) sayı/araç sütunu tahtanın sağında 300 px; DOM ve klavye sırası değişmez.

### Tahta boyutu

`.board-wrap` bir `container-type:size` kutusudur ve gezinme/durum/sayı/araç/ileti sabit yükseklikleri çıkarıldıktan sonra kalanı alır; tahta `min(100cqw,100cqh,640px)` karedir. Ek ileti çıktığında tahta küçülür, sayfa kaymaz. Alt sınır 280 px: bunun altında sayfa belge içinde kayar (kontrol hiçbir zaman kırpılmaz).

## Debug bayrağı

`?debug=1` bu tarayıcıda hatırlanır (`localStorage sudoku-rivals.debug`), `?debug=0` kapatır. Açıkken `html[data-debug]`: seçili hücre satırı, "kaydedildi"/"yerel değişiklikler gönderilecek" notları, klavye yardımı, düello notu ve oyun içinde başlık/alt bilgi görünür. Üretimde varsayılan kapalıdır; hiçbir test kimliği (`data-testid`) kaldırılmadı.

## Ölçüm (Chromium 155, `isMobile`, dokunmatik, DPR 2, koyu tema; her biri solo/yarış/düello × 6/9)

| Görünüm | Sonuç |
| --- | --- |
| 320×568, 360×640, 375×667, 375×812, 390×844, 412×915, 430×932 | Kaydırma yok; hedefler ≥48 px |
| 375×553 ve 390×664 (adres çubuklu Safari benzeri) | Kaydırma yok; 9×9 tahta 286–358 px |
| 667×375, 844×390 yatay | Kaydırma yok; sayılar/araçlar yanda; hedefler ≥48 px |
| 320×480 (çok küçük) | Kayar: +67 px (solo/yarış), +6 px (düello). Alt sınır tahta 280 px |

Hücre genişliği 9×9'da 30,5–43,8 px, 6×6'da 45,8–65,7 px. 9×9 hücreleri 48 px altındadır; sözleşmedeki gibi ikinci erişim yolu büyük sayı tuşları ve ok/seçim gezintisidir.

Ayrıca doğrulandı: açık tema; 28 px kök yazı boyutunda (≈%175) örtüşme/yatay kayma yok ve sayfa belge içinde kayar; solo klavye (oklar, 1–N, N, Delete, Ctrl+Z / Shift+Ctrl+Z); yarış klavye ve 4 araç, ipucu yok; düello araçsız ve klavyeyle sayı; çakışmada "Çakışma var" metni + hücrede "!"; ipucu notu çıkınca 375×667'de kayma yok; çevrimdışı olunca bağlantı metni görünür.

Ekran görüntüleri ve ham ölçümler: `../g14-mobile-layout-evidence/` (çalışma alanı kökü, repo dışı), `metrics.json`, `checks.log`.

## Bilinen açıklar

- `node tests/web-solo.mjs --browser` taban sürümde de (bu değişiklik olmadan) 175. satırdaki iddiada düşer: bozuk 9×9 kaydı yazıp sayfayı yenilemek, duraklatılmış açık oyunun `pagehide` kaydıyla üzerine yazılır. Bu iş sonucu değiştirmedi; o satır geçici atlanınca kalan tarayıcı akışı geçer.
- Minimum metin boyutu (sözleşmede 16) araç etiketlerinde ve durum satırında 12–13 px'tir; önceki ekranda da böyleydi.
- Fiziksel cihaz, VoiceOver/TalkBack ve gerçek Safari adres çubuğu davranışı denenmedi.

## Native için öneri (uygulanmadı)

**Android** (`MainActivity.kt`): oyun satırında "Sudoku Rivals" başlığı yerine `Mod · N×N` + süre; seçili hücre satırı ve "Oyun cihazına kaydedildi" notu `BuildConfig.DEBUG` veya `--ez debug true` arkasına (`testTag` ve `contentDescription` ekran okuyucu için korunur); durum satırı yalnız çakışma/bağlantı/rakip bilgisi; tahta boyutu `screenHeight - 330/390` tahmini yerine kalan yüksekliğe göre; 9×9 sayı tuşları `5+4`, 6×6 tek sıra (dar telefonda 3×2); `verticalScroll` yalnız sığmadığında. Canlı ekranda aynı sıra (`LiveScreens.kt`).

**iOS** (`GameViews.swift`, `LiveViews.swift`): `ScrollView` yerine `ViewThatFits(in: .vertical)` ile sabit düzen, sığmazsa `ScrollView`; klavye altyazısı ve seçili hücre metni launch argümanı `-SudokuDebug 1` arkasına (donanım klavyesi bağlıysa klavye ipucu gösterilebilir); "kaydedildi" durumu `accessibilityIdentifier` kalacak şekilde görünmez; durum satırı web ile aynı içerik.

Üç platformda ortak: aynı altı bölge ve sıra, aynı varsayılan-gizli unsurlar, debug anahtarı yerel (web URL/localStorage, Android debug/ekstra, iOS launch argümanı), kimlikler (`timer`, `game-status`, `storage-status`, `opponent-progress`, `own-score`, `input-lock`…) değişmez.
