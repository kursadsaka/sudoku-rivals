# sudoku-rivals

Geliştirme iskeleti ve çalıştırma adımları: [G03.md](G03.md).

Güncel kapsam ve kanıt: [PROGRESS](docs/PROGRESS.md). Uygulama kuralları:
[G01](docs/rules.md), ortak ekran/öğe sözleşmesi [G05](docs/ui-contract.md),
yerel maç protokolü [G06](docs/match-protocol.md).

Web kaynakları `apps/web` içindedir. WASM üretildikten sonra `node tests/wasm.mjs`,
`node tests/web-solo.mjs` ve `node tests/ui-contract.mjs` çalıştırılır. Offline
solo ilk başarılı varlık yüklemesinden sonra kullanılabilir. Native/gerçek cihaz
ve mağaza durumları ilerleme kaydında ayrı belirtilir.

Render Free staging paketi ve sınırlı ortak giriş kotası:
[G16 kurulum ve doğrulama](docs/staging-render.md).
