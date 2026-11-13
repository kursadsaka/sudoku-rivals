#!/usr/bin/env python3
"""ADB integration check for the debug APK on a disposable Android emulator.

Run after assembleDebug + adb install -r. Existing solo files and airplane mode
are restored even if an assertion fails. No third-party Python packages needed.
"""
import argparse
import json
import os
import pathlib
import re
import subprocess
import time
import xml.etree.ElementTree as ET

parser = argparse.ArgumentParser()
parser.add_argument('--adb', default=os.path.join(os.environ.get('ANDROID_HOME', ''), 'platform-tools/adb'))
parser.add_argument('--serial', default='emulator-5554')
parser.add_argument('--screenshots', type=pathlib.Path)
args = parser.parse_args()
PKG = 'com.sudokurivals'


def adb(*arguments, data=None, check=True):
    result = subprocess.run([args.adb, '-s', args.serial, *arguments], input=data,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=check, timeout=30)
    return result.stdout


def shell(*arguments):
    return adb('shell', *arguments).decode().strip()


def files():
    listing = adb('exec-out', 'run-as', PKG, 'ls', '-1p', 'files', check=False).decode().splitlines()
    return [name for name in listing if re.fullmatch(r'solo-[\w.-]+', name)]


def read(name):
    return adb('exec-out', 'run-as', PKG, 'cat', 'files/' + name)


def write(name, data):
    adb('shell', '-T', 'run-as', PKG, 'tee', 'files/' + name, data=data)


def record(size):
    return json.loads(read(f'solo-{size}.json'))


def game(size):
    return json.loads(record(size)['game'])


def launch():
    shell('am', 'start', '-W', '-n', PKG + '/.MainActivity')


def stop():
    shell('am', 'force-stop', PKG)


def hierarchy():
    shell('uiautomator', 'dump', '/sdcard/sudoku-solo-check.xml')
    return ET.fromstring(adb('exec-out', 'cat', '/sdcard/sudoku-solo-check.xml'))


def find(tag, scroll=False):
    for attempt in range(9 if scroll else 1):
        nodes = list(hierarchy().iter('node'))
        for node in nodes:
            resource = node.attrib.get('resource-id', '')
            if resource == tag or resource.endswith(':id/' + tag):
                return node
        if scroll:
            shell('input', 'swipe', '540', '1580', '540', '650', '180')
            time.sleep(.15)
    raise AssertionError('UI node missing: ' + tag)


def tap(tag, scroll=False):
    node = find(tag, scroll)
    assert node.attrib.get('enabled') == 'true', tag + ' disabled'
    x1, y1, x2, y2 = map(int, re.findall(r'\d+', node.attrib['bounds']))
    shell('input', 'tap', str((x1 + x2) // 2), str((y1 + y2) // 2))
    time.sleep(.2)


def start_new(size):
    tap('solo-start')
    tap(f'board-size-{size}')
    tap('game-start')
    try:
        # Replacement is an explicit action in this test, never a silent reset.
        nodes = list(hierarchy().iter('node'))
        replacement = next((n for n in nodes if n.attrib.get('text') == 'Yeni oyun'), None)
        if replacement is not None:
            x1, y1, x2, y2 = map(int, re.findall(r'\d+', replacement.attrib['bounds']))
            shell('input', 'tap', str((x1+x2)//2), str((y1+y2)//2))
            time.sleep(.3)
    except ET.ParseError:
        pass
    find('game-board')
    visible = list(hierarchy().iter('node'))
    density = int(re.findall(r'(?:Physical|Override) density: (\d+)', shell('wm', 'density'))[-1])
    for digit in range(1, size + 1):
        node = next((node for node in visible if node.attrib.get('resource-id') == f'number-{digit}' or node.attrib.get('resource-id', '').endswith(f':id/number-{digit}')), None)
        assert node is not None, 'Number pad requires scrolling'
        x1, y1, x2, y2 = map(int, re.findall(r'\d+', node.attrib['bounds']))
        assert min(x2 - x1, y2 - y1) >= 48 * density / 160 - 2, 'Number target is smaller than 48 dp'
    if args.screenshots:
        args.screenshots.mkdir(parents=True, exist_ok=True)
        (args.screenshots / f'android-solo-{size}.png').write_bytes(adb('exec-out', 'screencap', '-p'))


assert shell('getprop', 'ro.kernel.qemu') == '1', 'This test only runs on an emulator.'
# Debug APK installed before the test; initial launch ensures files/ exists.
launch()
original = {name: read(name) for name in files()}
airplane = shell('settings', 'get', 'global', 'airplane_mode_on')
try:
    shell('cmd', 'connectivity', 'airplane-mode', 'enable')
    assert shell('settings', 'get', 'global', 'airplane_mode_on') == '1'
    stop()
    launch()
    start_new(6)
    print('Offline 6x6 new game loaded', flush=True)
    # Rotation and night mode must not rebuild the activity and drop the game.
    rotation = (shell('settings', 'get', 'system', 'accelerometer_rotation'),
                shell('settings', 'get', 'system', 'user_rotation'))
    try:
        shell('settings', 'put', 'system', 'accelerometer_rotation', '0')
        shell('settings', 'put', 'system', 'user_rotation', '1')
        shell('cmd', 'uimode', 'night', 'yes')
        time.sleep(1)
        find('game-board')
    finally:
        shell('cmd', 'uimode', 'night', 'no')
        shell('settings', 'put', 'system', 'user_rotation', rotation[1] if rotation[1] != 'null' else '0')
        shell('settings', 'put', 'system', 'accelerometer_rotation', rotation[0] if rotation[0] != 'null' else '1')
        time.sleep(1)
    find('game-board')
    print('Rotation and night mode kept the open game', flush=True)
    initial = game(6)
    given = next(i for i, v in enumerate(initial['givens']) if v)
    blank = next(i for i, v in enumerate(initial['givens']) if not v)
    tap(f'cell-{given//6+1}-{given%6+1}')
    shell('input', 'keyevent', 'KEYCODE_1')
    assert game(6) == initial, 'Given cell changed'
    tap(f'cell-{blank//6+1}-{blank%6+1}')
    shell('input', 'keyevent', 'KEYCODE_N')
    shell('input', 'keyevent', 'KEYCODE_1')
    assert game(6)['cells'][blank]['notes'] == 1
    tap('undo', scroll=True)
    assert game(6)['cells'][blank]['notes'] == 0
    tap('redo', scroll=True)
    assert game(6)['cells'][blank]['notes'] == 1
    shell('input', 'keyevent', 'KEYCODE_DEL')
    assert game(6)['cells'][blank]['notes'] == 0
    tap('undo', scroll=True)
    assert game(6)['cells'][blank]['notes'] == 1
    before_failure = read('solo-6.json')
    shell('run-as', PKG, 'mkdir', 'files/solo-6.json.new')
    write('solo-6.json.new/blocked', b'occupied')
    shell('input', 'keyevent', 'KEYCODE_N')
    shell('input', 'keyevent', 'KEYCODE_2')
    assert read('solo-6.json') == before_failure, 'Failed write changed previous disk record'
    assert 'kaydedilemedi' in find('storage-status', scroll=True).attrib.get('text', '')
    shell('run-as', PKG, 'rm', '-rf', 'files/solo-6.json.new')
    tap('retry', scroll=True)
    assert game(6)['cells'][blank]['notes'] == 0
    assert game(6)['cells'][blank]['value'] == 2, 'Retry lost live changes'
    persisted = game(6)
    print('Given protection, notes, undo/redo, erase and failed-save retry passed', flush=True)
    stop()
    launch()
    tap('solo-resume')
    assert not any(n.attrib.get('resource-id') == 'game-board' for n in hierarchy().iter('node')), 'Paused board is exposed'
    tap('resume')
    assert game(6) == persisted, 'Cold reopen lost board/history/notes'
    shell('input', 'keyevent', 'KEYCODE_ESCAPE')
    tap('save-exit')
    game6 = game(6)
    print('Cold reopen and paused board privacy passed', flush=True)
    start_new(9)
    assert len(game(9)['cells']) == 81
    assert game(6) == game6, '9x9 game overwrote 6x6 slot'
    print('Offline 9x9 and separate size slots passed', flush=True)
    stop()
    corrupt = b'{broken-save'
    write('solo-9.json', corrupt)
    launch()
    tap('board-size-9')
    tap('solo-resume')
    assert read('solo-9.json') == corrupt, 'Corrupt save was overwritten during load'
    assert 'Kayıt açılamadı' in find('storage-status').attrib.get('text', '')
    start_new(9)
    assert any(read(name) == corrupt for name in files() if '-recovery-' in name), 'Corrupt save not preserved'
    assert len(game(9)['cells']) == 81
    print('Corrupt record preservation passed', flush=True)
    # A known one-blank puzzle drives the explainable hint and result UI.
    stop()
    fixture_path = pathlib.Path(__file__).resolve().parent.parent / 'vectors/boards.json'
    fixture = next(f for f in json.loads(fixture_path.read_text()) if f['size'] == 6)
    solution = fixture['solution']
    almost = game6.copy()
    almost['givens'] = solution.copy()
    almost['givens'][0] = 0
    almost['cells'] = [{'value': v, 'notes': 0} for v in almost['givens']]
    almost['undo'] = []
    almost['redo'] = []
    write('solo-6.json', json.dumps({'version': 1, 'size': 6, 'elapsedMillis': 1000, 'game': json.dumps(almost)}).encode())
    launch()
    tap('solo-resume')
    tap('resume')
    tap('hint', scroll=True)
    nodes = list(hierarchy().iter('node'))
    apply_hint = next(n for n in nodes if n.attrib.get('text') == 'Uygula')
    x1, y1, x2, y2 = map(int, re.findall(r'\d+', apply_hint.attrib['bounds']))
    shell('input', 'tap', str((x1+x2)//2), str((y1+y2)//2))
    time.sleep(.2)
    assert 'tamamlandı' in find('result-heading').attrib.get('text', '')
    assert game(6)['cells'][0]['value'] == solution[0]
    print('Android solo passed: offline new games in both sizes, givens, notes, erase, undo/redo, cold restore, failed-save retry, pause privacy, separate slots, corrupt-save preservation, logical hint and completion')
finally:
    stop()
    shell('run-as', PKG, 'rm', '-rf', 'files/solo-6.json.new')
    for name in files():
        shell('run-as', PKG, 'rm', '-f', 'files/' + name)
    for name, data in original.items():
        write(name, data)
    shell('cmd', 'connectivity', 'airplane-mode', 'enable' if airplane == '1' else 'disable')
    shell('rm', '-f', '/sdcard/sudoku-solo-check.xml')
