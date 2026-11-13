# G01 — uygulama kuralları v1

Bu belge, Sistem konuşmasındaki kesin kapsamı ve bağımsız G01 inceleme düzeltmelerini tek uygulama kaynağına toplar. **Kesin:** üç platform, üç mod, iki boyut, davetli 1v1, yarışta ilk doğru bitiş ve düelloda 90 saniye. Diğer sayılar/ayrıntılar ilerlemek için geri alınabilir uygulama varsayılanlarıdır; insanlarla oyun dengesi testi veya kullanıcı marka onayı değildir. İnceleme gerekçeleri `rules-reviewed-amendments.md` içinde korunur.

## Ortak çekirdek ve solo

- 6×6: 1–6, 2 satır × 3 sütun kutular. 9×9: 1–9, 3×3 kutular. 0 boş hücredir; aynı satır, sütun veya kutuda rakam tekrarlanmaz. Verilen hücre değişmez.
- Çakışmasız kısmi tahta mutlaka çözülebilir değildir. Tamamlanma tüm hücrelerin dolu ve kısıtların geçerli olmasını gerektirir. Sunulan üretilmiş bulmacaların tek çözümü ayrıca aranarak denetlenir.
- Ortak Rust çekirdek, native için UniFFI ve web için WASM kullanılır. Native UI'ler kuralları yeniden yazmaz; çekirdeğin durum ve sonuçlarını gösterir.
- Solo ağ olmadan yeni oyun ve devam sunar. Web ilk başarılı yüklemeden sonra kendi uygulama ve WASM varlıklarını saklar; ilk ziyarette ağ gerekebilir. Aynı boyutta yeni oyun eski kaydın yerini alacaksa açık onay gerekir.
- Solo hatalı oyuncu rakamını tutar, çakışmaları gösterir. Doğrudan çözümle karşılaştırarak hata sayısı veya üç hatada bitiş yoktur. Yerel süre duraklatmada ve arka planda durur. Canlı mod durmaz.
- Notlar elle değiştirilir. Rakam yazmak yalnız o hücrenin notlarını siler; eş hücre notları otomatik silinmez. Sil hem rakamı hem notları temizler. Undo/redo bütün bu değişikliği geri alır. No-op/ret geçmişi bozmaz; yeni değişiklik redo dalını siler.
- Sürüm 1 JSON kayıt tahta, verilenler, notlar ve undo/redo taşır. Yükleme en fazla 1 MiB; toplam geçmiş 4096 işlem, daha eskiler atılır. Bunlar kaynak koruma varsayılanlarıdır. Süre/pause gibi sunum bilgileri platform zarfındadır; bozuk veri sessizce ezilmez ve başarısız kayıtta başarı söylenmez.
- İpucu cevap aramasının sonucunu göstermeden görünür adaylardan açık tekli veya birim içindeki gizli tekliyi açıklar. Hücreyi otomatik kilitlemez; kullanıcı uygularsa sıradan, geri alınabilir bir giriş olur. Çelişkili/çözümsüz durumda ipucu verilmez. Dışarıdan gelen tahtalarda arama 50.000 düğümle sınırlıdır; bütçe aşımı `SearchLimit` olarak ayrılır ve “çözümsüz” sayılmaz. Bu teknikler yetmiyorsa “bu tekniklerle ipucu bulunamadı” denir.
- Üretim tohumu aynı boyut/boş sayısıyla dört çalışma ortamında aynı bulmacayı vermelidir. Boş sayısı zorluk değildir. Çekirdek desteklenen tekniklerin adım sayılarını raporlayabilir; insan seviyesi derecelendirmesi, ileri teknikler ve üretim bantları henüz kalibre edilmemiştir.

## Oda, gizlilik ve zaman

İki misafir, oda kodu/davetle katılır. Sunucu tahmin edilemeyen koltuk yetkisi verir; kod mevcut koltuğu devralma yetkisi değildir. Oda doluysa yeni katılım reddedilir. Aynı koltukla yeni bağlantı nesli eski bağlantının işlenmemiş komutlarını geçersiz kılar.

İki taraf hazır olunca sunucu başlangıcı ve 3 saniyelik geri sayımı belirler. Başlamadan hamle uygulanmaz. Her oyuncunun tahtası ayrıdır; çözüm, rakibin hücreleri/notları/seri ayrıntısı ve koltuk yetkisi rakibe gönderilmez. Yarışta yalnız dolu hücre oranı, düelloda skor gösterilir.

Bütün bağlayıcı zamanlar sunucunun monoton saatidir. Yetkili giriş noktası geliş zamanı ve oda sırası verir; istemci damgası sonuç belirlemez. Aynı damgada sıra belirleyicidir. Önceden kabul edilmiş girişler sırayla tamamlanmadan süre sonucu kesinleşmez. İstemci kalan süreyi sunucudan kestirir, kesin sonucu üretmez.

## Klasik yarış

- Süre sınırı yoktur. Kendi tahtasında not/silme/undo/redo serbest, ipucu kapalıdır. Hücre doğruluğu açıklanmaz; görünür çakışmalar hesaplanabilir.
- Tam ve değişmiş tahta sunucuda hemen doğrulanır; doğruysa ilk kabul edilen bitiş kazanır. 100 ms beraberlik penceresi ve 10 saniyelik doğrulama kilidi yoktur. Yanlışsa yalnız “tahta doğru değil” denir. Aynı tahta sürümü yeniden sonuç üretmez.
- Kopma ilanından sonra 120 saniyede dönüş gerekir. Kesin son anda süre dolması önce uygulanır. İki kopmanın süreleri eşitse sonuçsuz; farklıysa ilk dolan kaybeder. Bağlı oyuncu hareketsiz diye kaybetmez.
- Kopukken yerel çözüm devam edebilir. Dönüşte son onaylı sürüme dayalı tek tahta sync gönderilir; kabul anı bitiş zamanıdır. 40 yerel düzenleme tek eşitlemedir; onaysız değişiklikler sessizce silinmez.

## 90 saniye düello

Aynı başlangıç ve aynı bulmaca dizisi, ayrı oyuncu ilerlemesi vardır. Boş hücreye doğru rakam sunucuda kabul edilip hücre kilitlenir. Yanlış rakam tahtaya yazılmaz. İpucu, not, silme, undo/redo yoktur. Sunucuya T0+90.000 ms'den **önce** giren geçerli komutlar işlenir; o anda/sonrasında girenler reddedilir. İşleme gecikmesi, önceden varmış geçerli hamleyi düşürmez.

| Son yanlıştan beri doğru seri (mevcut hamle dahil) | Hamle puanı |
| --- | --- |
| 1–3 | 10 |
| 4–6 | 15 |
| 7–9 | 20 |
| 10+ | 25 |

Yanlış: −5 puan, alt sınır 0; seri sıfırlanır, 2000 ms giriş kilidi. Kilitteki, geçersiz hücredeki, hız sınırındaki veya süre sonrasındaki ret ayrıca puan/yanlış sayısı değiştirmez. Yeni hamle denemesi doğruluk sorgusundan önce sınırlandırılır. Tamamlama +50 puanı tahta başına yalnız bir kez verir; sonraki tahta ve puan tek işlemde uygulanır. Seri tahtalar arasında sürer. Kural parametreleri iki boyutta aynıdır; üretim örnekleri 6×6 için 18, 9×9 için 43 boş hücre kullanır, “kolay” olduğu varsayılmaz.

Süre sonunda yüksek skor; eşitse az cezalı yanlış; o da eşitse beraberlik. Kopmada saat sürer ve skor korunur; yeniden bağlanmada eski bağlantıdan birikmiş hamleler uygulanmaz. Pes/ayrılma skordan bağımsız yenilgidir.

| Örnek | Sonuç |
| --- | --- |
| 5 doğru | 10+10+10+15+15 = 60 |
| 12 doğru | 30+45+60+75 = 210 |
| 4 doğru, yanlış, 2 doğru | 45−5+10+10 = 60 |
| Yanlış, 1.2 sn sonra deneme, kilit sonrası 2 doğru | 0+ret+10+10 = 20 |
| 2 doğru, yanlış, kilit sonrası yanlış, kilit sonrası doğru | 20−5−5+10 = 20 |
| Doğru, 3 ayrı kilit sonrası yanlış | 10−5−5−0 = 0 |
| 18 doğruyla tahta biter, sonraki tahtada 1 doğru | 360+50+25 = 435 |
| 89.999 ms giriş / 90.000 ms giriş | İlki değerlendirilebilir / ikincisi reddedilir |

## Tekrarlar, sınırlar ve arıza

- `(maç, koltuk, sıra)` aynı içerikle tekrarlandığında saklanmış kabul/ret sonucu döner; puan, ceza veya hız kotası yeniden tüketilmez. Aynı sırada başka içerik reddedilir. Sıra boşluğunda eylem uygulanmaz, eşitleme gerekir. Rövanş yeni maç kimliği; her tahta ayrı tahta kimliği taşır.
- Önerilen hamle kotası oyuncu başına kayan 1000 ms'de 6 yeni denemedir; tek yarış sync'i ayrı bir komuttur. Hız, oda ve saklama sınırlarının uygulanan kesin değerleri sürümlü maç protokolünde belgelenir. Teknik sınır sonucu başarı veya rakibe haksız galibiyet olarak gösterilmez.
- Sunucu arızasında güvenilir biçimde korunamayan maç sonuçsuzdur; eksik olay sırası, işlem sonuçları veya kesinleşmiş sonuçla yarış sürdürülmez. İlk yerel sunucu kalıcı kurtarma sunmayabilir; bunu istemciye açıkça bildirir. Üretim dayanıklılığı kanıtlanmış sayılmaz.
- Rövanş iki oyuncunun talebiyle yeni bulmacada başlar. Önceki maç paketleri yeni maçta geçerli değildir.
- Bulmacayı programla çözme ve klavye/dokunmatik hız farkı bütünüyle engellenemez. Kurallar bunun önlendiğini veya rastgele tahminin daima zararlı olduğunu iddia etmez. Gerçek oyuncularla süre, puan ve yöntem farkı G17'de değerlendirilir.

## Kanıt ve açık konular

G01 inceleme gerekçesi vardır; insan oyun dengesi testi yoktur. G04, G06 ve istemci testleri uygulama kurallarını kanıtlamalıdır; belge tek başına tamamlanma kanıtı değildir. İnsan zorluk dereceleri, ileri ipucu teknikleri, mağaza fiyatı/reklam, alan adı/barındırma ve imzalama kararları henüz verilmiş sayılmaz. Temel öğe/durumlar `ui-contract.json`, platform kabul matrisi `acceptance-scenarios.md`, güncel durum `PROGRESS.md` içindedir.
