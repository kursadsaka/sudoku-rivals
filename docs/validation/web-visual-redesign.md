# Web görsel yenileme ve isteğe bağlı sesler

Kapsam: yalnız `apps/web` ve onu sunan allowlist/paket listeleri (`server/src/http.rs`, `scripts/staging-build.sh`, `apps/web/sw.js`). iOS ve Android dosyalarına dokunulmadı. Üç mod, 6×6/9×9, tüm `data-testid` kimlikleri, klavye kısayolları, ekran okuyucu etiketleri ve `docs/ui-contract.md` içindeki dar ekran sırası (gezinme → durum → tahta → sayılar → araçlar) korunur. [Tek ekran yerleşimi notu](web-game-screen-layout.md) geçerliliğini korur; aşağıdakiler onun üzerine gelir.

## Görsel dil

- **Kâğıt ve mürekkep**: sıcak kâğıt zemin, basılı (siyah) verilen sayılar, tükenmez mavisi oyuncu girişleri, kurşun kalem gri notlar. Marka mavisi uygulama simgesiyle aynıdır (`#1d4ed8`). **Kehribar her yerde rakibi** gösterir (yarış çubuğu, düello puanı, lobi, sonuç). Koyu temada nötr koyu zemin kullanılır. Yazı tipi sistem yığınıdır; sayılar ve başlıklarda varsa `ui-rounded`. Harici font/görsel yok.
- **Ana ekran**: iki mod kartı (solo: tek oyuncu simgesi; arkadaş: mavi "sen" + kehribar "rakip" iki oyuncu), "Kaldığın yerden" kartlarında boyut rozeti. Alt bilgide çevrimdışı durumu simgeyle gösterilir (hazır: yeşil onay; hazırlanıyor: indirme; kullanılamıyor: üstü çizili bulut); "6×6 & 9×9" yazısı kaldırıldı.
- **Oyun ekranı**: üstte geri · `mod · boyut` + büyük süre · duraklat. Altında ince bir **gösterge**: solo'da doluluk, yarışta `Sen %` ↔ `% Rakip` karşılıklı iki çubuk, düelloda puanlar + 90 saniyelik kalan süre çubuğu (son 10 saniyede kırmızı). Seçili hücre dolu marka rengi; aynı sayı, ilgili satır/sütun/kutu ve çakışma ayrı tonlardadır (çakışmada ayrıca `!` ve "Çakışma var" metni).
- **Sayı tuşları**: her tuşta o rakamdan kaç tane eksik olduğu yazar (ekran okuyucuda gizli); biten rakam soluklaşır. Kalem modunda tuşlar kesik çizgili ve gri olur. Yüksekliği ≥790 px olan dikey telefonlarda tuşlar hesap makinesi düzeninde 3 sütundur (boşluk yerine büyük tuş); daha kısa ekranlarda 9×9 `5+4` kalır. 6×6 dikey telefonda her zaman `3+3` iki sıradır (9×9 ile aynı yükseklik; ipucu kartına yer bırakır).
- **Arkadaş kurulumu**: üstte "Oda kur | Katıl" sekmesi. Oda kur: ad, açıklamalı iki mod kartı, boyut. Katıl: büyük kod alanı, ad ve "Katıl" düğmesi. Davet bağlantısı (`/r/KOD`) doğrudan Katıl sekmesini kod dolu açar; 320×568, 375×553 ve 393×659'da kod, ad ve Katıl düğmesi kaydırmadan görünür. Yapıştırılan bağlantı alan terk edilince koda indirgenir; ad/kod alanında Enter formu gönderir.
- **Lobi/sonuç**: oda kodu harf harf karolarda, oyuncu satırlarında baş harf ve "Hazır/Bekliyor" rozeti; düello sonucu iki sütunlu skor.
- **Ayarlar**: tek kartta dil, ses, hareketi azalt, titreşim (bu sürümde kapalı) anahtarları.

## İletiler tahtayı ve tuşları oynatmaz

Kullanıcı geri bildirimi: İpucu'na basınca alttan çıkan ileti tahtayı küçültüp tuşları yukarı itiyordu. Artık oyun sırasında beliren hiçbir geçici içerik akışa girmez:

- **İpucu kartı** sayı tuşları + araçlar alanının üstüne (`.input-keys` içinde mutlak konumlu) oturur; tahta tümüyle görünür kalır ve ipucu hücresi seçilidir. Kart tuş alanından en çok 6 px taşabilir, sığmayan metin kart içinde kayar. "Bu sayıyı yaz" ve "Kapat" 44 px; odak birincil eyleme taşınır, açıklama canlı bölgeden okunur, `Esc` veya başka hücre seçimi kapatır ve odağı tahtaya döndürür.
- **Anlık iletiler** (ipucu bulunamadı, çakışmayı düzelt, verilen hücre, dolu hücreye not) başlık ve sürenin üstünde, geri ve duraklat düğmelerinin arasında koyu bir şerit olarak görünür; kapatma düğmesi, `Esc` veya sonraki hamle kaldırır.
- **Canlı maç hamle geri bildirimi** (yanlış hamle, tahta henüz doğru değil, çok hızlı giriş) durum satırında tek satır metindir; kilit sayacı görünürken gizlenir. Durum satırı artık sabit yükseklikte ve tek satırdır (taşan metin `…` ile kısalır).
- Akışta kalanlar yalnız kalıcı hata durumlarıdır: kayıt yazılamadı (tekrar dene), sunucu/bağlantı hatası ve yarış uzlaştırma seçimi. Bunlar görünürken tahta küçülebilir.

## Onay sayfası

Tarayıcının `confirm()` penceresi yerine uygulama içi `<dialog>` kullanılır (telefonda alttan sayfa, geniş ekranda ortada): kayıtlı oyunu değiştirme, kaydetmeden çıkma, maçtan ayrılma ("Maçta kal" / "Ayrıl"), canlı kaydı sıfırlama ve sunucu tahtasını kabul etme. Odak pencere içinde kalır; `Esc` ve arka plana dokunma vazgeçer. Ayrılma sorusu açıkken maç bittiyse ayrılma komutu yeniden değerlendirilir. `<dialog>` yoksa `confirm()` yedeği çalışır.

## Varsayılanlar ve hatırlanan seçimler

Seçim yapılmadıysa solo ve arkadaş kurulumunda tahta **9×9**, mod klasik yarıştır. Son seçilen solo boyutu, arkadaş modu/boyutu ve görünen ad `sudoku-rivals.preferences.v1` içinde (`soloSize`, `liveSize`, `liveMode`, `displayName`) saklanır ve sonraki açılışta geri gelir; eski kayıtlarda bu alanlar yoksa varsayılanlar geçerlidir.

## Zorluk seviyeleri

Solo kurulumda boyutun altında tek satırda beş kademe vardır: **Başlangıç / Kolay / Orta / Zor / Uzman** (EN: Beginner / Easy / Medium / Hard / Expert); varsayılan Kolay, son seçim hatırlanır (`soloDifficulty`). Açıklama satırı kullanıcı tarafından sadeleştirildi; adlar kendi başına sıralı bir ölçek olarak okunur. Satır 320 px'te sığar ve her seçenek en az 48 px genişliktedir. Seviyeler çekirdekte tekniğe göre üretilir ve derecelendirilir: [difficulty-levels.md](difficulty-levels.md). Başlat düğmesi üretim sürerken "Bulmaca hazırlanıyor…" olur. Seviye oyun başlığında (`Solo · 9×9 · Orta`), duraklatma/sonuç ekranında ve ana ekrandaki kayıt kartında gösterilir; derecelendirilemeyen eski kayıtlarda etiket yoktur. İpucu kartı Zor bulmacalarda eleme adımlarını da açıklar ve desen hücrelerini tahtada çerçeveler.

### Arkadaş maçında zorluk

"Oda kur" sekmesinde boyutun altında aynı beş kademeli seçim vardır (varsayılan Kolay, `liveDifficulty` olarak hatırlanır); altındaki satır seviyeyi ve "iki oyuncu aynı bulmacayı alır; düellodaki sonraki tahtalar ve rövanş da bu seviyededir" kuralını söyler. Lobide üst satırda (`90 saniye düello · 6×6 · Orta`) ve mavi bilgi kutusunda seviye ile açıklaması **iki oyuncuya da** gösterilir; katılan oyuncu neye girdiğini hazır demeden önce görür. Oyun başlığı (`Düello · 6×6 · Orta`) ve sonuç ekranı seviyeyi taşır. Seviyesiz kurulmuş odalarda (eski istemciler) etiket çizilmez.

### Başlık ile içerik arası boşluk (ortak yerleşim)

Kullanıcı logo satırı ile ekranın ilk öğesi arasındaki boşluğu fazla buldu. Kök neden üç boşluğun üst üste binmesiydi: başlığın alt dolgusu (14 px), `main` üst dolgusu (12 px, masaüstünde 32 px) ve Geri düğmesinin 48 px'lik dokunma kutusunun simgenin üstünde bıraktığı 12 px. Ekran bazlı yamalar yerine tek kural kondu:

- Başlık alt dolgusu 4 px (masaüstünde 8 px); `main` üst dolgusu tek değişken `--content-top` (12 px, masaüstünde 24 px).
- Sayfayı Geri ile açan ekranlarda düğmenin dokunma kutusu 12 px yukarı taşar (`main > section > .back-button:first-child`); kutu 48 px kalır, gözün gördüğü simge diğer sayfaların başladığı hizaya oturur.
- Ana ekran, duraklatma ve sonuç bölümlerinin kendi üst boşlukları kaldırıldı.

Ölçüm (logo altı → ilk görünür öğe): ana ekran, solo kurulum, ayarlar, arkadaş kurulumu, lobi, duraklatma, solo sonuç ve maç sonucu 320 ve 393 px'te **23 px**, 1280 px'te **39 px** (hepsi eşit; önceki 33–45 px). Oyun ekranlarında site başlığı yoktur; üst satır 8 px iç boşlukla başlar ve sayfa kaymaz. Geri, duraklat ve ayarlar hedefleri 48 px; içerik kutusu başlık satırıyla çakışmıyor, yatay taşma yok.

## Tema

Ayarlar'da **Sistem / Açık / Koyu** seçimi vardır; varsayılan Sistem'dir ve seçim `theme` alanında saklanır. Açık/Koyu seçiliyse `index.html` başındaki küçük satır içi betik, stil dosyası boyanmadan önce `html[data-theme]` ve `theme-color` değerlerini ayarlar; böylece sayfa sistem temasıyla açılıp sonradan sıçramaz. Uygulama betiği engellenmiş yeniden yüklemede de koyu zeminle açıldığı doğrulandı. Sistem seçilince öznitelik kaldırılır ve cihaz teması izlenir.

## Titreşim

Standart Titreşim API'si (`navigator.vibrate`) yalnız onu sunan dokunmatik cihazlarda (pratikte Android tarayıcıları) çalışır; **iOS Safari'de yoktur**. Ayar yalnız API ve kaba işaretçi birlikte varsa gösterilir, yoksa satır hiç çizilmez (masaüstünde ve API'siz ortamda gizli olduğu doğrulandı). Varsayılan kapalıdır. Açıkken: sayı girişi 8 ms, not 6 ms, anahtar/seçim 8–10 ms, ipucu 12 ms, hata 18-50-18, bitiş 12-70-24, maç başlangıcı 30-60-30 ms. Gerçek cihazda hissedilmesi denenmedi.

## "Ayrılma onayı bekleniyor" iletisi

Sebep: maçtan ya da lobiden ayrılınca bu not yazılıyor, sunucu ayrılmayı onaylayıp ana ekrana dönüldüğünde ise temizlenmiyordu. Arkadaş ekranına tekrar girildiğinde, ortada bekleyen bir şey olmadığı hâlde kırmızı hata kutusunda görünüyordu (lobiden ayrıl → ana ekran → "Arkadaşınla yarış" ile yeniden üretildi). Düzeltme: onay gelince ve ekrana her girişte not temizlenir; ayrılma sürerken görünen metin "Maçtan ayrılıyorsun…" oldu ve bilgi notları (ayrılıyor, bağlantı kopyalandı) artık hata renginde değil.

## Hareket

Yeni bir ekrana geçişte bölüm bir kez belirir (çocuklar 0/40/75/105/130/150 ms gecikmeyle, 260 ms, `cubic-bezier(.23,1,.32,1)`); aynı ekranın yeniden çizimi (her hamle, her yoklama) animasyonsuzdur. Oyun ekranı ve "hareketi azalt" tercihinde yalnız 160 ms opaklık geçişi vardır. Düğmelerde basışta `scale(.97)`; hover stilleri `(hover:hover) and (pointer:fine)` arkasındadır. Hücre seçimi ve sayı girişi animasyonsuzdur.

## Sesler (Cuelume, varsayılan kapalı)

[Cuelume](https://cuelume.dev) 0.2.8 (MIT) Web Audio ile canlı sentezlenir; ses dosyası yoktur. Paketleyici olmadığı için kütüphane `scripts/web-vendor-cuelume.mjs` ile tek dosyaya (`apps/web/cuelume.js`, ~30 KB, gzip ~8 KB; yalnız varsayılan tema ve `play/unlock/setEnabled`) indirgenmiştir. Dosya **yalnız ses açıkken veya Ayarlar açılınca** indirilir; servis çalışanı ilk indirmede önbelleğe alır.

- Ayarlar → **Ses** anahtarı; tercih `sudoku-rivals.preferences.v1` içinde `sound` alanında saklanır, varsayılan `false`.
- Kullanıcı sayfaya dokunmadan/tuşa basmadan hiçbir ses çalmaz (kayıtlı tercih açık olsa bile); `AudioContext` ilk seste oluşturulur.

| Etkileşim | Ses |
| --- | --- |
| Sayı yazma | `tap`; çakışma oluşturursa hafif `error` |
| Not yazma (kalem) | `type` |
| Sil / geri al / yinele | hafif `tap` |
| Kalem modu, lobi "Hazırım", ayar anahtarları | `toggle` |
| Boyut ve dil seçimi | `select` |
| Ekranlar arası ileri/geri | hafif `navigate` (geri yönlü) |
| Oyun başlatma/devam, duraklatmadan dönüş · duraklatma | `open` · hafif `close` |
| İpucu bulundu · davet bağlantısı kopyalandı | `ready` · hafif `success` |
| Arkadaş lobiye katıldı · maç başladı | `ready` · `attention` |
| Düelloda yanlış hamle (sunucu yanıtı) | `error` |
| Tahta tamamlandı / maç kazanıldı · diğer maç sonuçları | güçlü `success` · `close` |

Hücre seçimi, ok tuşlarıyla gezinme, sayaç ve rakip ilerlemesi sessizdir.

## Ölçüm (Chromium, `isMobile`, dokunmatik, DPR 2; solo/yarış/düello × 6/9, açık ve koyu tema)

| Görünüm | Sonuç |
| --- | --- |
| 390×844, 412×780, 390×664, 375×667, 360×640 | Kaydırma yok; 9×9 tahta 333–388 px |
| 375×553 (adres çubuklu küçük Safari), 320×568 | Kaydırma yok; tahta 282–328 px; ipucu ve iletiler açıkken de kayma yok |
| 844×390, 667×375 yatay | Kaydırma yok; sayılar/araçlar yanda |
| 1280×800 masaüstü | Kaydırma yok; tahta 596 px, sağda 300 px panel |

Dokunma hedefleri ≥48 px (ayar anahtarları 52×32, satırın tamamı 60 px yüksekliğinde etikettir). Oyun dışı ekranlar (ayarlar, arkadaş kurulumu, lobi) kısa telefonlarda belge içinde kayar. Yeni renk çiftleri metinde ≥4.5:1, sınırlarda ≥3:1 hesaplandı (iki tema).

## Doğrulama

- `cargo test --locked --offline -p sudoku-server`: geçti (allowlist'e `sound.js`, `cuelume.js` eklendi).
- `node tests/wasm.mjs`, `node tests/ui-contract.mjs`, `python3 -m unittest tests/test_web_icons.py`, `node tests/web-install.mjs`: geçti.
- `SUDOKU_SERVER_URL=… node tests/web-multiplayer.mjs --http`: yarış/düello × 6/9 geçti.
- `node tests/web-solo.mjs --browser`: yalnız [önceden belgelenen](web-game-screen-layout.md#bilinen-açıklar) 175. satırda düşer; o satır geçici atlanınca tarayıcı akışının tamamı geçer.
- `sh scripts/staging-build.sh` + `python3 tests/staging-package.py`: paket yeni dosyalarla açılıyor, HTTP maç matrisi geçti.
- İpucu/ileti: 320×568, 360×640, 375×553, 375×667, 393×659, 390×844, 844×390 ve 1280×800'de 6×6 ve 9×9 için ipucu kartı ve ileti şeridi açılmadan önce/sonra başlık, tahta, sayı tuşları ve araçların konum ve boyutu ölçüldü: birebir aynı; kart tahtayla, şerit tahta ve düğmelerle kesişmiyor. Düello (375×553, 320×568) ve yarışta hamle geri bildirimi yerleşimi değiştirmiyor; ayrılma onay sayfasından geçiyor, tarayıcı penceresi açılmıyor.
- 3. tur (başsız Chromium): ana ekran göstergesi tek simge ve "6×6 & 9×9" yok; solo ve arkadaşta varsayılan 9×9, seçim/ad yeniden yüklemede korunuyor; davet bağlantısı üç dar telefonda kod + ad + Katıl görünür; bağlantı → kod; Enter ile katılma; ayrıldıktan sonra kurulum ekranında eski ileti yok; sayfa hatası ve tarayıcı penceresi yok.
- 4. tur (başsız Chromium): zorluk seçimi ve ipuçlarıyla uçtan uca çözüm (5 kombinasyon), tema (varsayılan, kalıcılık, betik engelliyken ilk boyama, sistem takibi), titreşim ayarının görünürlüğü ve kapalıyken sessizliği, arkadaş kurulumunda logo–Geri boşluğu 4 px (Geri hedefi 48 px).
- 5. tur (başsız Chromium, iki gerçek oyuncu): kurucu düello 6×6 Orta ve yarış 9×9 Zor oda kurdu; Türkçe arayüzle katılan arkadaş lobide seviyeyi ve açıklamasını gördü; iki tarafın ekranındaki verilen rakamlar aynı ve motor onları odanın seviyesinde derecelendirdi; seviye yeniden yüklemede hatırlandı.
- 6. tur (beş kademe): on boyut·kademe kombinasyonu yalnız ipuçlarıyla çözüldü; beş seçenek 320 px'te tek satırda ve ≥48 px; arkadaş maçında düello 6×6 Uzman ve yarış 9×9 Başlangıç dahil dört oda iki oyuncuya aynı, doğru kademede bulmacayı verdi.
- Ses, başsız Chromium'da sayaçlı `AudioContext` ile denendi: varsayılanda bağlam ve kütüphane isteği yok; anahtar açılınca onay sesi; oyun etkileşimlerinde ses; yeniden yüklemede etkileşimden önce ses yok; kapatınca sessiz.

## Açık kalanlar

- Seslerin gerçek cihazda duyulması (özellikle iOS Safari sessiz anahtarı ve ilk dokunuş) ve ses düzeyi insan kulağıyla denenmedi.
- Fiziksel cihaz, VoiceOver/TalkBack ve gerçek Safari adres çubuğu davranışı denenmedi.
- `docs/ui-contract.json` renk rolleri değiştirilmedi; web paleti ondan ayrılır (sözleşmeye göre başlangıç değerleridir). Native uygulamalar eski paleti kullanmayı sürdürür.
