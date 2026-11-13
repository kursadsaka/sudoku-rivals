# G05 — ekran, öğe ve durum sözleşmesi v1

Makinece okunur kaynak `ui-contract.json`; uygulama için geri alınabilir varsayılanlardır, görsel marka veya insan testi onayı değildir. Sistem konuşmasındaki üç platform/üç mod/iki boyut kararını ve G01 inceleme düzeltmelerini uygular. iOS SwiftUI, Android Compose ve web kendi kontrollerini kullanır. Piksel eşitliği aranmaz.

## Düzen ve gezinme

Dar ekranda sıra: gezinme → durum → kare tahta → sayı tuşları → mod araçları → alt eylemler. Geniş ekranda sayılar/araçlar tahtanın yanında olabilir; okuma ve klavye sırası değişmez. Ana ekran ve kurulumda tahta bölgesi bulunmaz. Aynı modda aynı öğe kimliği kullanılır: web `data-testid`, Android `testTag`, iOS `accessibilityIdentifier`. Bunlar ekran okuyucunun konuştuğu etiketler değildir; etiketler yerelleştirilir. Hücre/numara kimlikleri ekranda 1 tabanlıdır; Rust indeksleri 0 tabanlıdır.

Ana ekran → solo kurulumu → solo oyun → duraklatma veya sonuç. Arkadaş seçimi → mod/boyut ve oda kur/katıl → lobi → geri sayım → yarış/düello → sonuç → karşılıklı rövanş/lobi. Geçersiz/kapalı/dolu oda bir maç sonucu değildir. Ayarlar ana ekrandan açılır; geri önceki ekrana döner. Sonuçtan ana sayfaya dönüş oynanan maçı sürdürmez.

Maçta geri eylemi ayrılma onayı açar; onaylanırsa sunucuya pes komutu gider. Sunucu onayı veya güncel görüntü gelmeden yerelde kazanan ilan edilmez. Bağlantı kopması gezinmeyi sonuca atmaz. Rövanş için iki oyuncunun onayı gerekir; yeni maç kimliği ve yeni bulmaca kullanılır.

## Mod davranışları

| Öğe | Solo | Yarış | Düello |
| --- | --- | --- | --- |
| Sayaç | Yerel geçen süre; arka planda/duraklatmada durur | Sunucu başlangıcından geçen süre | Sunucuya göre 90 saniyeden kalan süre |
| Tahta | Yerel durum | Yerel tahta + sunucu uzlaştırması | Sunucunun onayladığı kendi tahta |
| Not/sil/undo/redo | Var | Var; notlar yerel | Yok |
| İpucu | Açıklanabilir çekirdek ipucu varsa | Yok | Yok |
| Rakip | Yok | Dolu hücre oranı; doğruluk bilgisi yok | Skor; hücreleri ve serisi yok |
| Kopma | Oynanır ve kaydedilir | Yerelde oynanır; son onaylı sürümden tek sync | Giriş kapanır; süre sürer; eski hamleler biriktirilmez |

Aynı tahta boyutu ve moda uygun kontroller üç platformda da bulunur. Zorluk bir insan çözüm tekniğiyle doğrulanana kadar boş hücre sayısından “kolay/orta/zor” etiketi üretilmez. İpucu henüz desteklenmiyorsa doğru rakamı uyduran bir UI işlevi yazılmaz; çekirdek desteği tamamlanana kadar araç devre dışıdır. Bu eksikler tamamlandı sayılmaz.

## Veri ve hata durumları

Solo eylem önce çekirdekte atomik uygulanır, ardından sürümlü kayıt saklanır. Başarısız yazmada açık oyun ve önceki kayıt korunur; “kaydedildi” gösterilmez. Bozuk kayıt sessizce silinmez; mevcut kayıt korunarak tekrar deneme veya ayrı yeni oyun önerilir. Aynı boyutta yeni oyun önceki kaydın yerini alacaksa onay gerekir. Her boyutun kaydı ayrıdır.

Düello bekleyen hamlesi yeni bir sonuç gibi puan üretmez; onay gelmeden kesin skor artırılmaz. Kilit yalnız yanlış hamle cezasıdır; klasik yarışta 10 saniyelik tamamlama kilidi yoktur. Sunucu hatasında sonuçsuz ekranı görünür; hiçbir oyuncu yerel tahminle galip sayılmaz. Eski nesil/sürüm çakışmasında kullanıcı değişiklikleri silinmeden uzlaştırılır.

## Erişilebilirlik ve metin

- Konum, verilen/girilebilir, değer, manuel notlar ve çakışma hücre etiketinde söylenir. Çakışma renk yanında metin veya sembolle görünür.
- Tahta webde tek tab odağına ve oklarla dolaşıma sahip grid olur; 1–6/9 giriş, Delete/Backspace silme, N not, Ctrl/Cmd+Z geri alma, Shift+Ctrl/Cmd+Z yineleme. Modda kapalı eylem kısayolla da kapalıdır. Basılı tuş tekrarı düelloda yeni hamle oluşturmaz.
- Sayı tuşları her platformda vardır. Donanım klavyesi bağlıysa mobilde de giriş yapılabilir. Odak sayfa veya tahta değişiminde anlamlı yere taşınır; tuzak oluşturulmaz.
- Dokunma hedefi en az 48 platform birimi; büyütülmüş yazıda içerik sayfada kayar, kontroller üst üste binmez. 9×9 tahtanın dar sütunları için komşu hedefler birleştirilmez; seçili hücre gezinmesi ve büyük sayı tuşları ikinci erişim yoludur.
- Saat her saniye ekran okuyucuya bağırılmaz. Canlı bildirimler bağlantı, hata, hamle kilidi ve sonuca ayrılır. Duraklatmada solo tahta erişilebilir ağaçtan da kaldırılır.
- Sistem fontu, tema, azaltılmış hareket, ses ve titreşim tercihleri izlenir. Hareket girişi kilitlemez. Çözüm ve koltuk belirteci hiçbir etikete veya paylaşım metnine eklenmez.
- JSON renk rolleri yalnız başlangıç değerleridir. Metin çiftlerinde en az 4.5:1, sınır çiftinde 3:1 kontrast `node tests/ui-contract.mjs` ile hesaplanır; gerçek render/G15 testlerinin yerine geçmez.

## Kabul ve açık doğrulamalar

JSON'daki ekran/öğe/durumlar üç uygulamanın test hedefidir. Boyut 6/9 × mod solo/yarış/düello matrisinde uygun öğeler ve aynı okuma sırası doğrulanır. Solo ağsız yeniden açılır; davet webde kurulum olmadan karşılanır; kopma ve yeniden bağlanma her iki canlı modda görünür. Küçük ekran, büyük yazı, açık/koyu tema, VoiceOver/TalkBack ve yalnız web klavyesi gerçek uygulamalarda ayrıca denenir. Bu belge ve kontrast testi o kontrollerin geçtiği iddiası değildir.
