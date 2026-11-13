# G07 — ortak çok oyunculu istemci

`core/src/multiplayer.rs` bütün platformlar için saf Rust durum makinesidir. İşletim sistemi HTTP I/O'su, zamanlayıcısı ve güvenli kalıcı depolama platform adapterinin işidir. Maç kuralları ve sonuçlar sunucudadır. DTO'lar `core/src/protocol.rs`, sunucu sözleşmesi `docs/match-protocol.md` içindedir.

## Binding API

- `MultiplayerClient::new(seed: u32)`: yalnız yeniden deneme jitter'ı için tohum. Bu tohum kimlik veya kriptografik sır üretmez.
- `MultiplayerClient::load(saved: &str)`: sürümlü kaydı doğrular; aktif nesneyi yalnız başarıdan sonra değiştirin.
- `dispatch_json(action: &str, now_ms: u64) -> Result<String, ClientError>`.
- `view_json(now_ms: u64) -> Result<String, ClientError>`: kimlik bilgisi içermeyen sunum verisi.
- `save() -> Result<String, ClientError>`: kimlik bilgisi içeren **opaque yerel kayıt**, günlük veya paylaşım çıktısı değildir.

`now_ms`, tek istemci yaşamı boyunca aynı **monotonik** saatten gelen tam sayı milisaniyedir. JavaScript'te örneğin `Math.floor(performance.now())`; native'de monotonik uptime kullanılır. Duvar saati veya UTC damgası değildir. Bütün zaman/sıra değerleri JS'nin güvenli tam sayı sınırı `2^53−1` içindedir. Kaydı yüklemek saat ölçümlerini sıfırlar; yeni monotonik epoch ile önce `resume` gerekir. Save/load her eylemde nesne kurmak için kullanılmaz.

Dispatcher `{request: null | RequestDescriptor, changed: bool}` JSON'u döndürür. Descriptor:

```json
{"id":1,"method":"POST","path":"/v1/rooms","body":"{\"protocol\":1,...}"}
```

Adapter yalnız `method/path/body` ile HTTP yapar. Body string'ini dönüştürmez, loglamaz. Yanıtın ham metnini `receive` eylemine verir. HTTP hatasında bile gövdeyi başarı veya oyun sonucu gibi yorumlamaz; JSON olmayan HTTP yanıtı dispatcher tarafından reddedilirse adapter görünür bağlantı hatası sunabilir ve `network_failed` bildirebilir. Sunucu host/origin seçimi adapterdedir; descriptor dış URL veya yetkisiz header taşımaz.

## Eylemler

Bütün eylemler JSON nesnesi ve `type` alanıdır. Bilinmeyen alanlar ve hatalı tipler reddedilir. Geçersiz eylem/yanıt aktif durumu, yerel tahtayı veya bekleyen isteği değiştirmez.

| type | Diğer alanlar | Sonuç |
| --- | --- | --- |
| `create` | `name`, `mode: race|duel`, `size:6|9`, önerilen `guest` | Oda oluşturma isteği |
| `join` | `invite`, `name`, önerilen `guest` | Kod/davet doğrulama ve katılım isteği |
| `resume` | — | Yeni bağlantı nesli ve güncel görüntü isteği; eski transport callback'lerini geçersiz kılar |
| `poll` | — | Heartbeat ve güncel sunucu görüntüsü isteği |
| `sync` | — | Yarışta son onaylı sürüm + bütün yerel hücreleri tek komutta gönderir |
| `command` | `command`: protokol Command nesnesi | `ready/move/resign/rematch/lobby/disconnect`; bütün komutlar sunucuya gider |
| `leave` | — | İdempotent oda terk isteği; veri, yanıt gelmeden atılmaz |
| `receive` | `request_id`, `status`, `response`: ham metin string'i | İlgili isteğin sunucu yanıtı |
| `network_failed` | isteğe bağlı `request_id` | Bağlantı durumu ve sonraki yeniden deneme zamanı |
| `retry` | — | Bekleyen aynı descriptor ve **aynı body** tekrar gönderilir; create/join için güvenli olması `guest` gerektirir |
| `edit` | `command`: SoloCommand nesnesi | Yerel yarış tahtasında sayı/not/silme/undo/redo; ağ kapalıyken de çalışır |
| `reconcile` | `keep_local`: bool | Kullanıcının görünür veri uyuşmazlığını açık seçimiyle çözmesi |

HTTP callback hatalarında `network_failed` için **ilgili descriptor.id** verilir; geçersizleşmiş eski isteğin hatası yeni oturumu koparamaz. `request_id` verilmemesi, platformun genel ağ kapandı bildirimi içindir.

`sync`, gerçek yerel SoloGame'den üretilir. Generic `command` içine elle `sync` verilirse hücrelerin yerel tahta ve `base_version` değerinin onaylı sürümle aynı olması gerekir. Böylece adapter ikinci bir yarış durumu veya sıra sayacı tutmaz. Notlar ve undo geçmişi SoloGame'dedir, sunucuya gönderilmez. `view.race.solo` bu SoloGame'nin auth içermeyen JSON nesnesidir.

## View

Üst alanlar: `connection`, `snapshot`, `race`, `last_receipt`, `pending_command`, `pending_http`, `reconcile_required`, `retry_at_ms`, `poll_due`, `estimated_server_ms`, `remaining_ms`, `locked_remaining_ms`, `clock_samples`, `voluntarily_left`.

- `connection`: `connected/disconnected/reconnecting/session_lost/reconcile`.
- `last_error`: reddedilen create/join'in sunucu hata kodu (`room_full`, `room_missing`, `invalid_input`, `rate_limited`…); başarılı yanıtta `null` olur.
- `snapshot`: doğrulanmış protokol Snapshot'ı veya null. Rakipte yalnız sunucunun izin verdiği alanlar vardır. Guest/token ve request body burada yoktur.
- `race`: `{cells, notes: digit[][], solo, conflicts: index[], can_undo, can_redo, dirty}` veya null. `dirty`, hücrelerin son onaylı sunucu hücrelerinden farkını anlatır; notlar cihazda kalır.
- `last_receipt`: en son doğrudan doğrulanmış Receipt; bütün sayaçları ve kimliği sunucudan gelir. Rövanşı açan Receipt eski maçın, beraberindeki Snapshot yeni maçın olabilir; maç kimlikleri karşılaştırılır.
- `poll_due`: bağlı istemcide lobide/geri sayımda 500 ms, oyunda 1000 ms aralıkla; ayrıca geri sayım başlangıcı geçince görüntü istemeyi önerir. Gerçek I/O adapter tarafından yapılır. Session-lost durumda otomatik poll önerilmez.

Tahmini kalan süre sıfıra inse bile istemci kazanan veya maç sonucu üretmez. Kesin sonuç yalnız `snapshot.result` alanıdır. RTT örnekleri son beş ölçümden en düşük RTT olanı seçer; offset, monotonik gönderim/alım orta noktasından hesaplanır. Yeniden gönderilmiş isteğin belirsiz RTT'si örneğe katılmaz. p95 senkronizasyon hedefi bu algoritmanın varlığıyla doğrulanmış sayılmaz.

## Kayıp yanıt ve yeniden bağlanma

Bir yeni komut beklerken ikinci komut oluşturulamaz. Yerel yarış düzenlemeleri sürer. Kayıp yanıt için 0.5, 1, 2, 4, 5 saniye; sonra 5 saniye tabanıyla ±%20 belirlenimli jitter üretilir. `retry_at_ms` gelmeden retry ve bağlantı yenileme reddedilir. Aynı komut tekrarında sıra veya gövde değiştirilmez. Sunucunun dondurulmuş Receipt'i ve güncel Snapshot birlikte doğrulanır.

Bir `poll`, bekleyen komutun `next_sequence` tarafından zaten işlendiğini gösterirse komut yeniden gönderilmez; sunucu görüntüsü esas alınır. `resume` yeni generation getirir. Eski request-id, generation, sunucu olay sırası veya sunucu zamanı içeren yanıtlar yeni durumu ezemez. Düelloda eski nesle ait işlenmemiş hamleler bırakılır. Yarışta yerel tahta/not/geçmiş korunur; son onaylı sürümden tek eşitleme yapılır.

Eşitleme gönderildikten sonra kullanıcı yeni hücre düzenlerse, eşitlemenin onayı yalnız onaylı tabanı ileri taşır; daha yeni düzenlemeler ve undo geçmişi kalır. Farklı sunucu tahtası yerel işi/notları/geçmişi etkiliyorsa `reconcile` gerekir. Kullanıcı “yereli koru” derse son sunucu sürümüne karşı açıkça overwrite etmeyi seçmiş olur. “Sunucuyu kullan” derse yerel tahta açık seçimle yeniden kurulur. Farklı bulmacanın yerel tahtası yeni bulmacaya zorla uygulanmaz; “yereli koru” bu durumda reddedilir. Opaque save eski veriyi saklamak için alınabilir.

`room_missing/room_closed/unauthorized` session-lost olur; sonuç uydurulmaz ve yerel yarış verisi korunur. Başarılı explicit `leave`, bunu `voluntarily_left:true` ile ayırır. Eski çalışma hemen silinmez; kullanıcı yeni create/join seçerse buna izin verilir ve yalnız yeni Session başarıyla geldiğinde aktif eski çalışma değiştirilir. Yeni oda isteği başarısız olursa eski tahta hâlâ vardır.

## Kaydetme ve doğrulama

Kayıt sürümü 1, üst sınır 1 MiB; action/response JSON üst sınırı 128 KiB. Sırlar kayıtta gereklidir; platform bu metni korumalı, paylaşmamalı ve loglamamalıdır. Debug/Display hata çıktıları sır içermez. Yeni sürüm veya bozuk veri başarı gibi yüklenmez. Pending HTTP callback'i ve saat örnekleri restart'a taşınmaz; belirsiz komut kimliği korunur ve resume ile uzlaştırılır.

Snapshot güven sınırı: protokol, mod/boyut, kod ve kimlik biçimleri, JS-safe sayılar, generation/sıra, koltuk, tahta uzunluğu/rakamlar, sabit hücreler, zaman/phase/end/result tutarlılığı doğrulanır. Yarışta oyuncu çakışmaları kabul edilir; düelloda çatışmalı sunucu tahtası kabul edilmez. Gönderilen komutun Receipt kimliği, olay/zaman sınırları ve ilgili sayaçları doğrulanır. Deserialize türetmesi tek başına güvenlik kontrolü sayılmaz.

## Kanıt ve kalan

`cargo test -p sudoku-core --test multiplayer -p sudoku-server --test client --locked --offline` core testleri ve serileştirilmiş istemci↔Service bot testlerini çalıştırır. İki mod × iki boyutta tam maç; exact retry/tek yanlış cezası; eski callback ve generation; rövanş kimliği; 40 offline düzenleme + tek eşitleme; pending sırasında yeni düzenleme; divergent sunucu tahtası; not/geçmiş koruma; sunucu restart; explicit leave sonrası yeni oda; bozuk/future kayıt ve özel verisiz view kapsanır.

Bu testler saf Rust istemci ve gerçek sunucu durum makinesini kullanır; HTTP adapteri G06'da ayrı test edilmiştir. Web WASM, Android emülatörü ve iOS simülatöründe bu ortak istemcinin arayüzsüz çalışma kanıtı platform entegrasyonunda ayrıca gerekir. Gerçek ağ gecikmesi ve mobil arka plan/kill senaryoları burada doğrulanmış sayılmaz.

Geri sayım görüntülerinde `own.cells` ve `own.givens` boştur; tahta yalnız
`playing` ile açılır. Yeni maçın gizli geri sayımına geçildiğinde eski onaylı
race draft temizlenir; onaysız yerel düzenleme varsa korunup açık uzlaştırma
istenir. Ekranlar geri sayımda eski maçın tahtasını sunmaz.

## Create/join tekrarı

Adapter, ilk create/join'de kriptografik rastgele 64-hex `guest` gönderir (web `crypto.getRandomValues`, Android `SecureRandom`); mevcut oturum varsa çekirdek onun guest'ini kullanır. Yanıtı kaybolan istek aynı body ile tekrarlandığında sunucu, aynı guest'in lobideki koltuğunu (ad, mod ve boyut aynıysa) aynı Session ile döner; ikinci oda veya hayalet koltuk açılmaz. Ağ hatasıyla düşmüş create/join yerine yenisi başlatılabilir; çekirdek önceki guest'i korur.

Sunucunun reddettiği create/join (4xx) mevcut oturuma, bağlantı durumuna ve kayda dokunmaz; otomatik tekrar planlanmaz, kod `last_error` ile gösterilir. Bitmiş/kapanmış maçtan kalan gönderilmemiş yarış tahtası yeni oda veya rövanşı engellemez; eski maçın hücreleri zaten gönderilemez. Genel `network_failed`, kaybolmuş oturumu yeniden bağlantı denemesine çevirmez.
