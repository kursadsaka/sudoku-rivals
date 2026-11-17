# Web açılışı — erken ana ekran ve WASM preload

17 Kasım 2026, ASUS/Linux. Güncel uzak `agent/g16-render-free` (`041bcf8`) üzerinden ayrı `agent/g16-startup-fast` worktree'sinde uygulandı. Önceki araştırma kaynakları yeniden yazılmadı; bildirim PR'ının birleşmiş değişiklikleri korundu. Merge/deploy ve Render ayarı değişikliği yapılmadı.

## Davranış

Ana ekran artık motor import/init tamamlanmadan çiziliyor. Solo/arkadaş/ayarlar düğmeleri motor hazır olana kadar native `disabled`; olay işleyicileri de hazır olmayı kontrol ediyor. Bekleyen tıklamalar kuyruğa alınmıyor. Kayıtların henüz okunmadığı açıkça belirtiliyor; boş veya bozuk olduklarına erken karar verilmiyor. Motor hazır olunca mevcut Rust doğrulamasıyla kayıt kartları ve hatalar render ediliyor; önceki kaydı değiştirme onayları korunuyor.

`/r/...` yolu beklerken korunuyor ve davete özel hazırlık metni gösteriliyor. Hazır olunca mevcut davet/katılma akışı çalışıyor; otomatik katılım isteği eklenmedi. Korumalı canlı kayıt varsa mevcut kurtarma ekranı önceliği korunuyor. Motor glue importu da try/catch içinde: import veya WASM hatasında kayıtlar silinmeden yerelleştirilmiş hata ve sayfayı yenile eylemi sunuluyor; yenileme davet yolunu koruyor.

HTML başındaki `as=fetch`, `type=application/wasm`, `crossorigin` preload, glue'nun kullandığı aynı kök WASM URL'sini indiriyor. Service worker cache sürümü v18; mevcut API bypass, navigasyon network-first, statik cache ve sıkıştırma yolları korunuyor. Motor JS'i dinamik import edilerek kabuğu ve hata arayüzünü bekletmiyor.

Performance mark'ları: `sudoku-home-shell`, `sudoku-home-frame` (iki requestAnimationFrame sonrası; boyama garantisi veya animasyon bitişi değil), `sudoku-engine-start`, `sudoku-engine-initialized`, `sudoku-engine-ready`, hata halinde `sudoku-engine-failed`; `sudoku-engine` ölçüsü import başlangıcından ilk hazır render'a kadar. Kayıt, davet veya token içerikleri ölçümlere yazılmaz. Kaynak indirme zamanları Resource Timing'den okunabilir.

## Kontrollü performans karşılaştırması

Chromium 155.0.8059.84, headless, aynı makine ve aynı WASM SHA-256: `693f0d488b59e783e2f82f0f8f356a68506f3660440cce361ab4e11cebd7d877` (725569 B). Temel ve aday için beş ayrı temiz tarayıcı bağlamı, kayıt yok, SW bloklu. Loopback test sunucusu HTML'ye 100 ms, her JS yanıtına 80 ms, WASM yanıtına 600 ms gecikme ekliyor. Bant genişliği/CPU throttle uygulanmadı. Değerler navigation başlangıcından itibaren ortanca; bütün örnekler [JSON](web-startup-results.json) içinde.

| Ölçüt | Temel 041bcf8 | Aday | Azalma |
| --- | ---: | ---: | ---: |
| WASM isteği başlangıcı | 297,7 ms | 111,2 ms | 186,5 ms |
| Ana ekran DOM gözlemi | 915,2 ms | 297,6 ms | 617,6 ms (%67) |
| Ana ekran sonrası iki rAF | 928,6 ms | 328,2 ms | 600,4 ms |
| Oyun eylemleri etkin | 915,3 ms | 733,0 ms | 182,3 ms (%20) |

Her ölçümde sunucuda **tam bir WASM isteği** doğrulandı; aday Resource Timing initiator `link`, temel `fetch`. Böylece bu tarayıcı/koşullarda preload'nun çift indirme yaratmadığı gösterildi. SW kurulumunun cache.addAll trafiği bu tek istek iddiasına dahil değildir.

Bunlar canlı staging sonuçları veya gerçek internet kazancı tahmini değildir. Önceki raporun Chrome 157 / Render ölçümüyle doğrudan kıyaslanmaz. DOM/rAF, LCP veya giriş animasyonunun bitişi değildir; ilk render'ın motor indirmesinden ayrıldığını ve hazır olmanın erken indirmeyle ilerlediğini gösterir. Render soğuk başlangıcının HTML öncesindeki gecikmesi değişmedi.

## Regresyonlar

- `scripts/web-build.sh`: gerçek WASM yeniden derlendi.
- `cargo test --locked --workspace`: 105 geçti, mevcut 1 ignored. İlk koşunun geçici dosya oluşturması sistem `/tmp` kotasına takıldı; `TMPDIR` bu worktree'deki `.test-tmp` olarak verilerek tam suite başarıyla tekrarlandı.
- `node tests/wasm.mjs`, `node tests/web-solo.mjs`, `node tests/ui-contract.mjs`: geçti.
- `SUDOKU_SERVER_URL=http://127.0.0.1:3187 node tests/web-multiplayer.mjs --http`: gerçek izole Rust sunucusunda yarış/düello × 6/9, restore, kayıp create/join yanıtı, sonuç/rövanş/ayrılma ve mevcut G14 regresyonları geçti.
- `tests/web-startup.mjs`: WASM beklerken sentetik tıklama/klavye/marka bağlantısı; doğru ve bozuk solo kayıtları; korumalı canlı kayıt; engelli localStorage; davet kodu/yolu; WASM ve glue yükleme hatası sonrası yenileme; kurulu SW ile gerçek offline reload; tek WASM isteği geçti. Tarayıcı pageerror yok.

Yeni browser testi bağımlılık eklemez; mevcut Playwright kurulumuyla çalışır:

```sh
PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs node tests/web-startup.mjs
# İsteğe bağlı önce/sonra: aynı WASM pkg klasörünü içeren temel web dizinini verin.
PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs node tests/web-startup.mjs --baseline /absolute/path/to/base/apps/web
```

Bu makinede `/tmp` kota hatası Chromium profilini de etkiledi; uzun worktree yolu ise Chromium UNIX socket sınırını aştı. Yalnız bu işe ait kısa `/home/ksao/.t3/scratch/g16-startup-tmp` dizini `TMPDIR` olarak kullanıldı. Bunlar ürün hatası olarak sayılmadı; diğer süreçler/dosyalar temizlenmedi.

## Yerel inceleme ve sınırlar

Test ortamı: `http://127.0.0.1:3187`, aynı izole worktree'nin debug sunucusu. Yeniden başlatma: önce sunucuyu derleyin, sonra `SUDOKU_BIND=127.0.0.1 SUDOKU_PORT=3187 target/debug/sudoku-server`. İnceleme için tutuldu; public URL değildir. Paylaşılan Render staging'e deploy yapılmadı.

Uygulama ve doğrulama yaklaşık 15 dakika; 13:04–13:11 TRT arası derleme/test kayıtları var, ön okuma ve PR hazırlığı buna ek. İlk ortam hataları tmp dizini değiştirilerek çözüldü. Gerçek telefon/Safari/Firefox, insan erişilebilirlik incelemesi, eski v17'den canlı SW yükseltmesi, gerçek Render uyku dönüşü ve kullanıcıya özel kayıtlar denenmedi. Testler sentetik kayıtlarla izole tarayıcı bağlamlarında çalıştı. HTML'den önceki sunucu beklemesini veya yeni Uzman bulmacanın senkron üretimini bu değişiklik çözmez.
