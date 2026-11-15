# iOS app icon

`SudokuRivals/Assets.xcassets/AppIcon.appiconset/icon-1024.png` is the single 1024×1024 App Store icon
(opaque, no alpha channel, square: iOS rounds the corners itself), rendered from the shared brand source
`apps/android/branding/ic_launcher_master.svg` (192×192 full-bleed; brand blue `#1d4ed8`, white 3×3 grid,
filled centre cell; same drawing as `apps/web/icon.svg` without rounded corners). No other asset is used.

Regenerate (any rasterizer works; this is what was used, headless Chrome at scale 1):

    printf '<!doctype html><style>html,body{margin:0;background:#1d4ed8}img{display:block;width:1024px;height:1024px}</style><img src="ic_launcher_master.svg">' > page.html
    chrome-headless-shell --headless --hide-scrollbars --force-device-scale-factor=1 --window-size=1024,1024 --screenshot=icon-1024.png file://$PWD/page.html
    sips -g hasAlpha icon-1024.png   # must say: hasAlpha: no
