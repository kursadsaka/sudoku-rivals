# G16 — iOS oyun ekranı: ortak tek ekran yerleşimi ve uygulama ikonu

15 Kasım 2026. Taban: `agent/g16-ios-store-prep` (`650c06c`, iOS Staging yapılandırması üzerinde). Web
(`58ae6d0`) ve Android (`6516d48`) yerleşimi `origin/agent/g16-render-free` üzerinden okundu; o dala
merge/rebase yapılmadı, iOS dosyaları web/Android dosyalarıyla çakışmaz. Render, hesap, imza, deploy, Notes yok.

## Yerleşim

Solo, klasik yarış ve düello (6×6 ve 9×9) aynı sırayı kullanır (`GameLayout.swift`: `GameScreenLayout`):

1. **Gezinme satırı**: geri (`back`) · `Mod · N×N` + süre (`timer`) · sağ slot (solo: duraklat `pause`; canlı: rakip adı).
   Sistem gezinme çubuğu oyun ekranında gizlenir (uygulama başlığı yok).
2. **Durum satırı** (en az 24 pt): canlıda bağlantı noktası (sağlıklıyken yalnız nokta, `connection-status`
   etiketi "Connected" kalır; sorun varsa metin), yarışta rakip ilerlemesi, düelloda puan/rakip/seri, çakışma,
   kilit, yanlış hamle iletileri. Nötr "oyun devam ediyor" görünmez ama erişilebilirlik ağacında (`game-status`).
3. **Tahta**: kalan yüksekliğe sığan en büyük kare (en fazla 560 pt).
4. **Sayı tuşları**: 6×6 tek sıra (328 pt'den dar alanda 3×2), 9×9 `5+4` ikinci sıra ortalı; hedef ≥48 pt
   (satır 52 pt, erişilebilir boyutlarda 72 pt).
5. **Araçlar**: solo 5 (kalem, sil, geri al, yinele, ipucu), yarış 4, düello yok.
6. **Geçici iletiler** (yalnız gerektiğinde; tahta küçülür): ipucu kartı, oyun iletisi, uzlaştırma, kayıt hatası + yeniden dene, sunucu hata iletisi.

Yatay ve geniş ekranda (genişlik ≥560 pt ve yüksekliğin 1,2 katından fazla) gezinme+durum+sayılar+araçlar tahtanın
sağında 300 pt'lik panelde. Erişilebilirlik sırası değişmez (gezinme → durum → tahta → sayılar → araçlar).
Tahta 280 pt'ten (yatayda 240 pt) küçülecekse (çok küçük ekran, erişilebilirlik yazı boyutları)
`ViewThatFits` ile `ScrollView`'a düşer: hiçbir kontrol kırpılmaz.

## Debug bayrağı

`-SudokuDebug` başlatma argümanı **yalnız Debug derlemesinde** geçerlidir (`UIDebug`; Staging/Release'te
derleme zamanında kapalı, `UserDefaults` kullanılmaz). Açıkken: sistem gezinme çubuğu/uygulama başlığı, seçili
hücre satırı, klavye yardımı, görünür "kaydedildi"/"oyun devam ediyor" ve düello kuralları notu. Kapalıyken:
"kaydedildi" ekranda yok ama aynı kimlikli 1 pt erişilebilirlik öğesi olarak kalır (`storage-status`, VoiceOver
için web'deki sr-only metin gibi); kayıt hatası ve yeniden dene her zaman görünür; düello kuralları sayı tuşlarının
`accessibilityHint`'idir. Hiçbir `accessibilityIdentifier` kaldırılmadı/yeniden adlandırılmadı; yalnız geri düğmesinin
görünen etiketi "Home/Ana sayfa" yerine "Back/Geri" oldu (kimlik `back`).

## Uygulama ikonu

`apps/ios/SudokuRivals/Assets.xcassets/AppIcon.appiconset/icon-1024.png`: tek 1024×1024, opak, alfa kanalsız
(`sips`: `hasAlpha: no`), kare (iOS köşeleri kendisi yuvarlar). Kaynak: `apps/android/branding/ic_launcher_master.svg`
(origin `f5eb6f6`: marka mavisi `#1d4ed8`, beyaz 3×3 ızgara, dolu orta hücre), headless Chrome ile 1:1 çizildi;
tarif `apps/ios/branding/README.md`. `ASSETCATALOG_COMPILER_APPICON_NAME=AppIcon`. Android/web ile aynı çizim.

## Kanıt (Mac, Xcode 27.2, iOS 27.2 simülatörü iPhone 17 402×874 pt, özel simülatör)

- Debug ve Staging yapılandırmaları derlendi (simge dahil); birim testleri 32/32.
- `SoloUITests` 14/14 (mevcut 11 + yeni 3): `testGameScreenFitsOnePhoneScreenInBothSizes` (solo 6×6 ve 9×9: geri,
  zaman, tahta, ilk/son rakam, 5 araç erişilebilir ve pencere içinde, sıra gezinme→tahta→rakam→araç, rakam hedefi
  ≥48, tahta kare, klavye/"kaydedildi" metni çizilmiyor, `game-status`/`storage-status` ağaçta), `testDebugFlagShowsDeveloperText`,
  `testLandscapeKeepsBoardDigitsAndToolsWithoutScrolling` (9×9 yatay, panel tahtanın sağında). Türkçe + AccessibilityXL
  testi (kaydırmayla ulaşım) değişmeden geçti.
- Canlı (`tests/ios-multiplayer.py -- -only-testing:SudokuRivalsUITests/LiveUITests`, yerel sunucu): 5/5 geçti. `startMatch`
  her canlı UI testinde aynı tek ekran denetimini yapar (yarış 4 araç, düello araçsız): yarış 6×6, yarış 9×9, düello 9×9
  ve süresi dolmuş yarış/düello kurtarma akışları dahil. (İlk koşuda iki test yardımcıdaki mod varsayımı yüzünden düştü —
  uygulama değil test hatası; mod bot anlık görüntüsünden okunacak şekilde düzeltilip yalnız o testler + yarış 6×6 yeniden koşuldu,
  düello 9×9 ve süresi dolmuş yarış ilk koşuda zaten geçmişti.) Aynı koşuda web paketi yerine taslak `SUDOKU_WEB_ROOT` kullanıldı.
- `python3 tests/ios-staging.py`: 10/10 (yeni: `-SudokuDebug` Staging ikilisinde yok, simge katalogda ve Info.plist'te).
- Ekran görüntüsü: iPhone 17'de solo 9×9 kaydırmasız (gezinme, tahta, 5+4 rakam, araçlar).

## Sınırlar

- Yalnız iPhone 17 (402×874) ve yatayı ölçüldü; 320×568 / SE sınıfı küçük ekran ve diğer boyutlar çalıştırılmadı
  (bu cihazlar bu Mac'te yok). Kaydırma yedeği AccessibilityXL ile denendi, küçük ekranla değil.
- Tahta genişlikle sınırlandığında artan yükseklik tahta üstü/altı arasında paylaşılır (Android ile aynı); iPhone 17'de
  nav ile tahta arasında ~80 pt boşluk kalır. Daha sıkı istenirse `spare` dağılımı tek satırda değişir.
- Düello 6×6 ve yarış 9×9 yerleşimi yalnız canlı UI testlerinin kapsadığı kadar; yeni tam matris tekrarı yapılmadı.
- Ekran okuyucu/VoiceOver gezintisi elle denenmedi; sıra ve kimlikler testle korundu. Fiziksel cihaz yok.
- iPad: aynı yerleşim (portrede genişlik/560 pt sınırı) derlenir ama iPad simülatöründe çalıştırılmadı.
