# Android uygulama simgesi ve uyarlanabilir simge

15 Kasım 2026. Taban `agent/g16-render-free` / `b6082d1`. Yalnız `apps/android`, testler ve bu belge; web/iOS dosyaları değişmedi.

## Mevcut tasarım ve karar

Android'de simge hiç yoktu (release lint "missing application icon" uyarısı veriyordu); iOS'ta da simge varlığı yok. Görsel kimlik
`apps/web/icon.svg`'de zaten var: marka mavisi `#1d4ed8` (= `docs/ui-contract.json` accent'i), beyaz 3×3 ızgara, ortada dolu hücre.
Yeni tasarım uydurulmadı; bu çizim Android'e uyarlandı. Dış/telifli asset, yazı tipi veya raster kaynak yok; her şey vektör.

## Eklenenler (`apps/android`)

- `app/src/main/res/mipmap-anydpi/ic_launcher.xml` ve `ic_launcher_round.xml`: `adaptive-icon` (arka plan rengi, ön plan, Android 13+ tek renkli katman).
  minSdk 26 olduğundan raster mipmap gerekmez.
- `drawable/ic_launcher_foreground.xml` (beyaz), `drawable/ic_launcher_monochrome.xml` (temalı simge katmanı; yalnız alfa kullanılır),
  `values/ic_launcher_background.xml` (`#1D4ED8`). 108 dp tuvalde ızgara 48 dp, ortalı; en uzak nokta + çizgi yarı kalınlığı merkezden
  ≈34 dp, yani launcher maskesinin hiç kesmediği 72 dp (yarıçap 36) dairenin içinde.
- `AndroidManifest.xml`: `android:icon` ve `android:roundIcon`. Debug/staging/release için tek kaynak; `applicationId` (`com.sudokurivals`),
  staging soneki (`.staging`) ve etiketler değişmedi.
- `branding/`: `ic_launcher_master.svg` (web ikonunun köşesi yuvarlatılmamış tam kanama hâli), `ic_launcher-playstore.png` (512×512 RGBA, ~5 KB;
  Play Console mağaza simgesi, önceden yuvarlatılmamış) ve `README.md` (yeniden üretme komutu).

## Doğrulama

- `tests/test_android_icon.py` (7 test, cihazsız): manifest bağlantıları ve kimliklerin korunması, üç katmanlı adaptive-icon, marka rengi web ikonu ve UI sözleşmesi
  ile aynı, ön planın güvenli bölgede ve ortalı olması (yol uç noktalarından hesaplanır), ön plan beyaz/monochrome tek renk, master SVG = web ikonu − `rx`,
  mağaza PNG'si 512×512 RGBA ve < 1 MB.
- Derleme: debug, staging ve release derlendi. `tests/android-staging.py --aapt2` üç APK'da simgeyi (aapt2 `application-icon`) ve önceki tüm statik
  denetimleri geçti. `aapt2 dump badging`: paket adları `com.sudokurivals` / `com.sudokurivals.staging`, etiketler `Sudoku Rivals` / `Sudoku Rivals Staging`.
- Lint: debug 0 hata/**7** uyarı (öncesi 8), release 0/**3** (öncesi 4): "eksik simge" uyarısı kalktı, yeni uyarı yok.
- Emülatör (API 36, Pixel launcher): debug ve staging uygulamaları launcher'da mavi daire içinde beyaz ızgara ve ortada beyaz hücre olarak görünür; ikisi aynı simgeyi,
  ayrı etiketleri taşır. Ekran görüntüleri repo dışında: `../g16-android-icon-evidence/`.

## Sınırlar

- Temalı (monochrome) simge yalnız statik olarak (katman var, tek renk, güvenli bölgede) doğrulandı; emülatörde duvar kağıdı renklendirmesi açılıp görülmedi.
- Squircle/yuvarlatılmış kare gibi diğer üretici maskeleri denenmedi; güvenli bölge hesabı bunları kapsar. Gerçek cihaz yok.
- Debug ve staging aynı simgeyi kullanır (etiket ayırır). İstenirse staging'e küçük bir rozet eklenebilir; bu tur kimlik/yapı değiştirmemek için yapılmadı.
- Play Console mağaza girişi (öne çıkan 1024×500 görsel, ekran görüntüleri, açıklamalar) bu işin dışında.

## Platformlar arası ortak simge: kaynak ve öneriler (uygulanmadı)

Ortak kaynak `apps/android/branding/ic_launcher_master.svg` (tam kanama, 192×192, yalnız `#1d4ed8` + beyaz). Öneriler:

1. **Ortak konum:** iki platform da kullanacağı için ana SVG'yi platform-nötr bir yere (ör. `design/app-icon/master.svg` + `README` ile `rsvg-convert` komutları) taşımak;
   Android, iOS ve web bu tek dosyadan üretilsin. (Android dosyaları bu tur yalnız `apps/android` altında kaldı.)
2. **iOS:** `apps/ios/project.yml`'da simge/`AppIcon` ayarı yok. Master'dan 1024×1024 **opak** PNG (alfa ve köşe yuvarlatma yok; iOS maskeler) üretip
   `Assets.xcassets/AppIcon.appiconset` (Xcode 14+ tek boyut) ve `ASSETCATALOG_COMPILER_APPICON_NAME=AppIcon` ile bağlamak; karanlık/tinted varyant isteğe bağlı.
3. **Web:** `manifest.webmanifest` yalnız `icon.svg`'yi `any maskable` olarak veriyor, ama dosyada `rx="42"` yuvarlak köşe ve saydam köşeler var; maskable için uygun değil.
   Öneri: master'dan 192/512 PNG (`any`) ve ayrı bir `maskable` PNG (tam kanama, içerik merkez %80'inde: ızgara zaten ≈%62), ayrıca 180×180 `apple-touch-icon`;
   `icon.svg` tarayıcı sekmesi için kalabilir. Chrome kurulabilirliği PNG 192/512 ister.
4. **Tutarlılık:** marka mavisi tek yerden (UI sözleşmesi accent'i) gelsin; simge testleri (`test_android_icon.py`) renk eşitliğini zaten denetler, iOS/web için benzeri eklenebilir.
5. **Staging/debug ayrımı:** gerekirse Android/iOS staging derlemelerine köşe rozeti veya farklı arka plan tonu verilebilir (sade tutmak için şimdilik yok).
