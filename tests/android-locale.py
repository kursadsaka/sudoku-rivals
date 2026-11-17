#!/usr/bin/env python3
"""Basic screens in English and Turkish on a disposable emulator (debug APK, Android 13+ per-app language).

Expected texts are read from apps/android/app/src/main/res/values{,-tr}/strings.xml, so nothing is
duplicated here. Covers home, settings, solo setup, a solo 6x6 game (visible text and accessibility
labels), live setup, the lobby of a room created on a local server, and the leave dialog. Languages:
tr-TR, en-US, de-DE (unsupported: must fall back to English). `--system tr|en` instead clears the per-app
override and expects the emulator's own system language. The per-app language, solo/live records and
airplane mode are restored in `finally`.
"""
import argparse
import json
import os
import pathlib
import re
import socket
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import android_locale
from android_layout_check import nodes_by_tag

RES = HERE.parent / 'apps/android/app/src/main/res'
parser = argparse.ArgumentParser()
parser.add_argument('--adb', default=os.path.join(os.environ.get('ANDROID_HOME', ''), 'platform-tools/adb'))
parser.add_argument('--serial', default='emulator-5554')
parser.add_argument('--server-bin', default=str(HERE.parent / 'target/debug/sudoku-server'))
parser.add_argument('--server-port', type=int, default=3103)
parser.add_argument('--screenshots', type=pathlib.Path)
parser.add_argument('--system', choices=['tr', 'en'], help='Expect the emulator system language with no per-app override')
args = parser.parse_args()
PKG = 'com.sudokurivals'


def load(locale):
    root = ET.parse(RES / locale / 'strings.xml').getroot()
    return {e.attrib['name']: (e.text or '').replace("\\'", "'") for e in root.findall('string')}


STRINGS = {'en': load('values'), 'tr': load('values-tr')}


def fmt(template, *values):
    return re.sub(r'%(\d+)\$[sd]', lambda m: str(values[int(m.group(1)) - 1]), template.replace('%%', '%'))


def adb(*parts, check=True):
    return subprocess.run([args.adb, '-s', args.serial, *parts], stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=40, check=check).stdout


def shell(*parts): return adb('shell', *parts).decode().strip()
def read(name): return adb('exec-out', 'run-as', PKG, 'cat', 'files/' + name)
def write(name, data): subprocess.run([args.adb, '-s', args.serial, 'shell', '-T', 'run-as', PKG, 'tee', 'files/' + name], input=data, stdout=subprocess.PIPE, check=True, timeout=40)
def private_files():
    return [n for n in adb('exec-out', 'run-as', PKG, 'ls', '-1p', 'files', check=False).decode().splitlines() if re.fullmatch(r'(?:solo|live)-[\w.-]+', n)]


def launch(*extra):
    shell('am', 'start', '-W', '-n', PKG + '/.MainActivity', *extra)


def hierarchy():
    shell('uiautomator', 'dump', '/sdcard/sudoku-locale-check.xml')
    return ET.fromstring(adb('exec-out', 'cat', '/sdcard/sudoku-locale-check.xml'))


def visible():
    """(tag nodes, all visible texts and content descriptions)."""
    root = hierarchy()
    found = set()
    for node in root.iter('node'):
        for key in ('text', 'content-desc'):
            if node.attrib.get(key): found.add(node.attrib[key])
    return nodes_by_tag(root), found


def wait_for(tag, tries=40):
    for _ in range(tries):
        tags, _ = visible()
        if tag in tags: return tags
        time.sleep(.5)
    raise AssertionError('UI node never appeared: ' + tag)


def tap(tag):
    tags = wait_for(tag)
    x1, y1, x2, y2, _ = tags[tag]
    shell('input', 'tap', str((x1 + x2) // 2), str((y1 + y2) // 2))
    time.sleep(.5)


failures, checked = [], 0


def expect(label, shown, *wanted):
    global checked
    for text in wanted:
        checked += 1
        if text not in shown: failures.append(f'{label}: missing {text!r}')


def expect_prefix(label, shown, prefix):
    global checked
    checked += 1
    if not any(t.startswith(prefix) for t in shown): failures.append(f'{label}: nothing starts with {prefix!r}')


def shot(name):
    if args.screenshots:
        args.screenshots.mkdir(parents=True, exist_ok=True)
        (args.screenshots / f'{name}.png').write_bytes(adb('exec-out', 'screencap', '-p'))


def run_language(tag, lang, label):
    s = STRINGS[lang]
    other = STRINGS['tr' if lang == 'en' else 'en']
    # The two languages must really differ on what we assert (guards against a vacuous check).
    assert s['home_solo_start'] != other['home_solo_start']
    shell('am', 'force-stop', PKG)
    for name in private_files(): shell('run-as', PKG, 'rm', '-f', 'files/' + name)
    if tag is None: android_locale.set(shell, PKG, '')
    else: android_locale.set(shell, PKG, tag)
    launch('--es', 'dev_server_url', f'http://10.0.2.2:{args.server_port}')
    tags, shown = visible()
    for _ in range(20):
        if 'solo-start' in tags: break
        time.sleep(.5); tags, shown = visible()
    expect(f'{label} home', shown, s['home_title'], s['home_subtitle'], s['home_solo_start'], s['home_friend_start'], s['home_settings'])
    shot(f'{label}-home')
    tap('settings-open'); _, shown = visible()
    expect(f'{label} settings', shown, s['home_settings'], s['settings_body_system'], s['settings_body_storage'], s['action_back'])
    tap('back')
    tap('solo-start'); wait_for('game-start'); _, shown = visible()
    expect(f'{label} setup', shown, s['setup_title'], s['setup_note'], s['setup_start'])
    tap('board-size-6'); tap('game-start'); tags = wait_for('game-board')
    time.sleep(1.2)
    tags, shown = visible()
    # The heading also names the level ("Solo · 6×6 · Easy"); the level is the remembered choice.
    expect_prefix(f'{label} solo heading', shown, fmt(s['solo_mode_label'], 6))
    expect(f'{label} solo game', shown, s['action_pause'], s['action_erase'], s['action_back'],
           s['cd_notes'], s['cd_undo'], s['cd_redo'], s['cd_hint'], s['status_in_progress'])
    expect_prefix(f'{label} elapsed time', shown, s['elapsed_description'].split('%')[0])
    expect_prefix(f'{label} cell label', shown, fmt(s['cell_label'].split(', %3$s')[0], 1, 1) + ', ')
    shot(f'{label}-solo-game')
    tap('pause'); wait_for('resume'); _, shown = visible()
    expect(f'{label} paused', shown, s['paused_title'], s['action_resume'], s['action_save_exit'])
    tap('save-exit'); wait_for('friend-start')
    tap('friend-start'); wait_for('room-create'); tags, shown = visible()
    expect(f'{label} live setup', shown, s['home_friend_start'], s['live_setup_title'], s['live_name_label'], s['mode_race'], s['mode_duel_long'], s['live_create'], s['live_join'],
           s['guest_name'], s['difficulty'])
    shot(f'{label}-live-setup')
    # Rooms default to 9×9 race and remember the last mode and size; this check reads a 6×6 race lobby.
    tap('mode-race'); tap('board-size-6')
    tap('room-create'); wait_for('ready-toggle', 60); time.sleep(1)
    tags, shown = visible()
    expect(f'{label} lobby', shown, s['lobby_waiting'], s['conn_connected'], s['invite_copy'], s['invite_share'],
           s['lobby_friend_missing'], s['ready_action'], s['guest_name'], s['player_you'], s['player_not_ready'])
    expect_prefix(f'{label} lobby mode', shown, fmt(s['lobby_mode_size'], s['mode_race'], 6))
    shot(f'{label}-lobby')
    tap('back'); wait_for('leave-confirm'); _, shown = visible()
    expect(f'{label} leave dialog', shown, s['leave_title'], s['leave_body'], s['action_leave'], s['action_stay'])
    shot(f'{label}-leave-dialog')
    tap('leave-confirm'); wait_for('solo-start', 60)
    print(f'ok   {label} ({"system" if tag is None else tag})', flush=True)


assert shell('getprop', 'ro.kernel.qemu') == '1', 'This test only runs on an emulator.'
with socket.socket() as probe:
    assert probe.connect_ex(('127.0.0.1', args.server_port)) != 0, 'server port in use'
server = subprocess.Popen([args.server_bin], env=dict(os.environ, SUDOKU_PORT=str(args.server_port)), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
shell('am', 'start', '-W', '-n', PKG + '/.MainActivity')
original = {name: read(name) for name in private_files()}
airplane = shell('settings', 'get', 'global', 'airplane_mode_on')
locale_before = android_locale.get(shell, PKG)
try:
    shell('cmd', 'connectivity', 'airplane-mode', 'disable')
    for _ in range(40):
        if socket.socket().connect_ex(('127.0.0.1', args.server_port)) == 0: break
        time.sleep(.25)
    if args.system:
        run_language(None, args.system, f'system-{args.system}')
    else:
        run_language('tr-TR', 'tr', 'tr')
        run_language('en-US', 'en', 'en')
        run_language('de-DE', 'en', 'de-falls-back-to-en')  # unsupported language -> default (English) resources
finally:
    shell('am', 'force-stop', PKG)
    for name in private_files(): shell('run-as', PKG, 'rm', '-f', 'files/' + name)
    for name, data in original.items(): write(name, data)
    android_locale.set(shell, PKG, locale_before)
    shell('cmd', 'connectivity', 'airplane-mode', 'enable' if airplane == '1' else 'disable')
    shell('rm', '-f', '/sdcard/sudoku-locale-check.xml')
    server.terminate()
    try: server.wait(timeout=10)
    except subprocess.TimeoutExpired: server.kill()
if failures:
    print('FAILED:'); [print(' ', f) for f in failures]; sys.exit(1)
print(f'Android locale passed: {checked} text/accessibility expectations')
