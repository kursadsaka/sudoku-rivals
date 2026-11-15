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
