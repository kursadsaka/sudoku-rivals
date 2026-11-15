"""Static checks for the web install icons (PNG sizes/content, manifest, HTML, service worker, server allowlist).

No browser or network: PNGs are decoded with the standard library. Run: python3 -m unittest tests/test_web_icons.py
"""
import json
import math
import pathlib
import re
import struct
import unittest
import zlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
WEB = ROOT / 'apps/web'
BLUE = (0x1d, 0x4e, 0xd8)
ICONS = {'icon-192.png': 192, 'icon-512.png': 512, 'icon-maskable-512.png': 512, 'apple-touch-icon.png': 180}


def read_png(path):
    data = path.read_bytes()
    assert data[:8] == b'\x89PNG\r\n\x1a\n'
    pos, idat, header = 8, b'', None
    while pos < len(data):
        length, kind = struct.unpack('>I4s', data[pos:pos + 8])
        chunk = data[pos + 8:pos + 8 + length]
        if kind == b'IHDR': header = struct.unpack('>IIBBBBB', chunk)
        elif kind == b'IDAT': idat += chunk
        pos += 12 + length
    width, height, depth, color_type, _, _, interlace = header
    assert depth == 8 and interlace == 0 and color_type in (2, 6), header
    channels = 3 if color_type == 2 else 4
    raw, stride, rows, previous = zlib.decompress(idat), width * channels, [], bytearray(width * channels)
    for y in range(height):
        kind, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - channels] if i >= channels else 0
            b, c = previous[i], (previous[i - channels] if i >= channels else 0)
            if kind == 1: line[i] = (line[i] + a) & 255
            elif kind == 2: line[i] = (line[i] + b) & 255
            elif kind == 3: line[i] = (line[i] + (a + b) // 2) & 255
            elif kind == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(line); previous = line
    return width, height, channels, rows


def pixel(image, x, y):
    width, _, channels, rows = image
    r = rows[y][x * channels:(x + 1) * channels]
    return tuple(r) if channels == 4 else (*r, 255)


def white_extent(image):
    """(max distance of any near-white pixel from the centre, bounding box) in pixels."""
    width, height, channels, rows = image
    far, xs, ys = 0.0, [], []
    for y in range(height):
        row = rows[y]
        for x in range(width):
            r, g, b = row[x * channels:x * channels + 3]
            if r > 160 and g > 160 and b > 160:
                far = max(far, math.hypot(x + .5 - width / 2, y + .5 - height / 2)); xs.append(x); ys.append(y)
    return far, (min(xs), min(ys), max(xs), max(ys))


class WebIconTests(unittest.TestCase):
    def test_png_sizes_and_formats(self):
        for name, size in ICONS.items():
            with self.subTest(icon=name):
                width, height, channels, _ = read_png(WEB / name)
                self.assertEqual((width, height), (size, size))
                self.assertLess((WEB / name).stat().st_size, 64 * 1024)
                # Opaque icons carry no alpha channel at all; the "any" icons keep theirs for the rounded corners.
                self.assertEqual(channels, 3 if name in ('icon-maskable-512.png', 'apple-touch-icon.png') else 4)

    def test_any_icons_keep_the_rounded_square_of_icon_svg(self):
        for name in ('icon-192.png', 'icon-512.png'):
            image = read_png(WEB / name)
            size = image[0]
            self.assertEqual(pixel(image, 0, 0)[3], 0, 'transparent corner')
            self.assertEqual(pixel(image, int(size * .1), size // 2), (*BLUE, 255))
            far, box = white_extent(image)
            self.assertAlmostEqual((box[0] + box[2]) / 2, size / 2, delta=2)

    def test_maskable_icon_is_full_bleed_with_content_in_the_safe_zone(self):
        image = read_png(WEB / 'icon-maskable-512.png')
        for corner in ((0, 0), (511, 0), (0, 511), (511, 511)):
            self.assertEqual(pixel(image, *corner)[:3], BLUE)
        far, box = white_extent(image)
        self.assertLessEqual(far, 512 * .40, 'content leaves the 80% safe circle')
        self.assertAlmostEqual((box[0] + box[2]) / 2, 256, delta=2)
        self.assertAlmostEqual((box[1] + box[3]) / 2, 256, delta=2)
        self.assertGreater(box[2] - box[0], 512 * .5, 'content should not be tiny')

    def test_apple_touch_icon_is_opaque_and_full_bleed(self):
        image = read_png(WEB / 'apple-touch-icon.png')
        for corner in ((0, 0), (179, 0), (0, 179), (179, 179)):
            self.assertEqual(pixel(image, *corner)[:3], BLUE)

    def test_manifest_icons_exist_with_matching_type_and_size(self):
        manifest = json.loads((WEB / 'manifest.webmanifest').read_text())
        purposes = {}
        for icon in manifest['icons']:
            path = WEB / icon['src'].lstrip('/')
            self.assertTrue(path.is_file(), icon['src'])
            if icon['type'] == 'image/png':
                width, height, _, _ = read_png(path)
                self.assertEqual(f'{width}x{height}', icon['sizes'])
            else:
                self.assertEqual((icon['type'], icon['sizes']), ('image/svg+xml', 'any'))
            purposes.setdefault(icon['purpose'], []).append(icon['sizes'])
        self.assertIn('192x192', purposes['any'])
        self.assertIn('512x512', purposes['any'])
        self.assertEqual(purposes['maskable'], ['512x512'])
        self.assertNotIn('any maskable', purposes, 'the rounded icon.svg must not be offered as maskable')
        # Other manifest fields are unchanged.
        self.assertEqual((manifest['start_url'], manifest['scope'], manifest['display'], manifest['theme_color']), ('/', '/', 'standalone', '#ffffff'))

    def test_html_links_the_icons(self):
        html = (WEB / 'index.html').read_text()
        self.assertIn('<link rel="icon" href="/icon.svg" type="image/svg+xml"', html)
        self.assertIn('<link rel="apple-touch-icon" href="/apple-touch-icon.png">', html)
        self.assertIn('<link rel="manifest" href="/manifest.webmanifest">', html)

    def test_every_icon_is_served_cached_and_packaged(self):
        names = list(ICONS) + ['icon.svg']
        server = (ROOT / 'server/src/http.rs').read_text()
        worker = (WEB / 'sw.js').read_text()
        build = (ROOT / 'scripts/staging-build.sh').read_text()
        for name in names:
            with self.subTest(icon=name):
                mime = 'image/svg+xml' if name.endswith('.svg') else 'image/png'
                self.assertIn(f'("{name}", "{mime}")', server)
                self.assertIn(f"'/{name}'", worker)
                self.assertRegex(build, rf'(?<![\w.-]){re.escape(name)}(?![\w.-])')

    def test_source_vector_is_unchanged_and_script_documents_it(self):
        svg = (WEB / 'icon.svg').read_text()
        self.assertIn('rx="42" fill="#1d4ed8"', svg)
        self.assertIn('apps/web/icon.svg', (ROOT / 'scripts/web-icons.sh').read_text())


if __name__ == '__main__':
    unittest.main()
