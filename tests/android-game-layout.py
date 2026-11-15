#!/usr/bin/env python3
"""Solo game-screen layout on a disposable emulator: one screen, no scrolling, hidden debug text.

For 6x6 and 9x9 it runs each display size below, checks order/targets/digit layout and that a
vertical swipe does not move the board, and compares debug_ui off/on. Display size/density, the
app's solo records and airplane mode are restored in `finally`. Live race/duel use the same
checker via `tests/android-multiplayer.py --layout-check`.
"""
import argparse
import json
import os
import pathlib
import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import android_locale
from android_layout_check import check, nodes_by_tag

parser = argparse.ArgumentParser()
parser.add_argument('--adb', default=os.path.join(os.environ.get('ANDROID_HOME', ''), 'platform-tools/adb'))
parser.add_argument('--serial', default='emulator-5554')
parser.add_argument('--screenshots', type=pathlib.Path)
parser.add_argument('--locale', help='Per-app language to run in, e.g. en-US or tr-TR (default: leave as is)')
parser.add_argument('--json', type=pathlib.Path, help='Write the measured metrics here')
args = parser.parse_args()
PKG = 'com.sudokurivals'
# name, width px, height px, density, must fit without scrolling (very small screens may scroll)
VIEWPORTS = [
    ('411x914dp', 1080, 2400, 420, True), ('411x731dp', 1080, 1920, 420, True), ('360x640dp', 720, 1280, 320, True),
    ('320x568dp', 640, 1136, 320, False), ('731x411dp-landscape', 1920, 1080, 420, True),
]


def adb(*parts, data=None, check_=True):
    return subprocess.run([args.adb, '-s', args.serial, *parts], input=data, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, check=check_, timeout=40).stdout


def shell(*parts): return adb('shell', *parts).decode().strip()
def read(name): return adb('exec-out', 'run-as', PKG, 'cat', 'files/' + name)
def write(name, data): adb('shell', '-T', 'run-as', PKG, 'tee', 'files/' + name, data=data)
def private_files():
    return [n for n in adb('exec-out', 'run-as', PKG, 'ls', '-1p', 'files', check_=False).decode().splitlines() if re.fullmatch(r'solo-[\w.-]+', n)]


def launch(debug_ui):
    shell('am', 'start', '-W', '-n', PKG + '/.MainActivity', '--ez', 'debug_ui', 'true' if debug_ui else 'false')


def hierarchy():
    shell('uiautomator', 'dump', '/sdcard/sudoku-layout-check.xml')
    return ET.fromstring(adb('exec-out', 'cat', '/sdcard/sudoku-layout-check.xml'))


def tags(retries=8):
    for _ in range(retries):
        found = nodes_by_tag(hierarchy())
        if found: return found
        time.sleep(.4)
    return {}


def tap(tag):
    for _ in range(10):
        found = tags()
        if tag in found:
            x1, y1, x2, y2, _ = found[tag]
            shell('input', 'tap', str((x1 + x2) // 2), str((y1 + y2) // 2))
            time.sleep(.4)
            return
        time.sleep(.4)
    raise AssertionError('UI node missing: ' + tag)


def wait_for(tag):
    for _ in range(40):
        if tag in tags(1): return
        time.sleep(.5)
    raise AssertionError('UI node never appeared: ' + tag)


def display():
    width, height = map(int, re.findall(r'(\d+)x(\d+)', shell('wm', 'size'))[-1])
    return width, height, int(re.findall(r'(?:Physical|Override) density: (\d+)', shell('wm', 'density'))[-1])


def start(size):
    shell('am', 'force-stop', PKG)
    for name in private_files(): shell('run-as', PKG, 'rm', '-f', 'files/' + name)
    launch(False)
    tap('solo-start'); tap(f'board-size-{size}'); tap('game-start')
    wait_for('game-board')


def measure(label, size, debug_ui, must_fit):
    time.sleep(1.2)
    width, height, density = display()
    hierarchy_root = hierarchy()
    found = nodes_by_tag(hierarchy_root)
    texts = [n.attrib.get('text', '') for n in hierarchy_root.iter('node')]
    problems, metrics = check(found, width, height, density, 'solo', size, debug_ui, texts)
    scrolls = None
    if 'game-board' in found and not problems:
        before = found['game-board'][:4]
        shell('input', 'swipe', str(width // 2), str(height * 8 // 10), str(width // 2), str(height * 3 // 10), '200')
        time.sleep(.6)
        after = tags().get('game-board', (0, 0, 0, 0))[:4]
        scrolls = before != tuple(after)
        if scrolls:  # leave the page where it was for the next display size
            shell('input', 'swipe', str(width // 2), str(height * 3 // 10), str(width // 2), str(height * 8 // 10), '200')
        if must_fit and scrolls: problems.append('page scrolls although it must fit')
    if args.screenshots:
        args.screenshots.mkdir(parents=True, exist_ok=True)
        suffix = '-debug' if debug_ui else ''
        (args.screenshots / f'solo-{size}-{label}{suffix}.png').write_bytes(adb('exec-out', 'screencap', '-p'))
    record = {'viewport': label, 'size': size, 'debug_ui': debug_ui, 'scrolls': scrolls, **metrics}
    results.append(record)
    print(('ok   ' if not problems else 'FAIL ') + json.dumps(record), flush=True)
    for problem in problems: print('     -', problem)
    return problems


assert shell('getprop', 'ro.kernel.qemu') == '1', 'This test only runs on an emulator.'
shell('am', 'start', '-W', '-n', PKG + '/.MainActivity')
original = {name: read(name) for name in private_files()}
airplane = shell('settings', 'get', 'global', 'airplane_mode_on')
orig_size = re.findall(r'(\d+x\d+)', shell('wm', 'size'))[0]
orig_density = re.findall(r'(\d+)', shell('wm', 'density'))[0]
results, failures = [], []
locale_before = android_locale.get(shell, PKG)
if args.locale: android_locale.set(shell, PKG, args.locale)
try:
    shell('cmd', 'connectivity', 'airplane-mode', 'enable')
    for size in (6, 9):
        # Start at the first display size, then resize the running game (config changes keep it).
        shell('wm', 'size', f'{VIEWPORTS[0][1]}x{VIEWPORTS[0][2]}'); shell('wm', 'density', str(VIEWPORTS[0][3]))
        start(size)
        for label, w, h, d, must_fit in VIEWPORTS:
            shell('wm', 'size', f'{w}x{h}'); shell('wm', 'density', str(d))
            failures += [f'{size}x{size} {label}: {p}' for p in measure(label, size, False, must_fit)]
    shell('wm', 'size', f'{VIEWPORTS[0][1]}x{VIEWPORTS[0][2]}'); shell('wm', 'density', str(VIEWPORTS[0][3]))
    start(9)
    shell('am', 'force-stop', PKG); launch(True)
    wait_for('board-size-9'); tap('board-size-9'); wait_for('solo-resume'); tap('solo-resume'); wait_for('resume'); tap('resume'); wait_for('game-board')
    failures += [f'debug_ui on: {p}' for p in measure('411x914dp', 9, True, False)]
    # A default launch must not inherit the flag.
    shell('am', 'force-stop', PKG); launch(False)
    wait_for('board-size-9'); tap('board-size-9'); wait_for('solo-resume'); tap('solo-resume'); wait_for('resume'); tap('resume'); wait_for('game-board')
    failures += [f'debug_ui reset: {p}' for p in measure('411x914dp-relaunch', 9, False, True)]
finally:
    shell('am', 'force-stop', PKG)
    shell('wm', 'size', orig_size); shell('wm', 'density', orig_density)
    android_locale.set(shell, PKG, locale_before)
    for name in private_files(): shell('run-as', PKG, 'rm', '-f', 'files/' + name)
    for name, data in original.items(): write(name, data)
    shell('cmd', 'connectivity', 'airplane-mode', 'enable' if airplane == '1' else 'disable')
    shell('rm', '-f', '/sdcard/sudoku-layout-check.xml')
if args.json: args.json.write_text(json.dumps(results, indent=1))
if failures:
    print('FAILED:'); [print(' ', f) for f in failures]; sys.exit(1)
print('Android game layout passed:', len(results), 'solo measurements')
