#!/usr/bin/env python3
"""Static checks for the iOS `Staging` configuration (G16). No network, no signing, no simulator.

Builds `SudokuRivalsStaging` for the simulator into a private derived-data directory
(unsigned) and inspects the .app: placeholder staging bundle ID, no ATS exception,
privacy manifest present and valid, fixed HTTPS staging origin compiled in, and no
loopback/cleartext origin string in the binary.

Usage: python3 tests/ios-staging.py [--derived-data DIR] [--destination 'generic/platform=iOS Simulator']
Needs `apps/ios/SudokuRivals.xcodeproj` (scripts/ios-build.sh) first.
"""
import argparse
import pathlib
import plistlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
ORIGIN = 'https://sudoku-rivals-g16-staging.onrender.com'
parser = argparse.ArgumentParser()
parser.add_argument('--derived-data')
parser.add_argument('--destination', default='generic/platform=iOS Simulator')
args = parser.parse_args()
dd = pathlib.Path(args.derived_data or tempfile.mkdtemp(prefix='ios-staging-dd-'))
subprocess.run(['xcodebuild', '-project', str(ROOT / 'apps/ios/SudokuRivals.xcodeproj'), '-scheme', 'SudokuRivalsStaging',
                '-destination', args.destination, '-derivedDataPath', str(dd), 'CODE_SIGNING_ALLOWED=NO', 'build'],
               check=True, stdout=subprocess.DEVNULL)
apps = sorted(dd.glob('Build/Products/Staging-*/SudokuRivals.app'))
assert len(apps) == 1, f'expected one Staging app, found {apps}'
app = apps[0]
failures = []


def check(name, ok):
    print(('PASS ' if ok else 'FAIL ') + name)
    if not ok:
        failures.append(name)


info = plistlib.loads((app / 'Info.plist').read_bytes())
check('bundle id is the staging placeholder', info['CFBundleIdentifier'] == 'com.example.sudokurivals.staging')
check('display name marks staging', info['CFBundleDisplayName'] == 'Sudoku Rivals Staging')
check('no NSAppTransportSecurity exceptions at all', 'NSAppTransportSecurity' not in info)
check('no URL schemes or universal-link entitlements claimed', 'CFBundleURLTypes' not in info)
privacy = plistlib.loads((app / 'PrivacyInfo.xcprivacy').read_bytes())
check('privacy manifest: no tracking, no tracking domains', privacy['NSPrivacyTracking'] is False and privacy['NSPrivacyTrackingDomains'] == [])
binary = b''.join(p.read_bytes() for p in [app / 'SudokuRivals', *app.glob('*.dylib')])
check('fixed https staging origin is compiled in', ORIGIN.encode() in binary)
check('no cleartext loopback origin string in staging binary', b'http://127.0.0.1' not in binary and b'LIVE_TEST_ORIGIN' not in binary)
check('no test hooks (DEBUG-only env keys) in staging binary', b'LIVE_TEST_RESET' not in binary and b'SOLO_TEST_RESET' not in binary)
sys.exit(1 if failures else 0)
