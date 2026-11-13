# Sudoku Rivals — devam kaydı

Kaynak: Sistem konuşması, 13 Kasım 2026 goal mesajı (5598) ve sonrasındaki kararlar (son kayıt 5916). Bu checkout üzerinde devam edilir; mevcut G03/G04 commit'leri korunur. Hesap/ödeme, mağaza gönderimi, production ve merge yetkilendirilmiş değildir.

## Kesin kapsam ve çalışma düzeni

Web, Android (Compose), iOS (SwiftUI); üç modun tamamı 6×6 (2×3 kutu) ve 9×9 (3×3 kutu). Solo çevrimdışı; diğer modlar davetli 1v1. Yarışta ilk doğru tamamlayan kazanır; düello 90 saniyedir. Ortak Rust çekirdek; UniFFI/WASM; Axum sunucu. Exact/Expo kullanılmaz. Platformların öğeleri ve okuma sırası ortaktır, görünüm platforma uygundur.

Son kayıtlı tercih: geliştirme Sol high veya Opus 5.5 high; web/Android'de önce Codex B, Apple'da Codex, bağımsız incelemede Claude. Kota önceliği Sistem Codex kotasını korumaktır. Bu thread ilk olarak G04 işçisi olarak açıldı. Kullanıcı 13 Kasım devam turunda yeni işçi açmaya açıkça izin verdi; G04 için Claude Opus 5.5 xhigh read-only inceleme ve G06 için Codex B / Sol high geliştirme işçisi başlatıldı. G04 bağımsız inceleme tamamlandı; bulunan beş madde parent tarafından regresyon testleriyle düzeltildi. G06 bağımsız incelemesi başlatılıyor; G07 istemci ve G11 web canlı ekranları paralel ilerliyor. Notes'a canlı bağlantı yok; yukarıdaki tercihler konuşmadan okunmuştur.

## Durum ve kanıt

| Adım | Durum | Kanıt / kalan |
| --- | --- | --- |
| G01 | Taslak + inceleme düzeltmeleri var | `docs/rules.md`, `docs/rules-reviewed-amendments.md`; puan dengesi insanlarla doğrulanmadı |
| G02 | Başlangıç mimarisi seçildi | Rust/UniFFI/WASM + Axum; yerel araçlar `../sudoku-rivals/.tools`; envanterdeki ilk PATH yokluğu güncel kurulum durumu değildir |
| G03 | Dört ortamda ilk iskelet doğrulandı | c5133b1 çekirdeği; web, Android x86_64 emülatör, sunucu ve iPhone 18 Pro/iOS27.2 simülatörü. Mac native Swift probe20PASS. iOS build düzeltmesi7598738 incelenip kaynak yamasıyla bu checkout'a alındı; yeni hint/generator/UI henüz iOS'ta doğrulanmadı |
| G04 | Çekirdek ve mantıksal ipuçları uygulandı | Belirlenimli tek çözümlü üretici/çözücü, görünür çakışma indeksleri, açık/gizli tekli ipucu. 512 tohum + native/WASM seed vektörleri geçti. Claude incelemesinin SearchLimit, InvalidCommand, future-version, WASM sayı aralığı ve strict undo/redo bulguları testle düzeltildi. İnsan zorluk kalibrasyonu ve ileri teknikler açık |
| G05 | Uygulama sözleşmesi hazır | `docs/ui-contract.md` ve `docs/ui-contract.json`; ekran bağlantıları, mod kontrolleri, TR/EN şablonları ve kontrast `node tests/ui-contract.mjs` ile doğrulandı. Gerçek UI ve görsel onay açık |
| G06 | Yerel sunucu uygulandı | 14 durum testi ve 2 gerçek HTTP testi geçti. Tam maç 2 mod × 2 boyut; replay/generation/deadline/40edit-sync/kopma/skor/gizlilik. Explicit leave koltuğu serbest bırakır. RAM-only, loopback. Bağımsız inceleme bulguları (geri sayımda gizli tahta, koltuk başı kayıt kotası, büyük komut saklamama, giriş zamanı) ve aynı guest ile idempotent create/join tekrarı regresyon testleriyle düzeltildi |
| G07 | Ortak istemci uygulandı | Private state+typedJSON dispatch, seq/gen/retry/clock/draft/reconcile. 12 core + 16 serialized Service testi geçti. Claude incelemesinin dört bulgusu (reddedilen create/join'in oturumu bozması, kayıp yanıtta güvensiz tekrar, bitmiş yarış taslağının rövanşı kilitlemesi, genel çevrimdışı sinyalinin session_lost'u silmesi) testle düzeltildi; `last_error` view alanı eklendi. Gerçek platform runtime kanıtı açık |
| G08 | Web solo uygulandı | Gerçek WASM Node testleri; T3 preview6/9tamamlama,320/375px,büyükfont,koyu,klavye,not/undo/hint; sunucu gerçekten kapalıyken coldreload/resume+yeni9. Fiziksel ekran okuyucu ve zorluk menüsü açık |
| G09 | Android solo uygulandı | Son APK320dp runtime suite geçti: offline6/9, tahta+sayılarbirlikte,>=48dp sayı hedefleri, given/not/erase/undo-redo, coldresume/pauseprivacy, AtomicFile writefailure/retry, ayrı kayıtlar, bozuk yedek, hint/tamamlama. TürkçeUI. İnceleme bulgusu düzeltildi: `configChanges` ile döndürme/gece modu oyunu kaybetmiyor; emülatör testine eklendi ve geçti. Gerçek cihaz/TalkBack/büyükfont açık |
| G10 | Çalışıyor | Mac Claude işçisi b8008a5 tabanında native SwiftUI solo ve XCTest/XCUITest yazdı; devam eden simülatör koşusu henüz sonuç kanıtı değildir |
| G11 | Çalışıyor | Web aynı origin maç adaptörü, davet/lobi/yarış/düello ekranları; G07 state ortak |
| G12–G13 | Bekliyor | Android ve iOS canlı maç arayüzleri |
| G14–G15 | Bekliyor | Gerçek platform matrisi ve erişilebilirlik |
| G16 | Hazırlık yapılabilir | Staging sağlayıcı, alan adı ve dağıtım onayı gerekli |
| G17–G18 | Bekliyor | İnsan/gerçek cihaz, hesap ve imzalama; mağaza gönderimi yapılmayacak |

## Solo durum API'sinin mevcut varsayılanları

Çakışan oyuncu sayıları tutulur; doğrulama görünür tahta kısıtlarını denetler. Notlar manueldir; sayı yazmak yalnız o hücrenin notlarını temizler. Hatalar atomiktir. JSON kayıt sürümü 1; yükleme sınırı 1 MiB; undo+redo toplamı 4096 işlemle sınırlıdır ve eskiler atılır. Bunlar uygulama varsayılanlarıdır; ilk G01 taslağındaki sınırsız/500 geri alma ve otomatik eş notu temizleme önerilerinin uygulandığı iddia edilmez. Kayıt string'inin cihaza güvenle yazılması platform işidir.

## Sonraki işler

1. G04/G05/G06 ve web solo b8008a5 ile çalışma dalına push edildi. G06 bağımsız incelemesini yap; G07 client testlerini bitir.
2. G05 sözleşmesi `docs/ui-contract.*` içinde hazır; uygulama ve gerçek erişilebilirlik testleriyle karşılaştır.
3. G07 ortak client tamamlanınca üç platformun canlı maç entegrasyonunu bağla; G10 native iOS solo UI'sini güncel core ile doğrula.
4. G07 ve platform arayüzleri; iOS/Mac ve bağımsız inceleme engelleri diğer işleri durdurmaz.

Başarı kanıtı üretilmeden bir adım tamamlandı sayılmaz. Bu dosya çalışma ilerledikçe güncellenir.

## 14 Kasım devamı

Yerel üç Codex B işçisi 00:27 kullanım limitine takıldı; yenilenme saati geçtikten sonra aynı kaynaklardan devam ettirildi. Mac Codex daha önce kullanım limitine takıldığı için Apple işçisi kullanıcı tercihindeki Claude fallback ile çalışıyor. G10 tek tur sonucu testin bittiğini kanıtlamadı; thread'in hâlâ aktif olduğu doğrulandı, test yeniden başlatılmadı.

Parent G11 için `/health`, allowlist statik varlık sunumu ve `/r/CODE` girişi ekledi; aynı origin `/v1` kullanılır. Dosya sızmasını engelleyen gerçek HTTP testleri eklendi. Ortak istemci object bağlamaları UniFFI/WASM tarafında yazıldı; `cargo check --workspace` geçti. Bunlar henüz G07 bot testlerinin geçtiği anlamına gelmez.

Kullanıcının Chromium Modern Web Guidance 1.0 ve Vercel UI SDK sorusu resmî kaynaklarla yanıtlandı: ikisi de şu an kurulu/kullanılıyor değil. MWG geliştirme rehberi, UI SDK beta davranış kütüphanesi olarak ayrıldı; soru tek başına yeni bağımlılık veya skill kurulumu yetkisi olarak yorumlanmadı.
