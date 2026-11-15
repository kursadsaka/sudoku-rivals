# Web görsel yenileme ve isteğe bağlı sesler

Kapsam: yalnız `apps/web` ve onu sunan allowlist/paket listeleri (`server/src/http.rs`, `scripts/staging-build.sh`, `apps/web/sw.js`). iOS ve Android dosyalarına dokunulmadı. Üç mod, 6×6/9×9, tüm `data-testid` kimlikleri, klavye kısayolları, ekran okuyucu etiketleri ve `docs/ui-contract.md` içindeki dar ekran sırası (gezinme → durum → tahta → sayılar → araçlar) korunur. [Tek ekran yerleşimi notu](web-game-screen-layout.md) geçerliliğini korur; aşağıdakiler onun üzerine gelir.

## Görsel dil

- **Kâğıt ve mürekkep**: sıcak kâğıt zemin, basılı (siyah) verilen sayılar, tükenmez mavisi oyuncu girişleri, kurşun kalem gri notlar. Marka mavisi uygulama simgesiyle aynıdır (`#1d4ed8`). **Kehribar her yerde rakibi** gösterir (yarış çubuğu, düello puanı, lobi, sonuç). Koyu temada nötr koyu zemin kullanılır. Yazı tipi sistem yığınıdır; sayılar ve başlıklarda varsa `ui-rounded`. Harici font/görsel yok.
- **Ana ekran**: iki mod kartı (solo dolu marka kartı; arkadaş kartında mavi/kehribar yarış çubukları), "Kaldığın yerden" kartlarında boyut rozeti.
- **Oyun ekranı**: üstte geri · `mod · boyut` + büyük süre · duraklat. Altında ince bir **gösterge**: solo'da doluluk, yarışta `Sen %` ↔ `% Rakip` karşılıklı iki çubuk, düelloda puanlar + 90 saniyelik kalan süre çubuğu (son 10 saniyede kırmızı). Seçili hücre dolu marka rengi; aynı sayı, ilgili satır/sütun/kutu ve çakışma ayrı tonlardadır (çakışmada ayrıca `!` ve "Çakışma var" metni).
- **Sayı tuşları**: her tuşta o rakamdan kaç tane eksik olduğu yazar (ekran okuyucuda gizli); biten rakam soluklaşır. Kalem modunda tuşlar kesik çizgili ve gri olur. Yüksekliği ≥790 px olan dikey telefonlarda tuşlar hesap makinesi düzeninde 3 sütundur (boşluk yerine büyük tuş); daha kısa ekranlarda 9×9 için `5+4`, 6×6 için tek sıra korunur.
- **Arkadaş kurulumu/lobi/sonuç**: mod ve boyut için bölmeli seçim, oda kodu harf harf karolarda, oyuncu satırlarında baş harf ve "Hazır/Bekliyor" rozeti; düello sonucu iki sütunlu skor.
- **Ayarlar**: tek kartta dil, ses, hareketi azalt, titreşim (bu sürümde kapalı) anahtarları.

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
| 375×553 (adres çubuklu küçük Safari), 320×568 | Kaydırma yok; 9×9 tahta 280–296 px. Yalnız ipucu açıklaması görünürken +13–29 px kayar |
| 844×390, 667×375 yatay | Kaydırma yok; sayılar/araçlar yanda |
| 1280×800 masaüstü | Kaydırma yok; tahta 596 px, sağda 300 px panel |

Dokunma hedefleri ≥48 px (ayar anahtarları 52×32, satırın tamamı 60 px yüksekliğinde etikettir). Oyun dışı ekranlar (ayarlar, arkadaş kurulumu, lobi) kısa telefonlarda belge içinde kayar. Yeni renk çiftleri metinde ≥4.5:1, sınırlarda ≥3:1 hesaplandı (iki tema).

## Doğrulama

- `cargo test --locked --offline -p sudoku-server`: geçti (allowlist'e `sound.js`, `cuelume.js` eklendi).
- `node tests/wasm.mjs`, `node tests/ui-contract.mjs`, `python3 -m unittest tests/test_web_icons.py`, `node tests/web-install.mjs`: geçti.
- `SUDOKU_SERVER_URL=… node tests/web-multiplayer.mjs --http`: yarış/düello × 6/9 geçti.
- `node tests/web-solo.mjs --browser`: yalnız [önceden belgelenen](web-game-screen-layout.md#bilinen-açıklar) 175. satırda düşer; o satır geçici atlanınca tarayıcı akışının tamamı geçer.
- `sh scripts/staging-build.sh` + `python3 tests/staging-package.py`: paket yeni dosyalarla açılıyor, HTTP maç matrisi geçti.
- Ses, başsız Chromium'da sayaçlı `AudioContext` ile denendi: varsayılanda bağlam ve kütüphane isteği yok; anahtar açılınca onay sesi; oyun etkileşimlerinde ses; yeniden yüklemede etkileşimden önce ses yok; kapatınca sessiz.

## Açık kalanlar

- Seslerin gerçek cihazda duyulması (özellikle iOS Safari sessiz anahtarı ve ilk dokunuş) ve ses düzeyi insan kulağıyla denenmedi.
- Fiziksel cihaz, VoiceOver/TalkBack ve gerçek Safari adres çubuğu davranışı denenmedi.
- `docs/ui-contract.json` renk rolleri değiştirilmedi; web paleti ondan ayrılır (sözleşmeye göre başlangıç değerleridir). Native uygulamalar eski paleti kullanmayı sürdürür.
