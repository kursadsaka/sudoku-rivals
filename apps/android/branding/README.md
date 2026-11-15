# Sudoku Rivals Android simgesi

Kaynak: `apps/web/icon.svg` (marka mavisi `#1d4ed8`, beyaz 3×3 ızgara, ortada dolu hücre). Dış asset yok.

- `ic_launcher_master.svg`: köşesi yuvarlatılmamış tam kanama ana çizim (192×192). Web ikonunun maskeleme için yuvarlatılmamış hâli.
- `ic_launcher-playstore.png`: Play Console 512×512 mağaza simgesi; `rsvg-convert -w 512 -h 512 ic_launcher_master.svg -o ic_launcher-playstore.png`.
  Google köşeleri kendisi yuvarlar; önceden yuvarlatma.
- Uygulama içi uyarlanabilir simge (`app/src/main/res`): `mipmap-anydpi/ic_launcher{,_round}.xml` (arka plan rengi + ön plan + Android 13 tek renkli katman),
  `drawable/ic_launcher_{foreground,monochrome}.xml`, `values/ic_launcher_background.xml`. 108 dp tuvalde ızgara 48 dp ve 66 dp güvenli daire içinde.
  minSdk 26 olduğundan raster mipmap gerekmez.
