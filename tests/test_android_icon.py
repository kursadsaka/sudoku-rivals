"""Static checks for the Android launcher icon (adaptive icon, monochrome layer, Play icon). No device needed."""
import math
import pathlib
import re
import struct
import unittest
import xml.etree.ElementTree as ET

ROOT = pathlib.Path(__file__).resolve().parent.parent
ANDROID = ROOT / 'apps/android'
RES = ANDROID / 'app/src/main/res'
NS = '{http://schemas.android.com/apk/res/android}'
CENTER, SAFE_RADIUS = 54.0, 36.0  # 108dp canvas; the launcher mask never cuts inside a 72dp circle


def path_points(data):
    """End points of an absolute M/L/H/V/A/Z path (the only commands the icon uses)."""
    tokens = re.findall(r'[MLHVAZ]|-?\d+(?:\.\d+)?', data)
    x = y = 0.0
    points, i = [], 0
    while i < len(tokens):
        command = tokens[i]; i += 1
        if command in 'ML': x, y = float(tokens[i]), float(tokens[i + 1]); i += 2
        elif command == 'H': x = float(tokens[i]); i += 1
        elif command == 'V': y = float(tokens[i]); i += 1
        elif command == 'A': x, y = float(tokens[i + 5]), float(tokens[i + 6]); i += 7
        elif command == 'Z': continue
        else: raise AssertionError('unsupported path command ' + command)
        points.append((x, y))
    return points


class LauncherIconTests(unittest.TestCase):
    def test_manifest_uses_the_adaptive_icon_for_every_variant(self):
        manifest = (ANDROID / 'app/src/main/AndroidManifest.xml').read_text()
        self.assertIn('android:icon="@mipmap/ic_launcher"', manifest)
        self.assertIn('android:roundIcon="@mipmap/ic_launcher_round"', manifest)
        # Debug/staging/release identities are unchanged.
        gradle = (ANDROID / 'app/build.gradle.kts').read_text()
        self.assertIn('applicationId = "com.sudokurivals"', gradle)
        self.assertIn('applicationIdSuffix = ".staging"', gradle)
        self.assertFalse((ANDROID / 'app/src/staging').exists() or (ANDROID / 'app/src/debug/res/mipmap-anydpi').exists())

    def test_adaptive_icon_layers(self):
        for name in ('ic_launcher', 'ic_launcher_round'):
            root = ET.parse(RES / 'mipmap-anydpi' / f'{name}.xml').getroot()
            self.assertEqual(root.tag, 'adaptive-icon')
            self.assertEqual({child.tag: child.attrib[NS + 'drawable'] for child in root},
                             {'background': '@color/ic_launcher_background', 'foreground': '@drawable/ic_launcher_foreground',
                              'monochrome': '@drawable/ic_launcher_monochrome'})

    def test_brand_color_matches_web_icon_and_ui_contract(self):
        color = ET.parse(RES / 'values/ic_launcher_background.xml').getroot().find('color').text.lower()
        web = (ROOT / 'apps/web/icon.svg').read_text()
        self.assertEqual(re.search(r'<rect width="192" height="192" rx="42" fill="(#[0-9a-f]{6})"', web).group(1), color)
        self.assertIn(f'"accent": "{color}"', (ROOT / 'docs/ui-contract.json').read_text())

    def test_foreground_stays_inside_the_safe_zone_and_is_centred(self):
        for name in ('ic_launcher_foreground', 'ic_launcher_monochrome'):
            root = ET.parse(RES / 'drawable' / f'{name}.xml').getroot()
            self.assertEqual((root.attrib[NS + 'viewportWidth'], root.attrib[NS + 'viewportHeight'], root.attrib[NS + 'width']), ('108', '108', '108dp'))
            xs, ys = [], []
            for path in root.findall('path'):
                half_stroke = float(path.attrib.get(NS + 'strokeWidth', 0)) / 2
                for x, y in path_points(path.attrib[NS + 'pathData']):
                    self.assertLessEqual(math.hypot(x - CENTER, y - CENTER) + half_stroke, SAFE_RADIUS, f'{name} leaves the safe zone at {(x, y)}')
                    # The front tile is centred, as on the web; the rival card behind it peeks out to the top right.
                    if path.attrib.get(NS + 'name') == 'tile': xs.append(x); ys.append(y)
            self.assertAlmostEqual((min(xs) + max(xs)) / 2, CENTER, delta=.01)
            self.assertAlmostEqual((min(ys) + max(ys)) / 2, CENTER, delta=.01)
            self.assertGreaterEqual(max(xs) - min(xs), 44)  # not a tiny glyph

    def test_foreground_uses_the_web_icon_colours_and_monochrome_single_colour(self):
        fg = (RES / 'drawable/ic_launcher_foreground.xml').read_text()
        web = (ROOT / 'apps/web/icon.svg').read_text()
        # White tile, blue box lines, amber rival card and centre: the web icon's own colours (blue is also the background).
        self.assertEqual(set(re.findall(r'(?:strokeColor|fillColor)="(#\w+)"', fg)), {c.upper() for c in re.findall(r'fill="(#[0-9a-f]{6})"', web.replace('#fff"', '#ffffff"'))})
        mono = (RES / 'drawable/ic_launcher_monochrome.xml').read_text()
        self.assertEqual(len(set(re.findall(r'(?:strokeColor|fillColor)="(#\w+)"', mono))), 1)

    def test_master_svg_is_the_web_icon_without_rounded_corners(self):
        web = (ROOT / 'apps/web/icon.svg').read_text().strip()
        master = (ANDROID / 'branding/ic_launcher_master.svg').read_text().strip()
        self.assertEqual(master, web.replace(' rx="42"', '', 1))

    def test_play_store_icon_is_512_rgba_png_under_1mb(self):
        data = (ANDROID / 'branding/ic_launcher-playstore.png').read_bytes()
        self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
        width, height, depth, color_type = struct.unpack('>IIBB', data[16:26])
        self.assertEqual((width, height, depth), (512, 512, 8))
        self.assertEqual(color_type, 6, '32-bit RGBA')
        self.assertLess(len(data), 1024 * 1024)


if __name__ == '__main__':
    unittest.main()
