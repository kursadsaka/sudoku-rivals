#!/usr/bin/env python3
"""Android HTTPS staging build checks.

Static part (no device): the staging APK is a non-debuggable, cleartext-free build
with its own application ID, no loopback/App Links intent filters and a pinned
HTTPS origin, while debug keeps loopback and release keeps the empty default.

Emulator part (--adb): installs only the staging package (never debug/release,
never any Render setting). With --live it sends one create and one leave to the
staging origin to prove TLS reachability, then checks the record survives a cold
restart and a same-package reinstall. Session data is never printed.
"""
import argparse
import pathlib
import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

ORIGIN = 'https://sudoku-rivals-g16-staging.onrender.com'
HOST = ORIGIN.removeprefix('https://')
ROOT = pathlib.Path(__file__).resolve().parent.parent
ANDROID = ROOT / 'apps/android/app'

parser = argparse.ArgumentParser()
parser.add_argument('--aapt2', help='Path to aapt2; enables APK manifest checks')
parser.add_argument('--adb', help='Path to adb; enables emulator checks')
parser.add_argument('--serial', default='emulator-5554')
parser.add_argument('--live', action='store_true', help='Send one create + leave to the staging origin')
args = parser.parse_args()

APKS = ANDROID / 'build/outputs/apk'
STAGING_APK = APKS / 'staging/app-staging.apk'


def fail(message):
    print('FAIL:', message)
    sys.exit(1)


def check(condition, message):
    if not condition: fail(message)
    print('ok  ', message)


def read(path): return pathlib.Path(path).read_text()


def build_config(kind):
    path = ANDROID / f'build/generated/source/buildConfig/{kind}/com/sudokurivals/BuildConfig.java'
    return read(path)


def field(source, name):
    match = re.search(rf'{name}\s*=\s*(.*?);', source)
    value = match.group(1) if match else None
    # AGP renders DEBUG as Boolean.parseBoolean("true") in debug variants.
    return re.sub(r'Boolean\.parseBoolean\("(true|false)"\)', r'\1', value) if value else value


def xmltree(apk):
    return subprocess.run([args.aapt2, 'dump', 'xmltree', '--file', 'AndroidManifest.xml', str(apk)],
                          check=True, stdout=subprocess.PIPE, text=True).stdout


def static_checks():
    staging, debug, release = build_config('staging'), build_config('debug'), build_config('release')
    check(field(staging, 'MATCH_SERVER_URL') == f'"{ORIGIN}"', 'staging origin is the fixed HTTPS Render origin')
    check(field(staging, 'STAGING') == 'true' and field(staging, 'DEBUG') == 'false', 'staging flag set, not DEBUG')
    check(field(debug, 'STAGING') == 'false' and field(debug, 'DEBUG') == 'true', 'debug keeps DEBUG loopback mode')
    check(field(release, 'STAGING') == 'false' and field(release, 'DEBUG') == 'false', 'release is neither staging nor debug')
    check(field(release, 'MATCH_SERVER_URL') in ('""',) or 'onrender' not in (field(release, 'MATCH_SERVER_URL') or ''),
          'release does not inherit the staging origin')
    gradle = read(ANDROID / 'build.gradle.kts')
    check('http://' not in re.sub(r'//.*', '', gradle), 'build script names no cleartext origin')
    check(not (ANDROID / 'src/staging').exists(), 'no staging source set overriding manifest or network config')
    main = read(ANDROID / 'src/main/AndroidManifest.xml')
    check('usesCleartextTraffic="false"' in main and 'networkSecurityConfig' not in main and 'autoVerify' not in main,
          'main manifest forbids cleartext, has no network config override or App Links')
    if not args.aapt2: return
    tree = xmltree(STAGING_APK)
    check('com.sudokurivals.staging' in subprocess.run([args.aapt2, 'dump', 'packagename', str(STAGING_APK)],
                                                       check=True, stdout=subprocess.PIPE, text=True).stdout,
          'staging package ID is separate from debug/release')
    check('networkSecurityConfig' not in tree, 'staging APK has no network security config')
    check('usesCleartextTraffic' in tree and re.search(r'usesCleartextTraffic.*=false', tree) is not None,
          'staging APK keeps cleartext disabled')
    check('debuggable' not in tree, 'staging APK is not debuggable')
    check('android:scheme' not in tree and 'autoVerify' not in tree and 'android.intent.action.VIEW' not in tree,
          'staging APK has no VIEW/scheme/autoVerify filters')
    debug_tree = xmltree(APKS / 'debug/app-debug.apk')
    check('networkSecurityConfig' in debug_tree and '10.0.2.2' in debug_tree and 'localhost' in debug_tree,
          'debug APK keeps loopback links and network config')
    for apk in (STAGING_APK, APKS / 'debug/app-debug.apk', APKS / 'release/app-release-unsigned.apk'):
        badging = subprocess.run([args.aapt2, 'dump', 'badging', str(apk)], check=True, stdout=subprocess.PIPE, text=True).stdout
        check(re.search(r"application-icon-\d+:'res/\w[^']*\.xml'", badging) is not None and "icon='res/" in badging,
              f'{apk.name} has an adaptive launcher icon')
    release_tree = xmltree(APKS / 'release/app-release-unsigned.apk')
    check('staging' not in release_tree.lower() and 'networkSecurityConfig' not in release_tree,
          'release APK manifest unchanged by staging')


# ---------------------------------------------------------------- emulator

PKG = 'com.sudokurivals.staging'


def adb(*parts, check_=True):
    return subprocess.run([args.adb, '-s', args.serial, *parts], stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, timeout=60, check=check_).stdout


def shell(*parts): return adb('shell', *parts).decode().strip()


def hierarchy():
    shell('uiautomator', 'dump', '/sdcard/staging-check.xml')
    return ET.fromstring(adb('exec-out', 'cat', '/sdcard/staging-check.xml'))


def find(tag, scroll=False, tries=8):
    for attempt in range(tries):
        for item in hierarchy().iter('node'):
            resource = item.attrib.get('resource-id', '')
            if resource == tag or resource.endswith(':id/' + tag): return item
        if scroll and attempt % 2:
            width, height = map(int, re.findall(r'(\d+)x(\d+)', shell('wm', 'size'))[-1])
            shell('input', 'swipe', str(width // 2), str(height * 85 // 100), str(width // 2), str(height * 30 // 100), '160')
        else:
            time.sleep(.4)
    return None


def tap(tag, scroll=False, tries=8):
    item = find(tag, scroll, tries)
    check(item is not None, f'UI node present: {tag}')
    x1, y1, x2, y2 = map(int, re.findall(r'\d+', item.attrib['bounds']))
    shell('input', 'tap', str((x1 + x2) // 2), str((y1 + y2) // 2))
    time.sleep(.3)


def until(test, timeout, message):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        value = test()
        if value: return value
        time.sleep(1)
    fail(message)


def launch(*extra):
    shell('am', 'start', '-W', '-n', PKG + '/com.sudokurivals.MainActivity', *extra)


def foreign_origin_record():
    """A saved record naming any non-staging origin must be rejected and left untouched."""
    adb('root', check_=False)
    adb('wait-for-device')
    time.sleep(2)
    if shell('id', '-u') != '0':
        print('skip  foreign-origin record check (adb root unavailable)')
        return
    path = f'/data/data/{PKG}/files/live-session.json'
    uid = shell('stat', '-c', '%u', f'/data/data/{PKG}/files')
    for foreign in ('http://10.0.2.2:3000', 'https://example.invalid', 'https://' + HOST + '.evil.example'):
        shell('am', 'force-stop', PKG)
        record = '{"version":1,"origin":"%s","client":"opaque-test-value"}' % foreign
        shell(f"printf '%s' '{record}' > {path} && chown {uid}:{uid} {path} && chmod 600 {path} && restorecon {path}")
        before = shell('sha256sum', path)
        launch()
        tap('friend-start', True)
        check(find('live-recover', True) is not None, f'foreign origin record rejected and protected ({foreign.split("//")[1][:12]}…)')
        check(shell('sha256sum', path) == before, 'rejected record left byte-for-byte untouched')
    shell('am', 'force-stop', PKG)
    shell('rm', '-f', path, path + '.bak')
    launch()
    tap('friend-start', True)


def emulator_checks():
    boot = shell('getprop', 'sys.boot_completed')
    check(boot == '1', 'emulator booted')
    adb('install', '-r', str(STAGING_APK))
    check(PKG in shell('pm', 'list', 'packages', PKG), 'staging package installed')
    flags = shell('dumpsys', 'package', PKG)
    check('DEBUGGABLE' not in re.search(r'pkgFlags=\[(.*?)\]', flags).group(1), 'installed staging app not debuggable')
    shell('pm', 'clear', PKG)
    # A debug-only intent extra must not reconfigure the staging origin.
    launch('--es', 'dev_server_url', 'http://10.0.2.2:3000')
    tap('friend-start', True)
    fixed = find('live-server-fixed', True)
    check(fixed is not None and HOST in fixed.attrib.get('text', ''), 'setup shows the fixed staging host')
    check(find('live-server', False, 1) is None and find('live-server-apply', False, 1) is None,
          'no editable server field in staging')
    foreign_origin_record()
    if not args.live: return
    # One lightweight setup request: the shared staging quota allows 5 creates / 10 minutes.
    tap('room-create', True)
    def lobby_code():
        item = find('room-code', False, 1)
        return item.attrib.get('text') if item is not None else None
    code = until(lobby_code, 90, 'Create over HTTPS did not reach a lobby (cold start can take ~1 min)')
    check(re.search(r'[A-Z2-9]{6}', code) is not None, 'lobby shows a room code from the HTTPS staging server')
    shell('am', 'force-stop', PKG)
    launch()
    check(until(lobby_code, 60, 'Cold restart did not resume the saved room') == code,
          'record survives cold restart (same room resumed)')
    adb('install', '-r', str(STAGING_APK))
    launch()
    check(until(lobby_code, 60, 'Same-package reinstall lost the saved room') == code,
          'record survives same-package reinstall')
    tap('back')
    tap('leave-confirm')
    until(lambda: find('solo-start', False, 1) is not None and find('room-code', False, 1) is None, 60,
          'Leave was not acknowledged by staging (home screen not reached)')
    print('ok   room left; no record kept open on staging')


static_checks()
if args.adb: emulator_checks()
print('PASS')
