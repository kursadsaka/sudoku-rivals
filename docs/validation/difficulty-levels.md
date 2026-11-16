# Zorluk seviyeleri: teknik tabanlı beş kademe, üretim ve ipuçları

16 Kasım 2026 (beş kademe: aynı gün ikinci tur). Kapsam: ortak Rust çekirdeği (`core/src/difficulty.rs`, `core/src/expert6.rs`), WASM ve UniFFI bağlamalarına **eklenen** üç fonksiyon, sunucunun isteğe bağlı oda zorluğu, web solo ve arkadaş kurulumu. Mevcut `generate_puzzle`, kayıt biçimi ve seviyesiz odalar değişmedi. Android ve iOS kaynaklarına dokunulmadı.

## Seviyeler ne demek

Seviye boş hücre sayısından değil, bulmacanın **hangi çözüm tekniğini gerektirdiğinden** çıkar. Çekirdekte yalnız adlandırılmış insan tekniklerini kullanan bir mantık çözücüsü vardır; bir bulmacanın seviyesi, onu bitirebilen en düşük teknik kümesidir. Her kademe bir altındakinden **bir teknik fazlasını zorunlu** kılar; aynı bulmaca iki kademeye birden giremez.

| Kademe (ad · tel adı) | Yeni gereken teknik | Ölçüt (kesin) |
| --- | --- | --- |
| Başlangıç / Beginner · `beginner` | Kutuya bakmak: sayının kutuda tek yeri; ya da bir satır/sütun/kutunun son boş hücresi | Yalnız bunlarla sonuna kadar çözülür |
| Kolay / Easy · `easy` | Satır veya sütunda gizli tekli (sayının o satır/sütunda tek yeri) | Kutular yetmez; satır/sütun teklileri eklenince çözülür |
| Orta / Medium · `medium` | Çıplak tekli (hücrede adaylar sayılınca tek sayı kalır) | Gizli tekliler yetmez; çıplak tekli eklenince çözülür |
| Zor / Hard · `hard` | Kilitli adaylar (işaret etme / sahiplenme) | Tekliler yetmez; kilitli adaylar eklenince çözülür, çift gerekmez |
| Uzman / Expert · `expert` | Çıplak veya gizli çift | Kilitli adaylar da yetmez; çiftler eklenince çözülür |

Sıralama Sudoku Explainer tarzı yaygın teknik sıralamasıyla aynı yöndedir (kutu teklisi < satır/sütun teklisi < çıplak tekli < kilitli adaylar < çiftler). Desteklenen tekniklerle bitmeyen tahtalar **derecelendirilmez** (`level: null`); etiket uydurulmaz. İnsanlarla kalibrasyon yapılmadı: seviyeler "bu teknik gerekir" beyanıdır, "şu kadar dakikada çözülür" beyanı değildir.

Mantık çözücüsünün attığı her adım zorunlu olduğundan, onun bitirdiği bulmacanın **tek çözümü** vardır ve tahmin gerekmez. Testler bunu ayrıca arama tabanlı `count_solutions == 1` ile doğrular.

Önceki üç kademeyle ilişki: eski Kolay (yalnız gizli tekli) artık Başlangıç ve Kolay'a, eski Zor (kilitli aday veya çift) Zor ve Uzman'a ayrıldı; Orta aynıdır. `easy`, `medium`, `hard` adları geçerli kalır, yalnız kapsamları daraldı. Seviye kayıtta saklanmadığı için eski kayıtların etiketi yeni ölçüte göre yeniden hesaplanır.

## Üretim

`generate_rated_puzzle(size, seed, difficulty)`:

1. Tohumdan rastgele dolu bir ızgara üretilir (en kısıtlı hücreye karıştırılmış adaylarla arama).
2. **Başlangıç:** hücreler, bulmaca yalnız kutu teklileriyle çözülebilir kaldıkça çıkarılır; 6×6'da 17, 9×9'da 38 boşta durur.
3. **9×9 Kolay–Uzman ve 6×6 Kolay:** önce bir alt kademenin teknikleriyle belli derinliğe kazılır. Sonra "alt kademenin çözemediği ama bu kademenin çözdüğü" çıkarma aranır; bulmacayı o kademeye taşıyan tam olarak bu çıkarmadır. Yoksa bir hücre daha kazılıp tekrar bakılır.
4. **6×6 Orta ve Zor:** bu boyutta tekli ötesi teknik yalnız seyrek bulmacalarda gerekir. Kademenin teknikleri izin verdikçe kazılır, alt kademe artık çözemiyorsa kabul edilir (genelde 24–28 boş, yani 8–12 ipucu).
5. **6×6 Uzman: havuz + simetri.** Çift gerektiren 6×6 bulmaca yaklaşık 5.000 kazıda bir çıkar; anlık arama ölçümde ortalama 0,77 sn, p99 3,3 sn sürdü (150 tohum, masaüstü). Telefonda ve sunucuda bu kabul edilemez olduğu için bu tek kademe aranmaz: aramanın bulduğu, her biri Uzman olarak derecelendirilmiş ve tek çözümlü 64 gerçek bulmaca çekirdeğe gömülüdür. Tohum birini seçer ve yeniden düzenler: rakamlar yeniden adlandırılır, satırlar üç bandın içinde ve bantlar arasında, sütunlar iki yığının içinde ve yığınlar arasında yer değiştirir. Bu hareketler her satır, sütun ve kutuyu yine satır, sütun ve kutu bıraktığı için bulmaca **tam olarak aynı teknikleri** gerektirir; taban başına yaklaşık 2,5 milyon biçim, toplam ~160 milyon bulmaca verir. Kısıt: bu kademedeki çeşitlilik 64 taban yapıdan türer.
6. Tutmayan deneme türetilmiş tohumla yinelenir; deneme tohumu, tohum ile deneme sayısı karıştırılarak (hash) elde edilir. Böylece yakın tohumlar birbirinin denemelerine girmez (sunucu düelloda tahta başına bir tohum türetir). Aynı tohum her platformda aynı bulmacayı verir (yalnız tam sayı aritmetiği; `vectors/boards.json` içindeki `rated` vektörleri Rust ve WASM'da karşılaştırılır).

Boş sayısı pencereleri yalnız bir kademenin bulmacalarını benzer uzunlukta tutar; kademeyi belirlemez.

### Ölçüm (`calibration_report`, sürüm derlemesi, kademe başına 1000 tohum)

| Boyut · kademe | Başarısız | Ortalama | p99 | En yavaş | Boş (min–ortanca–maks) | En zor teknik |
| --- | --- | --- | --- | --- | --- | --- |
| 6×6 Başlangıç | 0 | 0,10 ms | 0,17 ms | 0,20 ms | 17 | kutu teklisi |
| 6×6 Kolay | 0 | 1,8 ms | 7,5 ms | 18 ms | 19–24–26 | satır/sütun teklisi |
| 6×6 Orta | 0 | 14 ms | 60 ms | 93 ms | 24–26–28 | çıplak tekli |
| 6×6 Zor | 0 | 56 ms | 250 ms | 340 ms | 23–26–28 | kilitli aday |
| 6×6 Uzman (havuz) | 0 | 0,07 ms | 0,10 ms | 0,14 ms | 24–26–27 | çıplak çift 884, gizli çift 116 |
| 9×9 Başlangıç | 0 | 0,60 ms | 0,77 ms | 1,3 ms | 38 | kutu teklisi |
| 9×9 Kolay | 0 | 4,4 ms | 15 ms | 24 ms | 41–45–50 | satır/sütun teklisi |
| 9×9 Orta | 0 | 9,2 ms | 39 ms | 58 ms | 45–50–54 | çıplak tekli |
| 9×9 Zor | 0 | 23 ms | 113 ms | 145 ms | 48–52–58 | kilitli aday |
| 9×9 Uzman | 0 | 90 ms | 404 ms | 648 ms | 51–53–59 | çıplak çift 757, gizli çift 243 |

WASM'da (Node 26, masaüstü; 300 tohum) ortanca / p99: 6×6 Orta 10 / 81 ms, 6×6 Zor 43 / 258 ms, 9×9 Orta 8 / 44 ms, 9×9 Zor 19 / 126 ms, 9×9 Uzman 65 / 395 ms (en uzun 541 ms); diğerleri 20 ms altı, 6×6 Uzman 1 ms altı. Telefonda birkaç kat yavaş olması beklenir; bu yüzden web başlat düğmesi üretimden önce "Bulmaca hazırlanıyor…" durumuna geçer.

Deneme sınırı 6×6'da 12.000, 9×9'da 400'dür; sınır aşılırsa `CannotReachDifficulty` döner (ölçümde görülmedi) ve web başka bir tohumla üç kez daha dener.

## İpuçları

`find_technique_hint` görünen rakamlardan sıradaki yerleştirmeyi bulur: önce kutu teklisi, sonra satır/sütun teklisi, sonra çıplak tekli. Tekli yoksa elemeleri uygular ve tekli çıkınca **yalnız o yerleştirmenin dayandığı** elemeleri (geriye doğru bağımlılık izleyerek) bildirir: teknik, sayı(lar), desenin görüldüğü birim, kilitli adayda sayının silindiği birim ve desen hücreleri. Derecelendirilmiş her bulmacanın her adımı bu ipuçlarıyla açıklanabilir; testler beş kademeyi iki boyutta yalnız ipuçlarını izleyerek sonuna kadar çözer ve her kademede beklenen tekniğin gerçekten kullanıldığını, bir üst kademenin tekniğinin kullanılmadığını doğrular. Çözümden rakam sızdırılmaz; çelişkili veya çözümsüz tahta hata döner.

Web ipucu kartı her elemeyi bir cümleyle, ardından yerleştirmeyi yazar ve desen hücrelerini tahtada çerçeveler. Önceki `solo_hint` (yalnız tekliler) aynen durur.

## Uyumluluk

- **Kayıtlar:** seviye kaydedilmez; `solo_difficulty` onu kaydın ipuçlarından (givens) her seferinde türetir. `SoloGame` kayıt alanları aynıdır (testte alan listesi sabitlenmiştir). Eski kayıtlar ve boş sayısıyla üretilmiş bulmacalar açılır; teknikler yetiyorsa seviye gösterilir, yetmiyorsa etiket gösterilmez. Web tercihlerinde saklı `easy`/`medium`/`hard` değerleri geçerli kalır.
- **Çok oyunculu:** oda kurulurken isteğe bağlı `difficulty` artık beş değerden biri olabilir; seçilirse yarışın bulmacası, düellonun bütün tahtaları ve rövanşlar o kademede üretilir ve iki oyuncuya aynı tahta verilir ([match-protocol.md](../match-protocol.md#oda-zorluğu)). Seçilmezse sunucu `generate_puzzle(size, seed, blanks)` kullanmayı sürdürür; o fonksiyon ve tohum vektörleri değişmedi.
- **Bilinmeyen kademe:** `Snapshot.difficulty` okunurken bu derlemenin tanımadığı bir ad "seviye yok" sayılır; oda oynanabilir kalır, yalnız etiket gösterilmez. Böylece ileride kademe eklemek eski istemcileri bozmaz. (Bu tolerans bu commit'le geldi; ondan önceki üç kademeli web derlemesi `beginner`/`expert` odasının durumunu okuyamaz. Web istemcisi sunucuyla aynı origin'den yüklendiği ve önbellek sürümü yükseltildiği için sayfa yenilenince güncellenir.)
- **Android / iOS:** UniFFI'ye yalnız ekleme yapıldı: `new_rated_solo`, `solo_difficulty`, `solo_technique_hint` (beş kademe adını kabul eder). Mevcut fonksiyonların imzası ve JSON biçimi aynı; iki uygulama `new_generated_solo(…, 18/43)` ve `solo_hint` ile olduğu gibi derlenip çalışmaya devam etmelidir. Yerel uygulamalar oda kurarken seviye göndermez (seviyesiz oda), beş kademenin herhangi biriyle kurulmuş odaya katılabilir ve bulmacayı normal oynar ama seviyeyi göstermez. Yerel uygulamalar bu turda derlenmedi ve cihazda denenmedi; kademe seçimi onlarda yoktur.
- `PuzzleError` içine `CannotReachDifficulty` eklendi; hata metni olarak taşınır, eşleştiren kod yoktur.

## Doğrulama

- `cargo test --workspace` (çekirdek, FFI, sunucu): geçti. `core/tests/difficulty.rs`: istenen kademe = derecelendirilen kademe, tek çözüm, belirlenimcilik, her adımın ipucuyla açıklanması, kademeye özgü teknik var / üst kademenin tekniği yok, 6×6 Uzman havuzunun 64 girdisinin gerçekliği ve 400 tohumda yeniden düzenlemelerin kademeyi koruması, derecelendirilemeyen/çelişkili tahtalar, kayıt alanları, ortak vektörler.
- `server/tests/difficulty.rs`: iki mod × iki boyut × beş kademede iki koltuğa aynı ve doğru kademede bulmaca; düello sonraki tahtaları (6×6 Uzman dahil), rövanş, lobiye dönüş, seviyesiz odalar, kayıp create tekrarı, bilinmeyen kademe adının "seviye yok" okunması.
- `node tests/wasm.mjs`: beş kademenin `rated` vektörleri yerel üretimle aynı, derece ve uçtan uca ipucu yolu WASM'da da geçiyor.
- Tarayıcı (başsız Chromium): on boyut·kademe kombinasyonu yalnız "İpucu → Bu sayıyı yaz" ile sonuca kadar çözüldü; Zor ve Uzman'da eleme cümlesi ve tahtada desen işareti görüldü; beş kademe 320 px'te tek satıra sığıyor (her seçenek ≥48 px); seçim yeniden yüklemede korunuyor. İki gerçek oyuncuyla düello 6×6 Orta ve Uzman, yarış 9×9 Zor ve Başlangıç: iki ekranda aynı bulmaca, motor odanın kademesinde derecelendirdi.

## Sınırlar

- İnsan kalibrasyonu yok; kademe adları teknik ölçütün kısaltmasıdır. Başlangıç ile Kolay arasındaki fark (kutu ↔ satır/sütun taraması) gerçek ve ölçülebilirdir ama deneyimli oyuncuya küçük gelebilir.
- Uzman'ın tavanı çiftlerdir: üçlüler, X-Wing ve ötesi yoktur; bunları gerektiren bulmaca üretilmez.
- 6×6 Uzman 64 taban bulmacadan simetriyle türetilir; 6×6 Orta, Zor ve Uzman zorunlu olarak seyrektir (8–12 ipucu).
- Arkadaş maçında seviyeli tahtalar sunucuda, maçın işlendiği iş parçacığında üretilir: 9×9 Uzman ortalama ~90 ms, en kötü ~0,65 sn; 6×6 Zor ortalama ~56 ms, en kötü ~0,34 sn (yerel ölçüm). Zayıf bir staging işlemcisinde daha uzun sürebilir ve o an diğer istekler bekler. Düelloda sonraki tahta, bir oyuncu tahtayı bitirdiği hamlede üretilir.
- Yerel uygulamalar oda kurarken seviye göndermez; seviyeli odaya katılabilir ama seviyeyi göstermez.
- Yerel uygulamalar ve gerçek cihazda üretim süresi ölçülmedi.
