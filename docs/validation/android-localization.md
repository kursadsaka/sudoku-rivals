# Android dil desteği: İngilizce (varsayılan) ve Türkçe

15 Kasım 2026. Taban `agent/g16-render-free` / `364ffad`, çalışma ağacı temiz. Yalnız `apps/android`, testler ve bu belge.
Render servisi, staging, Notes, web/iOS ve oyun/protokol mantığı değişmedi.

## Değişiklik

- Kullanıcıya görünen tüm metinler Android kaynaklarına alındı: `res/values/strings.xml` (İngilizce, varsayılan) ve
  `res/values-tr/strings.xml` (Türkçe; mevcut Türkçe metinler birebir korundu). 181 metin + 2 çoğul. Uygulama
  sistem diline uyar: Türkçe cihazda Türkçe, diğer tüm dillerde İngilizce. Android 13+ cihazlarda sistem ayarlarındaki
  "Uygulama dilleri" listesinde `English (US)` ve `Türkçe` çıkar (`androidResources.generateLocaleConfig = true`,
  `res/resources.properties` ile `unqualifiedResLocale=en-US`; AGP `locales_config.xml` üretir). API 26–32'de sistem dili izlenir.
- Durum nesneleri (`SoloState`, `LiveState`) artık çözülmüş metin tutmaz, kaynak kimliği tutan `UiText` taşır; metin çizim
  anında geçerli dille çözülür (hata/ipucu/bağlantı iletileri, sunucu hata eşlemeleri). Dil değişince etkinlik yeniden
  oluşur; `configChanges` listesine dil eklenmedi, kayıtlar dosyadan geri yüklenir (oyun, kayıt, sunucu kodu değişmedi).
- Biçim: konumlu yer tutucular (`%1$d`), çoğullar için `plurals` (geri sayım ve giriş kilidi saniyesi: "1 second" /
  "2 seconds"; Türkçe iki biçim aynı). Sıra/sütun ifadeleri, ipucu açıklamaları ve hücre erişilebilirlik etiketi
  (`Row 3, column 5, given, 7, notes: none` / `Satır 3, sütun 5, verilen, 7, notlar: yok`) çevirmenin sırayı
  değiştirebileceği tek bir biçim kaynağından üretilir (`cellLabel`).
- Erişilebilir etiketler (geri, kalem, geri al, yinele, ipucu, süre, bağlantı noktası, rakip/skor/seri, durum canlı
  bölgeleri, hücre eylemi "Select cell") kaynaklara taşındı ve iki dilde var. Dekoratif `‹ ✎ ↶ ↷ ?` simgeleri ekran
  okuyucudan gizli, etiketi bir ayrı açıklamadır.
- Çevrilmeyenler (bilinçli): uygulama adı/marka (`Sudoku Rivals`, staging etiketi derleme yer tutucusu), rakibin
  görünen adı, oda kodu, sayılar/süre biçimi (`mm:ss`), sunucu protokol kodları. Varsayılan misafir adı dile göre
  (`Guest` / `Misafir`) gönderilir. Debug-only metinler (`--ez debug_ui true`) de kaynak olarak iki dilde.

## Doğrulama

- `python3 -m unittest tests/test_android_i18n.py` (8 test, cihazsız): iki dilde aynı anahtarlar, aynı yer tutucular,
  çoğullarda `one`/`other` (İngilizce biçimleri farklı), boş/çevrilmemiş metin yok, kodda kullanılan her kaynak tanımlı ve
  her tanımlı kaynak kullanılıyor, Kotlin'de sabit kodlanmış görünür metin ve Türkçe harf yok, `generateLocaleConfig`.
- `gradle lintDebug`: `MissingTranslation`/`ExtraTranslation`/`StringFormat*` yok; 6 `PluralsCandidate` uyarısı yanlış
  pozitif (`%d` bir adet değil: tahta boyutu, satır/sütun numarası, rakam) ve kaynakta `tools:ignore` ile gerekçeli.
- `tests/android-locale.py` (yeni, API 36 emülatör, debug APK): `tr-TR`, `en-US` ve desteklenmeyen `de-DE` (İngilizce'ye
  düşmeli) için Ana ekran, Ayarlar, Tek başına kurulum, solo 6×6 oyun (görünür metinler + erişilebilirlik etiketleri +
  hücre etiketi + süre açıklaması), duraklatma, canlı kurulum, yerel sunucuda kurulan odanın lobisi ve ayrılma
  iletişim kutusu. **141 beklenti geçti**; beklentiler doğrudan `strings.xml`'den okunur. Negatif kontrol: Türkçe uygulamaya
  İngilizce beklenti verilince test beklendiği gibi başarısız oldu.
- Regresyon (Türkçe sabitlenerek): `tests/android-solo.py` (320×640) ve tam `tests/android-multiplayer.py --layout-check`
  süiti; İngilizce'de `tests/android-game-layout.py --locale en-US` (solo 6×6/9×9 × beş ekran boyutu, 12 ölçüm).
- Testler Türkçe metin okuyorsa `tests/android_locale.py` ile uygulama dilini `tr-TR`'ye sabitler, sonunda eski ayarı geri alır
  (`cmd locale set-app-locales`).

Ekran görüntüleri çalışma alanında, repo dışında: `../g16-android-i18n-evidence/`.

## İngilizce canlı oyun ekranı (ek doğrulama)

`tests/android-multiplayer.py --locale en-US --layout-check <klasör>` (varsayılan `tr-TR`) gerçek emülatörde (API 36,
sistem dili en-US) tam canlı süiti İngilizce çalıştırır: iki mod × 6/9 (WASM botuna karşı) lobi, geri sayım, oyun, rövanş,
ayrılma, bağlantı kopması ve sunucu yeniden başlatma akışları geçti. Süit metin iddialarını `NEEDLE` tablosundan dile göre
okur; ek denetimler `--layout-check` ile:

- Her mod/boyutta oyun ekranının metin ve etiketleri `strings.xml`'den hesaplanır: `Classic race · 6×6` / `Duel · 9×9`,
  geri etiketi, bağlantı noktası (`Connected`), yarışta `Rival 0%` (+ açıklama), kalem/geri al/yinele etiketleri; düelloda
  `You 0`, `Rival 0` veya `–`, `Streak 0` ve erişilebilir karşılıkları; süre açıklaması ve hücre etiketi.
- Geri sayım çoğulu ("Starts in 2 seconds") ve düello giriş kilidi ("Lock 2 s" / "Input locked: 2 seconds") ekranda
  gözlemlenir; süitin sonunda ikisinin de en az bir kez görülmesi şarttır. Kilit, kayıp yanıt akışı süresini tükettiği için
  ek bir düşürülmeyen yanlış hamleyle (yalnız `--layout-check`) görünür kılınır.
- Beş ekran boyutunda × 4 mod/boyutla **20 yerleşim ölçümü İngilizce'de geçti** (kaydırma yok; yalnız yarışta 320×568 dp'de
  kayar, düello kaymaz). Aynı süit Türkçe'de de geçti.
- Bulgu ve düzeltme: İngilizce yarış durumu `Rival %0` gösteriyordu (yüzde işareti Türkçe sırasıyla). İngilizce kaynak
  `Rival %1$d%%` / `Rival: %1$d%% of cells filled` yapıldı; Türkçe `Rakip %N` aynı. Üretim kodu değişmedi.
- Uiautomator tıklanabilir düğümün etiketini çocuk düğümde gösterir; denetim butonlar için etiketi ağaçta arar.

## Sınırlar

- Canlı oyun ekranındaki İngilizce metinler yalnız yerel WASM botlu süitle ve 411×731 / 360×640 / 320×568 dp, yatay ekranlarda
  denendi; gerçek sunucu gecikmesi/cihaz yok.
- Metinlerin ana dili konuşan biri tarafından gözden geçirilmesi yapılmadı (makine/geliştirici çevirisi). Ek diller yok.
- Sağdan sola diller, çok büyük yazı tipi ve gerçek cihaz/TalkBack bu turda denenmedi. İngilizce metinler Türkçeden
  biraz uzun/kısa; 320×568 dp solo yerleşimi İngilizce'de de aynı (kaydırma yalnız bu boyutta).
- Mağaza girişi (başlık/açıklama/ekran görüntüleri), uygulama simgesi ve imzalı yayın bu işin dışında.
