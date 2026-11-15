# Zorluk seviyeleri: teknik tabanlı derecelendirme, üretim ve ipuçları

16 Kasım 2026. Kapsam: ortak Rust çekirdeği (`core/src/difficulty.rs`), WASM ve UniFFI bağlamalarına **eklenen** üç fonksiyon, web solo kurulumu. Çok oyunculu kurallar, sunucu, mevcut `generate_puzzle` ve kayıt biçimi değişmedi. Android ve iOS kaynaklarına dokunulmadı.

## Seviyeler ne demek

Seviye boş hücre sayısından değil, bulmacanın **hangi çözüm tekniğini gerektirdiğinden** çıkar. Çekirdekte yalnız adlandırılmış insan tekniklerini kullanan bir mantık çözücüsü vardır; bir bulmacanın seviyesi, onu bitirebilen en düşük teknik kümesidir.

| Seviye | Gereken | Ölçüt (kesin) |
| --- | --- | --- |
| Kolay (`easy`) | Yalnız gizli tekli: bir sayının satır, sütun veya kutuda girebileceği tek yer | Yalnız gizli teklilerle sonuna kadar çözülür |
| Orta (`medium`) | En az bir çıplak tekli: hücrede adaylar sayılınca tek sayı kalır | Gizli tekliler yetmez; gizli + çıplak teklilerle çözülür |
| Zor (`hard`) | Aday eleme: kilitli adaylar (işaret etme/sahiplenme), çıplak çift, gizli çift | Tekliler yetmez; tekliler + bu elemelerle çözülür |

Sıralama Sudoku Explainer tarzı yaygın teknik sıralamasıyla aynı yöndedir (gizli tekli < çıplak tekli < kilitli adaylar < çiftler). Desteklenen tekniklerle bitmeyen tahtalar **derecelendirilmez** (`level: null`); "uzman" gibi bir etiket uydurulmaz. İnsanlarla kalibrasyon yapılmadı: seviyeler "bu teknikler gerekir" beyanıdır, "şu kadar dakikada çözülür" beyanı değildir.

Mantık çözücüsünün attığı her adım zorunlu olduğundan, onun bitirdiği bulmacanın **tek çözümü** vardır ve tahmin gerekmez. Testler bunu ayrıca arama tabanlı `count_solutions == 1` ile doğrular.

## Üretim

`generate_rated_puzzle(size, seed, difficulty)`:

1. Tohumdan rastgele dolu bir ızgara üretilir (en kısıtlı hücreye karıştırılmış adaylarla arama). Eski üreticinin permütasyon deseni yerine bu kullanılır; 6×6'da üst seviyeler o desende neredeyse hiç çıkmıyordu.
2. **Kolay:** hücreler, bulmaca gizli teklilerle çözülebilir kaldıkça çıkarılır; 6×6'da 19, 9×9'da 42 boşta durur.
3. **9×9 Orta/Zor:** önce bir alt seviyenin teknikleriyle belli derinliğe kazılır (44 / 46 boş). Sonra "alt seviyenin çözemediği ama bu seviyenin çözdüğü" çıkarma aranır; bulmacayı o seviyeye taşıyan tam olarak bu çıkarmadır. Yoksa bir hücre daha kazılıp tekrar bakılır (üst sınır 54 / 58 boş).
4. **6×6 Orta/Zor:** bu boyutta tekli ötesi teknik yalnız seyrek bulmacalarda gerekir. Seviyenin teknikleri izin verdikçe kazılır, alt seviye artık çözemiyorsa kabul edilir (genelde 24–28 boş, yani 8–12 ipucu).
5. Tutmayan deneme türetilmiş tohumla yinelenir. Aynı tohum her platformda aynı bulmacayı verir (yalnız tam sayı aritmetiği; `vectors/boards.json` içindeki `rated` vektörleri Rust ve WASM'da karşılaştırılır).

Boş sayısı pencereleri yalnız bir seviyenin bulmacalarını benzer uzunlukta tutar; seviyeyi belirlemez.

### Ölçüm (`calibration_report`, sürüm derlemesi, seviye başına 2000 tohum)

| Boyut · seviye | Başarısız | Ortalama | En yavaş | Boş (min–ortanca–maks) | En zor teknik dağılımı |
| --- | --- | --- | --- | --- | --- |
| 6×6 Kolay | 0 | 0,12 ms | 0,45 ms | 19 | gizli tekli |
| 6×6 Orta | 0 | 14 ms | 110 ms | 24–26–28 | çıplak tekli |
| 6×6 Zor | 0 | 71 ms | 478 ms | 23–26–28 | kilitli aday 1807, çıplak çift 170, gizli çift 23 |
| 9×9 Kolay | 0 | 0,7 ms | 1,1 ms | 42 | gizli tekli |
| 9×9 Orta | 0 | 10 ms | 80 ms | 45–50–54 | çıplak tekli |
| 9×9 Zor | 0 | 25 ms | 191 ms | 47–52–58 | kilitli aday 1534, çıplak çift 353, gizli çift 113 |

WASM'da (Node 26, masaüstü; 300 tohum) ortanca / p99: 6×6 Orta 12 / 85 ms, 6×6 Zor 54 / 387 ms (en uzun 538 ms), 9×9 Orta 8 / 44 ms, 9×9 Zor 16 / 128 ms; Kolay 1 ms altı. Telefonda birkaç kat yavaş olması beklenir; bu yüzden web başlat düğmesi üretimden önce "Bulmaca hazırlanıyor…" durumuna geçer.

6×6 üst seviyeleri seyrektir: yaklaşık 70 kazıdan biri çıplak tekli, 470 kazıdan biri eleme gerektirir. Deneme sınırı 6×6'da 12.000, 9×9'da 400'dür; sınır aşılırsa `CannotReachDifficulty` döner (2000 tohumda görülmedi) ve web başka bir tohumla üç kez daha dener.

## İpuçları

`find_technique_hint` görünen rakamlardan sıradaki yerleştirmeyi bulur. Tekli yoksa elemeleri uygular ve tekli çıkınca **yalnız o yerleştirmenin dayandığı** elemeleri (geriye doğru bağımlılık izleyerek) bildirir: teknik, sayı(lar), desenin görüldüğü birim, kilitli adayda sayının silindiği birim ve desen hücreleri. Derecelendirilmiş her bulmacanın her adımı bu ipuçlarıyla açıklanabilir; testler üç seviyeyi iki boyutta yalnız ipuçlarını izleyerek sonuna kadar çözer. Çözümden rakam sızdırılmaz; çelişkili veya çözümsüz tahta hata döner.

Web ipucu kartı her elemeyi bir cümleyle, ardından yerleştirmeyi yazar ve desen hücrelerini tahtada çerçeveler. Önceki `solo_hint` (yalnız tekliler) aynen durur.

## Uyumluluk

- **Kayıtlar:** seviye kaydedilmez; `solo_difficulty` onu kaydın ipuçlarından (givens) her seferinde türetir. `SoloGame` kayıt alanları aynıdır (testte alan listesi sabitlenmiştir). Eski kayıtlar ve boş sayısıyla üretilmiş bulmacalar açılır; teknikler yetiyorsa seviye gösterilir, yetmiyorsa etiket gösterilmez.
- **Çok oyunculu:** sunucu `generate_puzzle(size, seed, blanks)` kullanmayı sürdürür; fonksiyon ve tohum vektörleri değişmedi (ızgara kurulumu yardımcı fonksiyona taşındı, çıktı aynı).
- **Android / iOS:** UniFFI'ye yalnız ekleme yapıldı: `new_rated_solo`, `solo_difficulty`, `solo_technique_hint`. Mevcut fonksiyonların imzası, JSON biçimi ve sağlama toplamları aynı; iki uygulama `new_generated_solo(…, 18/43)` ve `solo_hint` ile olduğu gibi derlenip çalışmaya devam etmelidir. Yerel uygulamalar bu turda derlenmedi; seviye seçimi onlarda yoktur. Benimsemek için: kurulum ekranına seviye seçimi + `new_rated_solo`, başlıkta `solo_difficulty`, ipucunda `solo_technique_hint` (yeni `eliminations` alanı ve `technique` değerleri için metinler).
- `PuzzleError` içine `CannotReachDifficulty` eklendi; hata metni olarak taşınır, eşleştiren kod yoktur.

## Doğrulama

- `cargo test --workspace` (çekirdek, FFI, sunucu): geçti. Yeni `core/tests/difficulty.rs`: seviye = istenen seviye, tek çözüm, belirlenimcilik, her adımın ipucuyla açıklanması, Kolay'da yalnız gizli tekli, Orta'da eleme yok ve çıplak tekli var, Zor'da en az bir eleme; derecelendirilemeyen/çelişkili tahtalar; kayıt alanları; ortak vektörler.
- `node tests/wasm.mjs`: `rated` vektörleri yerel üretimle aynı, derece ve uçtan uca ipucu yolu WASM'da da geçiyor.
- Tarayıcı (başsız Chromium): beş boyut·seviye kombinasyonu yalnız "İpucu → Bu sayıyı yaz" ile sonuca kadar çözüldü; Zor'da eleme cümlesi ve tahtada desen işareti görüldü; seçilen seviye yeniden yüklemede korunuyor; ana ekran kartı ve sonuç ekranı seviyeyi gösteriyor.

## Sınırlar

- İnsan kalibrasyonu yok; seviye adları teknik ölçütün kısaltmasıdır.
- Zor'un tavanı çiftlerdir: üçlüler, X-Wing ve ötesi yoktur; bunları gerektiren bulmaca üretilmez.
- 6×6 Orta/Zor bulmacaları zorunlu olarak seyrektir (8–12 ipucu) ve üretimi en yavaş olanlardır.
- Çok oyunculu maçlar seviye seçmez; eski boş sayısı üretimiyle devam eder.
- Yerel uygulamalar ve gerçek cihazda üretim süresi ölçülmedi.
