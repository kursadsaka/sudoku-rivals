# Sudoku Rivals — platformlar arası kabul senaryoları (taslak)

Bu senaryolar G05, G14, G15 ve G17 için uygulanacak doğrulamaların başlangıç listesidir. Henüz çalıştırılmış test veya onay kanıtı değildir. G01 oyun kuralları ve G02 mimari kararı tamamlanınca olaylar, süreler ve teknik düzenek kesinleştirilir.

## Matris

Her platformda (web, Android, iOS) her iki tahta boyutunda (6×6, 9×9) solo, klasik yarış ve 90 saniyelik düello çalışmalıdır: toplam 18 platform/boyut/mod birleşimi. Solo ayrı doğrulanır. İki oyunculu modlarda her boyut ve mod için şu altı yönsüz platform eşleşmesi çalıştırılır: web–web, Android–Android, iOS–iOS, web–Android, web–iOS, Android–iOS. Farklı platform eşleşmelerinde oda kuran taraf da değiştirilir. Her iki oyuncunun aynı sunucuya bağlı olduğu, aynı maç kimliği ve sonuç üzerinden doğrulanır.

## Solo

1. Her boyutta ağ kapalıyken yeni oyun başlat; verilen hücrelerin düzenlenemediğini, geçerli sayı aralığını ve ilgili bölge sınırlarını kontrol et.
2. Sayı girişi, silme, kalem notu ve geri almayı dene; uygulamayı kapatıp ağ kapalıyken aç. Tahta, notlar ve mevcut oyun doğru devam etsin.
3. Oyun bittiğinde geçersiz veya eksik tahta tamamlandı sayılmasın. Doğru tahta tamamlandı durumuna geçsin ve yeniden açılınca sonuç tutarlı olsun.
4. Yerel kayıt kullanılamaz veya bozuksa başarı gösterme; kullanıcının mevcut verisi sessizce üzerine yazılmadan anlaşılır hata sun.

## Lobi, davet ve başlangıç

1. Misafir oyuncu oda oluşturur; arkadaş davet bağlantısı veya kodla katılır. Web bağlantısı uygulama kurmadan oyuna eriştirir. Hatalı, süresi geçmiş ve dolu oda anlaşılır biçimde reddedilir.
2. İki oyuncu hazır olmadan maç başlamaz. Aynı boyut, mod ve bulmaca iki tarafta doğrulanır; başlangıç sırası ve zamanına sunucu karar verir.
3. İki istemci aynı anda katılmaya veya başlamaya çalıştığında oda kapasitesi aşılmaz ve yalnız tek maç başlar. Oda kodu tahmin denemeleri sunucu tarafında sınırlandırılır.

## Klasik yarış

1. Her oyuncu kendi tahtasına sayı ve not girer; birinin hücreleri diğerinin tahtasını değiştirmez. Silme ve geri alma yerel tahtada doğru işler.
2. Eksik veya yanlış bir tahta kazanmış sayılmaz. Tam ve doğru çözüm sunucuya iletildiğinde tek kazanan ile aynı sonuç iki istemcide gösterilir.
3. Yakın zamanlı iki doğru bitişte istemci saatleri değiştirilse de sunucunun belirlediği sıra kullanılır; tekrarlanan istek ikinci sonuç üretmez.

## 90 saniyelik düello

1. İstemciler aynı bulmaca sırasını alır. Doğru, yanlış ve art arda hamlelerde gösterilen skor ve kombo, G01 kurallarına göre sunucunun sonucuyla eşleşir.
2. Biten bulmaca sonraki bulmacayı açar; puan tekrarlanan mesaj veya yeniden bağlantı nedeniyle iki kez yazılmaz. Rakibin çözümü paylaşılmaz.
3. İstemci saati ileri/geri alındığında ve ağ gecikmesinde maç sınırını sunucu belirler. Süre bitiminden sonraki hamleler puan kazanmaz; beraberlik G01 kuralına göre çözülür.
4. Hatalı hamle cezası ve giriş kilidi sunucu tarafından uygulanır; kısa aralıklarla tekrar eden veya değiştirilmiş hamleler fazladan puan sağlamaz.

## Bağlantı, yeniden katılma ve sonuç

1. Her iki canlı modda, her platform eşleşmesinde en az bir oyuncunun bağlantısını kesip yeniden kur. Sunucunun güncel maç durumu, tahta ve sonuç tekrarlar üretilmeden geri gelsin; kopma süresindeki davranış G01 kuralına uygun olsun.
2. Uygulamayı arka plana alıp yeniden aç; ağ kesintisi ve sunucuya ulaşılamama durumları görünür olsun. Rakip ayrıldığında veya maç zaman aşımına uğradığında sonuç açıkça belirtilebilsin.
3. Sonuç ekranından rövanş talebi gönder; iki oyuncunun kararı ve yeni maç durumu tutarlı olsun. Aynı talebin tekrarı fazladan oda veya maç oluşturmasın.

## Erişilebilirlik ve yerleşim

Her platformda küçük ekran, büyük sistem yazısı, yüksek kontrast, azaltılmış hareket ve ekran okuyucuyla oyun başlatma, hücre seçimi, sayı girme ve sonucu anlama kontrol edilir. Webde klavye ile aynı temel akış tamamlanır. Tahta, durum, tuş takımı ve moda uygun araçların sırası [ekran sözleşmesine](ui-contract.md) uyarken platforma özgü görünüm kabul edilir.

## Kanıt kaydı

Her koşu için tarih, uygulama commit'i, sunucu commit'i, istemci/platform/OS sürümleri, cihaz veya emülatör, mod, boyut, iki oyunculu testte iki uç, test sonucu ve hata kaydı tutulur. Gerçek cihaz, simülatör ve otomatik test ayrı işaretlenir. Gerçek kullanıcı deneyimi ve davet akışına ilişkin geri bildirim teknik testin yerine geçmez; G17'de ayrıca kaydedilir.
