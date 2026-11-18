# Sudoku Rivals Android simgesi

Kaynak: `apps/web/icon.svg` (marka mavisi `#1d4ed8` zemin, beyaz kart üzerinde mavi kutu çizgileri ve kehribar orta hücre,
arkasında 12° eğik kehribar "rakip" kartı). Dış asset yok.

- `ic_launcher_master.svg`: web simgesinin köşesi yuvarlatılmamış tam kanama hâli (192×192; yalnız dış `rx="42"` kaldırılır).
- `ic_launcher-playstore.png`: Play Console 512×512 RGBA mağaza simgesi:
  `rsvg-convert -w 512 -h 512 ic_launcher_master.svg -o ic_launcher-playstore.png && magick ic_launcher-playstore.png PNG32:ic_launcher-playstore.png`.
  Google köşeleri kendisi yuvarlar; önceden yuvarlatma. Mağazaya yükleme bu depoda yapılmaz.
- Uygulama içi uyarlanabilir simge (`app/src/main/res`): `mipmap-anydpi/ic_launcher{,_round}.xml` (arka plan rengi + ön plan + Android 13 tek
  renkli katman), `drawable/ic_launcher_{foreground,monochrome}.xml`, `values/ic_launcher_background.xml`. Web çizimi 108 dp tuvalin ortasına
  0,41 ölçekle yerleşir: beyaz kart ~47 dp, eğik kart dahil her şey 72 dp güvenli dairenin içindedir. Tek renk katmanda eğik kart yarı saydam,
  ön kart çerçeve + kutu çizgileri + dolu orta hücredir. minSdk 26 olduğundan raster mipmap gerekmez.
