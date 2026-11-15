"""Per-app language for emulator tests (Android 13+ `cmd locale`).

The app now follows the system/per-app language and ships English (default) and Turkish, so tests that
read Turkish UI text pin Turkish first and restore the previous per-app setting afterwards.
"""
import re


def get(shell, package):
    match = re.search(r'\[(.*)\]', shell('cmd', 'locale', 'get-app-locales', package))
    return match.group(1).strip() if match else ''


def set(shell, package, tags):
    """`tags` like 'tr-TR' or 'en-US'; an empty string clears the override (follow the system language)."""
    shell('cmd', 'locale', 'set-app-locales', package, '--locales', tags or "''")


def load_strings(lang):
    """(strings, plurals) of values (lang 'en') or values-tr (lang 'tr'), decoded from the Android XML."""
    import pathlib
    import xml.etree.ElementTree as ET
    root = ET.parse(pathlib.Path(__file__).resolve().parent.parent / 'apps/android/app/src/main/res' / ('values' if lang == 'en' else 'values-' + lang) / 'strings.xml').getroot()
    unescape = lambda text: (text or '').replace("\\'", "'")
    strings = {e.attrib['name']: unescape(e.text) for e in root.findall('string')}
    plurals = {e.attrib['name']: {i.attrib['quantity']: unescape(i.text) for i in e.findall('item')} for e in root.findall('plurals')}
    return strings, plurals


def fmt(template, *values):
    return re.sub(r'%(\d+)\$[sd]', lambda m: str(values[int(m.group(1)) - 1]), template.replace('%%', '%'))
