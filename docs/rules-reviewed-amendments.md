# G01 — bağımsız inceleme sonrası düzeltmeler

Bu belge `/tmp/sudoku-goal-g01.md` taslağındaki aşağıda sayılan maddelerin yerine geçer. Diğer maddeler taslak olarak kalır. `[K]` kullanıcı kararı; `[Ö]` uygulamayı ilerletmek için geri alınabilir varsayılandır. Puanların oyun dengesi insanlarla sınanmadı.

## Kesin kapsam

- **[K]** Solo çevrimdışıdır. Klasik yarış ve 90 saniye düello davetli 1v1 oyunlardır; üç mod da 6×6 ve 9×9 çalışır. iOS/Android ayrı native arayüz, web ayrı arayüzdür. İlk doğru bitiren klasik yarışı kazanır.
- ODA-02'deki iki kişilik oda kararı soru değildir. Bölüm 12'deki S1 kaldırılır. S2'deki klasik yarışta yalnız çakışma gösterme seçeneği geri alınabilir varsayılandır; ilerlemek için soru beklenmez. Web solo ilk başarılı yükleme sonrası çevrimdışı çalışır.

## Klasik yarış

- **YRS-05/06/08 [K/Ö]:** Tam ve değişmiş bir tahta sunucuda hemen doğrulanır. Yanlışsa yalnız tahtanın doğru olmadığı söylenir; 10 saniyelik doğrulama kilidi yoktur. Aynı tahta sürümünün tekrarı yeni doğrulama veya sonuç oluşturmaz. Sunucuda ilk geçerli doğru bitiş kazanır; 100 ms beraberlik penceresi yoktur. Aynı saat damgasında oda olay sırası belirleyicidir. Örnek: A 754.210 ms, B 754.290 ms tamamladıysa A kazanır. A 600.000'de yanlış, 604.000'te doğru gönderir ve B 609.000'da doğru gönderirse A kazanır. L11 ve L12 kaldırılır; Y1–Y3 ve yarış durum tablosundaki eski kilit/pencere satırları bu kurallarla değiştirilir.
- **YRS-10/11 [Ö]:** Kopma süresi sunucunun bağlantıyı kopuk ilan ettiği anda başlar. Önerilen süre 120 saniyedir; kesin sınırda süre dolması önce uygulanır. İki sürenin aynı anda dolması sonuçsuz maçtır. Bağlı oyuncu için hareketsizlik yalnız uyarıdır; otomatik yenilgi değildir. L10'daki hükmen yenilgi ve durum tablosundaki buna bağlı satır kaldırılır.
- **YRS-12/BAG-05 [Ö]:** Kopuk oyuncu yerel tahtasında çalışabilir. Yeniden bağlantıda son onaylı sürüme dayalı tek bir tahta eşitlemesi gönderilir; sunucu tahtayı ve sürümü doğrular. Bitiş, eşitlemenin sunucuya kabul edildiği andır. Yerel değişiklikler onay gelmeden silinmez; uyuşmazlık kullanıcı verisi sessizce ezilmeden çözülür. Eşitleme her hücreyi sırayla hız sınırına sokmaz.

## Düello, süre ve işlem sırası

- **DLO-10 [Ö]:** İlk sürümde düelloda kalem notu ve geri alma yoktur; bu ekran sözleşmesiyle uyumludur. Solo ve yarışın not kuralları ayrıdır.
- **DLO-11/ADL-04 [Ö]:** Yetkili sunucu giriş noktası hamleye monoton saatle varış zamanı ve oda içi sıra verir. T0+90 saniyeden önce ulaşan geçerli komutlar sonuç kesinleşmeden sırayla işlenir; o anda veya sonrasında ulaşanlar reddedilir. Skor, son hücre bonusu ve sonraki tahtaya geçiş atomik uygulanır. 89.999 ms'de ulaşan hamle 90.010'da işlense bile kurallar çerçevesinde değerlendirilir.
- **DLO-14 [Ö]:** Düelloda kopuklukta süre işler. Yeniden bağlanınca eski bağlantı nesline ait işlenmemiş hamleler reddedilir; yeni sunucu görüntüsünden devam edilir. Geçerli bağlantıda süre dolmadan ulaşan hamleler işlenir. Sunucu gecikmiş paketle istemcide kasıtlı bekletilmiş hamleyi her durumda ayıramaz. Öndeyken kopup skoru koruma mevcut varsayılanın kabul edilen sonucudur; insan testinde değerlendirilir.
- **P5/P6 ve §6.2 [Ö]:** Ardışık yanlışlar arasında iki saniyelik kilit bitmiş olmalıdır. Puan değerleri yalnız deneysel varsayılandır; sıfır taban ve artan seri nedeniyle “rastgele deneme daima zararlıdır” sonucu çıkarılamaz. Tamamlama bonusu her tahta için en fazla bir kez verilir.

## Tekrar, oturum ve arıza

- **BAG-06 [Ö]:** `(maç kimliği, koltuk, sıra)` başına kabul veya ret sonucu saklanır; aynı içerikli tekrar aynı sonucu döndürür, tekrar ceza/puan üretmez. Aynı kimlikle farklı içerik reddedilir. Beklenen sırada boşluk varsa işlem uygulanmaz ve eşitleme istenir. Rövanş yeni maç kimliği, zincirdeki her bulmaca kendi tahta kimliği taşır.
- **ODA-09 [Ö]:** Aynı koltuk yetkisiyle bağlantı devrinde sunucu bağlantı neslini artırır ve eski neslin henüz işlenmemiş komutlarını reddeder. Yeni bağlantı son sıra ve durum sürümünü alır. Davet bağlantısı tek başına mevcut koltuğu devralamaz; oda doluysa reddedilir.
- **ADL-07/08 [Ö]:** Yeni hamle denemelerine doğruluk kontrolünden önce hız sınırı uygulanır. Aynı işlem kimliğiyle gelen tekrarlar hız kotasını veya cezayı yeniden tüketmez. Hız sınırı ve koparma eşikleri deneysel varsayılandır; yarışın tek tahta eşitlemesi ayrı sınırlandırılır.
- **BAG-08 [Ö]:** Sunucu arızasında kimseye haksız galibiyet veya yenilgi yazılmaz. Güvenli sürdürme için tahta yanında olay sırası, işlem sonuçları ve kesinleşmiş sonuç korunmalıdır; bunlar güvenilir biçimde kurtarılamazsa maç sonuçsuz biter. Canlı yarışta kopma sürelerinin çakışması ve yeniden başlama gerçek durum makinesi testlerinde doğrulanır.

## Kabul örnekleri

İstemci ve sunucu testleri en az şunları kapsar: 80 ms aralıklı iki doğru bitişte ilk kazanan; yanlış tahtayı düzeltenin kilitsiz tamamlaması; 89.999/90.000 ms sınırı ve geç işleme; yanıtı kayıp yanlış hamlenin tekrarı; farklı içerikli aynı sıra; eski bağlantı nesli; çevrimdışında 40 düzenleme sonrası tek eşitleme; iki kopmanın aynı anda sona ermesi; sunucu arızası; skor tabanında yanlış tahmin ve bonus tekrarları. Örnekler her iki boyut ve uygulanabilir iki canlı modda çalıştırılır.
