"""Static localization checks for the Android app (English default, Turkish). No device needed.

Run: python3 -m unittest tests/test_android_i18n.py   (from the repository root or tests/)
"""
import pathlib
import re
import unittest
import xml.etree.ElementTree as ET

APP = pathlib.Path(__file__).resolve().parent.parent / 'apps/android/app/src/main'
RES = APP / 'res'
KOTLIN = sorted((APP / 'java/com/sudokurivals').glob('*.kt'))
# Strings that are identical in both languages on purpose (format shells, brand, numbers).
SAME_IN_BOTH = {'lobby_mode_size', 'app_title', 'solo_mode_label', 'live_code_placeholder'}
# Literals that may stay in code: glyphs/icons drawn as text, separators, and developer-only tokens.
GLYPHS = {'‹', '✎', '↶', '↷', '?', '–', '!', '·', '·', ' ', '%02d:%02d', 'Sudoku'}


def load(locale):
    root = ET.parse(RES / locale / 'strings.xml').getroot()
    strings = {e.attrib['name']: e.text or '' for e in root.findall('string')}
    plurals = {e.attrib['name']: {i.attrib['quantity']: i.text or '' for i in e.findall('item')} for e in root.findall('plurals')}
    translatable = {e.attrib['name'] for e in root.findall('string') if e.attrib.get('translatable') != 'false'}
    return strings, plurals, translatable


def placeholders(text):
    return sorted(re.findall(r'%(?:\d+\$)?[sd]', text.replace('%%', '')))


class AndroidLocalizationTests(unittest.TestCase):
    def setUp(self):
        self.en, self.en_plurals, self.en_translatable = load('values')
        self.tr, self.tr_plurals, self.tr_translatable = load('values-tr')

    def test_same_keys_in_both_languages(self):
        self.assertEqual(sorted(self.en_translatable), sorted(self.tr_translatable),
                         'every translatable string needs an English and a Turkish value')
        self.assertEqual(sorted(self.en_plurals), sorted(self.tr_plurals))
        self.assertEqual(sorted(set(self.en) - self.en_translatable), ['app_title'])

    def test_placeholders_match_between_languages(self):
        for key in self.en_translatable:
            with self.subTest(key=key):
                self.assertEqual(placeholders(self.en[key]), placeholders(self.tr[key]))
        for key, quantities in self.en_plurals.items():
            for quantity, text in quantities.items():
                with self.subTest(plural=key, quantity=quantity):
                    self.assertEqual(placeholders(text), placeholders(self.tr_plurals[key][quantity]))

    def test_plurals_have_one_and_other(self):
        for plurals in (self.en_plurals, self.tr_plurals):
            for key, quantities in plurals.items():
                with self.subTest(plural=key):
                    self.assertEqual(sorted(quantities), ['one', 'other'])
        # English must actually distinguish one from many; the format argument is the count.
        for key, quantities in self.en_plurals.items():
            self.assertNotEqual(quantities['one'], quantities['other'], key)

    def test_nothing_is_left_untranslated_or_empty(self):
        for key in self.en_translatable:
            with self.subTest(key=key):
                self.assertTrue(self.en[key].strip() and self.tr[key].strip())
                if key not in SAME_IN_BOTH and re.search(r'[A-Za-zÇĞİÖŞÜçğıöşü]{3}', self.en[key]):
                    self.assertNotEqual(self.en[key], self.tr[key], 'identical in both languages: ' + key)
        # The default (English) resources must not carry Turkish letters, nor the Turkish ones English-only prose.
        for key, text in self.en.items():
            self.assertIsNone(re.search('[çğışöüÇĞİÖŞÜ]', text), f'Turkish text in the default resources: {key}')

    def test_every_resource_is_used_and_every_use_exists(self):
        source = '\n'.join(path.read_text() for path in KOTLIN)
        used = set(re.findall(r'R\.string\.(\w+)', source)) | set(re.findall(r'R\.plurals\.(\w+)', source))
        defined = set(self.en) | set(self.en_plurals)
        self.assertEqual(sorted(used - defined), [], 'resources used in code but not defined')
        self.assertEqual(sorted(defined - used), [], 'resources defined but never used')

    def test_no_user_visible_literals_left_in_code(self):
        patterns = [r'Text\(\s*"([^"]*)"', r'label\s*=\s*"([^"]*)"', r'contentDescription\s*=\s*"([^"]*)"', r'title\s*=\s*"([^"]*)"',
                    r'onClick\(\s*"([^"]*)"', r'\bmessage\s*=\s*"([^"]*)"', r'(?:storageError|inputError|hintMessage)\s*=\s*"([^"]*)"']
        for path in KOTLIN:
            code = path.read_text()
            code = re.sub(r'/\*.*?\*/', '', code, flags=re.S)
            code = re.sub(r'//[^\n]*', '', code)
            for pattern in patterns:
                for literal in re.findall(pattern, code):
                    with self.subTest(file=path.name, literal=literal[:40]):
                        self.assertTrue(literal in GLYPHS or not re.search(r'[A-Za-zÇĞİÖŞÜçğıöşü]', re.sub(r'\$\{[^}]*\}|\$\w+', '', literal)),
                                        f'hard-coded text in {path.name}: {literal!r}')
            self.assertIsNone(re.search('[çğışöüÇĞİÖŞÜ]', re.sub(r'"[^"\n]*"', '', code)), path.name)
            for literal in re.findall(r'"([^"\n]*[çğışöüÇĞİÖŞÜ][^"\n]*)"', code):
                self.fail(f'Turkish literal left in {path.name}: {literal!r}')

    def test_locale_config_is_generated_for_per_app_language(self):
        gradle = (APP.parent.parent / 'build.gradle.kts').read_text()
        self.assertIn('generateLocaleConfig = true', gradle)
        self.assertEqual((RES / 'resources.properties').read_text().strip(), 'unqualifiedResLocale=en-US')

    def test_android_resource_syntax(self):
        for locale in ('values', 'values-tr'):
            text = (RES / locale / 'strings.xml').read_text()
            for key, value in re.findall(r'<string name="(\w+)"[^>]*>(.*?)</string>', text):
                with self.subTest(locale=locale, key=key):
                    self.assertIsNone(re.search(r"(?<!\\)'", value), 'unescaped apostrophe')
                    self.assertIsNone(re.search(r'%(?!\d+\$[sd])', value.replace('%%', '')), 'bare % must be %% or a positional format')


if __name__ == '__main__':
    unittest.main()
