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

## Sınırlar

- Canlı oyun ekranı (yarış/düello oynarken) İngilizce'de emülatörde elle/otomatik açılmadı: o ekran WASM botlu Türkçe
  süitle sınandı; İngilizce metinler aynı biçim argümanlarını kullanır (yer tutucu eşliği testle doğrulandı) ve
  İngilizce lobi, ayrılma iletişim kutusu ve solo oyun görüntü/etiket olarak doğrulandı. Gerçek İngilizce duello/yarış
  oyun ekranını görmek için harness'e bir İngilizce çalıştırma modu gerekir (şimdi Türkçe sabit).
- Metinlerin ana dili konuşan biri tarafından gözden geçirilmesi yapılmadı (makine/geliştirici çevirisi). Ek diller yok.
- Sağdan sola diller, çok büyük yazı tipi ve gerçek cihaz/TalkBack bu turda denenmedi. İngilizce metinler Türkçeden
  biraz uzun/kısa; 320×568 dp solo yerleşimi İngilizce'de de aynı (kaydırma yalnız bu boyutta).
- Mağaza girişi (başlık/açıklama/ekran görüntüleri), uygulama simgesi ve imzalı yayın bu işin dışında.
