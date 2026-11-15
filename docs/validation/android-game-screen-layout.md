# Android oyun ekranı: tek ekran yerleşimi ve debug bayrağı

15 Kasım 2026. Taban `agent/g16-render-free` / `7ce2591`. Referans: web düzeni
`g14-fixes` `58ae6d0` ([web-game-screen-layout.md](../../../g14-fixes/docs/validation/web-game-screen-layout.md),
çalışma alanındaki ayrı checkout) ve kullanıcının eski ekran görüntüsü (PDF). `apps/web` ve `apps/ios`
değişmedi; sunucu, Render ve Notes'a dokunulmadı.

## Değişiklik

Solo, klasik yarış ve düello (6×6/9×9) oyun ekranları aynı altı bölgeyi aynı sırayla gösterir:

1. **Gezinme:** geri · `Mod · N×N` + süre (`mode-label`, `timer`) · sağ slot (solo: Duraklat; canlı: rakip adı).
2. **Tek satır durum:** bağlantı noktası (sorun varsa metin) · yarış: `Rakip %N` · düello: `Sen/Rakip/Seri` ·
   gerektiğinde kilit/çakışma/onay bekleniyor. Solo'da yalnız çakışma yazar; sağlıklı durum ekran okuyucuya
   `game-status` açıklamasıyla söylenir.
3. **Tahta:** kalan yüksekliğe sığan en büyük kare (en çok 560 dp). Notlar hücrede küçük rakamlarla çizilir
   (satır başına 3); önceki "seçili hücre" satırı bu yüzden gerekmez.
4. **Sayılar:** 6×6 tek sıra (≥318 dp içerik genişliğinde, yoksa 3×2), 9×9 `5+4` ortalı. Hedef ≥48 dp.
5. **Araçlar:** solo 5, yarış 4, düello yok.
6. **Geçici iletiler** yalnız gerektiğinde (kayıt hatası + tekrar dene, uzlaştırma, ipucu/yanlış hamle, `live-message`).

Yeni `GameLayout.kt` (`GameScreenLayout`, `GameNavBar`, `DigitPad`, `SudokuBoard`) iki ekranın yinelenen tahta kodunu
birleştirir; `testTag`, hücre açıklamaları, `collectionInfo` ve canlı bölgeler korundu. Sayfa yalnız 280 dp'lik tahta bile
sığmazsa kayar (hiçbir kontrol kırpılmaz). Yatay ve ≥560 dp'de sayı/araç sütunu tahtanın sağında 300 dp.

**Debug bayrağı:** `UiDebug.enabled`, yalnız debug build'de `adb shell am start ... --ez debug_ui true` ile açılır
(`false` kapatır); kalıcı değildir, staging/release'te yoktur. Açıkken uygulama başlığı, seçili hücre satırı
(`selected-cell`), "Oyun cihazına kaydedildi" ve "Oyun devam ediyor" görünür. Varsayılanda yok.
Kayıt hatası metni ve Tekrar dene her zaman görünür. Lobi, sonuç ve ana ekranlar değişmedi.

## Doğrulama (API 36 / 16 KB x86_64 emülatör, debug APK)

- `tests/android-game-layout.py` (solo, yeni): 6×6 ve 9×9 × 411×914, 411×731, 360×640, 320×568 dp ve 731×411 yatay;
  sıra, ≥48 dp hedef, sayı satırları, gizli debug metinleri ve "yukarı kaydırınca tahta yerinden oynamıyor".
  **Hepsi geçti**; 320×568 dp dışında kaydırma yok (orada tahta 280 dp'ye iner, sayfa kayar: kabul edilen çok küçük ekran).
  Tahta 387 dp (411 dp genişlik), 336/318 dp (360×640), 347 dp (yatay). `debug_ui` açık/kapalı/yeniden açılış geçti.
- `tests/android-multiplayer.py --layout-check <klasör>` (canlı): aynı denetim yarış/düello × 6/9 için aynı beş ekran
  boyutunda **20 ölçüm geçti** (düello 320×568'de bile kaydırmasız: 296 dp; yarış 320×568'de kayar). Aynı koşunun tam
  canlı süiti de geçti (deep link, kayıp yanıt tekrarı, çevrimdışı düzenleme, uzlaştırma, 90 sn, rövanş, ayrılma, restart).
- `tests/android-solo.py` (320×640): tam geçti. Kaydırma yardımcısı sabit `(540,1580)` koordinatı yerine geçerli ekranı
  kullanacak şekilde düzeltildi (aynı kusur önceki çok oyunculu harness'te de vardı).
- Birim: `test_android_layout_check` (4), `test_android_multiplayer_fixture`, `test_android_native_alignment` geçti;
  `tests/android-staging.py --aapt2` statik denetimi geçti (staging/release manifestleri değişmedi).

Ekran görüntüleri ve ham ölçümler çalışma alanında, repo dışında: `../g16-android-layout-evidence/` (`solo/`, `live/`,
`solo-metrics.json`).

## Sınırlar

- Gerçek cihaz, TalkBack, gerçek `fontScale` > 1 ve API 26 denenmedi; büyük yazıda yerleşim ölçüleri ölçüm
  sırasında hesaplandığından kayar ama bu turda emülatörde doğrulanmadı.
- "Oyun cihazına kaydedildi" bilgisi artık varsayılanda ekran okuyucuya da söylenmez (hata durumları söylenir);
  web'deki `sr-only` karşılığı eklenmedi.
- Uzun ekranda kalan boşluk tahtanın üstü ve kontrollerin altına bölünür (gezinme üstte sabit); piksel eşitliği aranmadı.
- 320×568 dp'de solo ve yarış kayar; düello kaymaz. Notlar hücrede 7 sp'den küçüğe inmez (9×9, 320 dp'de zor okunur).
- Canlı lobi/sonuç ekranları ve iOS bu sadeleştirmenin dışındadır.
