# G06 — yerel maç protokolü v1

Durum: yerel uygulama ve otomatik test. Puanlar, süre eşikleri ve boş hücre sayıları geri alınabilir uygulama varsayılanlarıdır; insanlarla dengelenmiş zorluk veya hile önleme iddiası yoktur. `docs/rules.md` ve G01 inceleme düzeltmeleri eski taslağın 100 ms beraberlik penceresi, 10 saniye yarış doğrulama kilidi, düello notları ve hareketsizlik yenilgisi önerilerinin yerine geçer.

## Çalıştırma ve sınır

`cargo run --locked -p sudoku-server`; varsayılan `127.0.0.1:3000`. Bağımsız test sunucusu için `SUDOKU_PORT=3001` seçilebilir; adres her zaman loopback kalır. Mevcut `POST /validate` aynen korunur. Ek protokol HTTP JSON'dur; WebSocket gerekmez. Taşıma adapteri her platformda HTTP isteğini gerçekleştirir, ortak DTO'lar `core/src/protocol.rs` içindedir. CORS genişletmesi, TLS, dağıtım ve kalıcı veritabanı eklenmedi. Web istemcisi bu API'ye aynı origin üzerinden eriştirilmelidir; staging düzeni G16 işidir.

`Service` bir monotonik saat akışıyla sınanabilir durum makinesidir. HTTP adapteri kısa bir giriş kilidi altında `Instant` saatini kaydederek isteği en fazla 256 öğelik FIFO kuyruğa ekler. Ayrı bir sahip iş parçacığı maç durumunu bu sırada işler; bulmaca üretimi giriş kilidini tutmaz. Kuyruk doluysa kabul edilmemiş istek `capacity` alır. **Yetkili varış noktası**, JSON isteğinin bu sıraya kabul edildiği andır; soketin ilk byte'ının gelişi değildir. İşleme aynı sırada senkron ilerler; bir zaman aşımı daha önce kabul edilmiş hamleyi geçemez. İstemci zaman damgası kabul edilmez. Aynı milisaniyede ilk olay önce işlenir.

## İstekler

Bütün yollar POST, `Content-Type: application/json`; gövde üst sınırı 16 KiB. `protocol` daima `1`. Token'lar URL'ye konmaz, günlüğe yazılmaz. Oda kodu yalnız davet yetkisidir; mevcut koltuğu devralamaz.

| Yol | Gövde | Başarılı yanıt |
| --- | --- | --- |
| `/v1/rooms` | `{protocol, guest?, name, mode, size}` | `Session` |
| `/v1/rooms/{code}/join` | `{protocol, guest?, name}` | `Session` |
| `/v1/rooms/{code}/resume` | `{protocol, token}` | `Snapshot` |
| `/v1/rooms/{code}/state` | `{protocol, token, generation}` | `Snapshot` |
| `/v1/rooms/{code}/command` | `Envelope` | `{receipt, snapshot}` |

`mode`: `race` veya `duel`; `size`: 6 veya 9. Görünen ad 1–16 Unicode karakteri, kontrol karakteri yok. `guest` verilmezse kişisel veri içermeyen rastgele 256 bit misafir belirteci üretilir. Verilirse önceki `Session.guest` gönderilir; etkin lobide/maçta aynı misafir ikinci koltuk alamaz. Bitmiş maç yeni oda açmayı engellemez. Bu yerel misafir modeli hesap veya cihaz doğrulaması değildir.

`Session`: `{protocol, guest, token, invite_path, snapshot}`. `guest` ve odaya özel `token` ayrı 256 bit `/dev/urandom` değerleridir. Hata durumunda sabit veya öngörülebilir token'a düşülmez. `invite_path` örneği `/r/ABC234`; oda kodu altı karakter, büyük/küçük harf duyarsızdır. Alfabe `23456789ABCDEFGHJKMNPQRSTUVWXYZ`, seçim modulo yanlılığı olmadan yapılır. Kapanmış oda kodu 24 saat tekrar kullanılmaz; süreç yeniden başlarsa eski odalar kurtarılmaz.

Örnek komut:

```json
{"protocol":1,"token":"<Session.token>","generation":1,"match_id":"<Snapshot.match_id>","sequence":1,"command":{"type":"ready","ready":true}}
```

| `command.type` | Diğer alanlar | Anlam |
| --- | --- | --- |
| `ready` | `ready: bool` | İki koltuk dolu lobide hazır olma; ikisi hazır ve bağlıysa 3 sn geri sayım |
| `sync` | `base_version: u64, cells: u8[]` | Yarışta tek tam tahta eşitlemesi; son onaylı sürüme dayanır |
| `move` | `board_id: u32, index: u32, value: u8` | Düelloda sıfır tabanlı hücreye 1..size rakam |
| `disconnect` | — | Bağlantı kopmasını bildir; koltuk yeniden bağlanma için tutulur |
| `resign` | — | Oynanan maçta pes; rakip kazanır |
| `rematch` | — | Sonuç ekranında rövanş iste; ikisi de bağlı ve istekliyse yeni maç |
| `lobby` | — | Sonuçtan lobiye dön; sonraki iki hazır isteği yeni maç açar |

Yeni maçın kimliği, bulmaca tohumu, işlem sıraları, skorları ve tahtaları yenidir. İki koltuk aynı bulmaca dizisini kendi hızlarında oynar. Tohum ve çözüm gönderilmez. Bulmaca parametreleri 6×6'da 18, 9×9'da 43 boş hücredir; bunlara “kolay” etiketi verilmez. Üretilen tahtalar ortak çekirdek tarafından tek çözümle doğrulanır. Kaldırma sırası bu sayıya ulaşamazsa en çok 16 belirlenimli aday denenir; arıza sessizce başka kurala dönüşmez.

## Snapshot ve erişim sınırı

Üst alanlar: `protocol, room_code, match_id, generation, mode, size, phase, server_ms, event_order, start_ms, end_ms, own, opponent, result`.

- `phase`: `lobby`, `countdown`, `playing`, `finished`, `closed`.
- `own`: `seat` (0 veya 1), `name`, `ready`, `connected`, `next_sequence`, `board_id`, `board_version`, `givens`, `cells`, `score`, `combo`, `wrong`, `locked_until_ms`, `rematch`.
- `opponent`: yalnız `name, ready, connected, progress_percent, score, rematch`; boş koltukta null. Yarışta dolu boş hücre oranı (doğruluk bilgisi değildir) en çok saniyede iki kez yayımlanır; düelloda yalnız skor gösterilir. Rakibin hücreleri, çözümü, serisi, yanlış sayısı veya token'ı gönderilmez.
- `result`: yalnız bitmiş maçta `{winner: 0|1|null, reason, at_ms}`; başka durumda null. `both_disconnected` ve `resource_limit` sonuçlarında `winner:null` bir galibiyet veya yenilgi değildir. `time` ve eşit ölçütlerde null beraberliktir.
- `end_ms` yalnız düelloda `start_ms + 90000`; yarışın süre sınırı yoktur.

`cells/givens` lobi ve geri sayımda boştur; ancak `playing` başladığında iki tarafa açılır. İptal edilen geri sayım bulmacayı önceden görme veya çözme imkânı vermez. Bu, istemcinin çözümü başka programla çıkarmasını engellemez. Otomatik çözücü hilesi kabul edilen sınırdır; otomatik yaptırım eklenmez.

## İşlem tekrarı ve kayıp yanıt

`sequence`, koltuk ve maç başına 1'den başlayan tam artan sayıdır; kaynak `own.next_sequence`. Kabul edilen komut veya kayıt altına alınan oyun kuralı reddi, bu sıranın tek sonucunu oluşturur. Yapısal olarak yanlış uzunlukta `sync` kuyruğa ulaştıktan sonra `invalid_input` alır; Receipt veya yeni sıra tüketmez. Aynı sıra daha önce kayıtlıysa önce içeriği karşılaştırılır. Aynı maç/koltuk/sıra ve aynı komut tekrarlandığında **aynı dondurulmuş Receipt** döner; skor, yanlış, kilit ve hız kotası ikinci kez değişmez. Snapshot güncel olabilir. Farklı içerikli aynı sıra `sequence_conflict`, sıra boşluğu `sequence_gap` verir; yeni işleme geçmeden durum eşitlenir.

`Receipt`: `match_id, at_ms, event_order, sequence, code, board_version, board_id, score, combo, wrong, locked_until_ms`. Bir rövanşı başlatan Receipt eski maç kimliğini taşır, beraberindeki Snapshot yeni maça ait olabilir. Eski maça ait saklı Receipt'ler oda ömrü boyunca tekrar dönebilir. İstemci yeni maçın sıra sayacına eski Receipt'in sayacını uygulamamalıdır.

`resume`, koltuk token'ını doğrulayıp `generation` artırır. Eski nesil komutlar, henüz uygulanmamış olsa da `stale_generation` ile reddedilir; yeni nesil son sıra/sürüm ve güncel görüntüyü alır. Resume sonrası önceki neslin hamleleri körlemesine yeniden gönderilmez. Yarışta yerel düzenlemeler ayrı saklanır, son onaylı sürümden **tek `sync`** gönderilir. `stale_version` olursa yerel tahta sessizce ezilmez. Düelloda işlenmemiş eski nesil hamleleri bırakılır; sunucu görüntüsünden devam edilir.

## Oyun kuralları

Yarış `sync` girişinde boyut/rakam aralığı/verilen hücreler korunur; çakışan oyuncu girişleri tutulabilir. Değişmeyen tahta `no_change`, sürüm artmaz ve tekrar bitiş denetimi doğurmaz. Değişmiş tam tahta anında doğrulanır: yanlışsa yalnız `not_correct`, doğruysa ilk kabul edilen tek kazanan olur. 10 saniye doğrulama kilidi ve 100 ms beraberlik penceresi yoktur. Notlar/undo yereldir; sunucuya sonunda tek tahta gider.

Düelloda `start_ms <= at_ms < end_ms` kabul aralığıdır. Boş, değiştirilebilir hücreye doğru sayı kilitlenerek yazılır. Seri 1–3: +10, 4–6: +15, 7–9: +20, 10 ve üstü: +25. Yanlış sayı yazılmaz; skor −5 (sıfır taban), seri sıfır, yanlış sayısı +1, 2000 ms giriş kilidi. Kilitteki ret yeni ceza değildir. Her tamamlanmış tahta +50'yi yalnız bir kez alır ve sıradaki tahta aynı işlemde açılır. Düelloda not, silme ve undo yoktur. Süre dolunca yüksek skor, ardından az yanlış kazanır; bunlar eşitse beraberlik. Pes skordan bağımsız yenilgidir.

Yeni tahta giriş denemeleri doğruluk incelenmeden önce koltuk başına kayan 1000 ms'de 6 ile sınırlanır. Retler cezasızdır; 10 saniyede 30 hız reddi bağlantıyı koparır. Tek `sync`, içindeki değişiklik sayısından bağımsız tek denemedir. Aynı işlem tekrarı kotayı tüketmez.

## Bağlantı, saat ve arıza

İstemci durumunu en geç 5 saniyede bir yoklamalıdır. 15 saniye istek gelmediyse bir sonraki yetkili sunucu olayında bağlantı kopuk ilan edilir; geri sayım iptal olur. Açık `disconnect` de aynı davranışı hemen başlatır. Zamanlayıcılar her sunucu isteğinde ilerletilir; arka planda ayrı görev yoktur. Bağlantıyı kopuk ilan etme anı sunucunun o olaydaki zamanıdır.

- Yarış: kopuk ilan edilen oyuncuya 120 saniye; kesin sınırda timeout önce işlenir. Daha erken sona eren kopma süresi yenilgi; iki eşit sürenin aynı anda bitmesi sonuçsuz maç. Bağlı ve yalnız hareketsiz oyuncuya otomatik yenilgi yoktur.
- Düello: kopma 90 saniyeyi durdurmaz veya skoru silmez. Yeni nesille kalan sürede devam edilebilir.
- Lobi: 30 dakika ömür; kopuk iki kişilik lobide koltuk 120 saniye rezerve edilir. Tek kalan koltuk lobi ömrünce kalabilir. Sonuç: kesinleştiği andan itibaren 10 dakika; sonrasında oda kapanır.
- Restart: veri yalnız RAM'de olduğundan eski oda `room_missing` olur. İstemci bunu galibiyet saymamalı; “Maç sürdürülemiyor / sonuç kurtarılamadı” göstermeli. Güvenilir olay/Receipt kurtarma olmadığı için sonuç uydurulmaz. Kalıcı kurtarma G16 öncesi ayrı karar gerektirir.

## Hatalar ve kaynak sınırları

JSON protokol hataları `{protocol:1,error:"..."}`: 401 `unauthorized`; 404 `room_missing`; 410 `room_closed`; 429 `rate_limited/capacity`; 409 `protocol, invalid_input, room_full, stale_generation, stale_match, sequence_gap, sequence_conflict, clock_order`; 500 `internal`. Bozuk JSON/body tipi için Axum'un 4xx çıkarıcı yanıtı kullanılabilir; istemci her başarısız HTTP yanıtını JSON varsaymamalıdır.

Komut Receipt kodları: `ok, wrong, not_correct, no_change, not_started, match_over, wrong_mode, invalid_board, invalid_cell, invalid_value, given_cell, filled_cell, stale_board, stale_version, locked, rate_limited, disconnected, invalid_state`. Bunlar HTTP başarısından ayrı oyun sonuçlarıdır; ret de bir sıra tüketir.

Tek süreçte en fazla 128 oda/tombstone; geçmiş maçlar dahil her koltuk konumu en fazla 8192 Receipt tutar (oda toplamı en fazla 16384). Kotasını dolduran yalnız o koltuk `capacity` alır; rakibinin kazanma hakkı veya maç sonucu silinmez. Eski Receipt'ler sessizce atılmaz. Yanlış uzunlukta `sync` dizileri Receipt olarak saklanmaz; saklanan komut dizileri en fazla size² bayttır. Oda açma aynı loopback IP'den 10 dakikada en çok 128, katılım 256 (bütün join istekleri); loopback dışındaki IP'ler için 5/10 sınırı korunur. Yerel test istemcilerinin aynı 127.0.0.1 IP'sini paylaşması bu şekilde hesaba katılır. Bu yerel sınırlar üretim güvenliği veya dağıtık yük kapasitesi kanıtı değildir.

## Test kanıtı

`cargo test -p sudoku-server --locked --offline -- --nocapture`:

- İki mod × iki boyutta tam bot maçları; aynı maç/sonuç.
- 80 ms aralıklı iki bitişte ilk kazanan; yanlış tam tahtanın kilitsiz düzeltilmesi.
- Service testlerinde 89.999/90.000 ms göreli süre sınırı. Ayrı giriş kuyruğu testinde, gerçek saat son sınırı geçene kadar işçi bloke edilir; önceden kuyruğa kabul edilmiş hamle özgün damgasıyla puan alır. HTTP endpointleri ayrıca gerçek loopback istekleriyle sınanır; Service'e verilen sabit zaman tek başına ağ gecikmesi kanıtı sayılmaz.
- Kayıp yanlış yanıtının 50 tekrarı; içerik değiştirme, sıra boşluğu, eski nesil.
- 40 yerel düzenleme sonrası tek eşitleme; eski sürümde veri korunması.
- Aynı anda kopma sürelerinin bitmesi, kesin sınırda resume ve restartta sonuç uydurmama.
- Hız limiti, yanlış kilidi, skor tabanı, altı puan örneği, bonus tekrarı, yeni rövanş kimliği ve eski Receipt tekrarı.
- HTTP `/validate`, sürüm/kimlik denetimi, iki botla gerçek yarış, resume ve rakip gizliliği.
- 1000 sıralı kimlik doğrulanmış yerel HTTP yoklaması: ilk koşuda yaklaşık 1.20 saniye, sınırlı test runtime ile son koşuda 463 ms. Bu küçük yerel ölçüm üretim kapasitesi veya p95 gecikme hedefini doğrulamaz.

Sandbox loopback soketini engelliyorsa HTTP testleri ağ sınırının dışında, yalnız geçici `127.0.0.1` portunda koşulur. Testler izin yokluğunu başarı gibi atlamaz. Gerçek platformlar arası ağ, mobil arka plan, internet gecikmesi ve bağımsız inceleme henüz bu G06 koşusunun kanıtı değildir.

## Explicit exit

`POST /v1/rooms/{code}/leave` takes `{protocol:1,token}` and returns
`{protocol:1,left:true}`. It releases that seat, so the guest can create/join a
new room immediately. In play it first records a single forfeit; in countdown
it cancels the start and leaves the other player in the lobby. Last exit closes
the room. It does not shift seat indices. A syntactically valid but absent token
or room is acknowledged identically, making retries harmless without revealing
credential existence. A removed token can no longer poll/resume/command.
This is separate from a temporary `disconnect`, which reserves a seat.

## Local web origin

The loopback server also serves the allowlisted public web build at `/`, and
`/r/CODE` returns the same entry HTML. API and assets share an origin; no broad
CORS policy is enabled. `/health` returns `{status:"ok",protocol:1}`. Known build
assets are loaded from this checkout's `apps/web`; arbitrary URI paths are never
joined to a filesystem path. Missing built assets return 503. This checkout-based
asset location is local development only; G16 packaging must provide deployed
assets explicitly. No saved games, credentials, source files or dotfiles are
served. Static responses carry their MIME type, `nosniff` and revalidation headers.
