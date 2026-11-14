#!/usr/bin/env python3
"""Check every packaged ELF64 PT_LOAD, including transitive JNA libraries.

Usage: python3 tests/android-native-alignment.py app.apk [app.aab ...]
APK ZIP alignment must also pass: zipalign -c -P 16 -v 4 app.apk
"""
import struct
import sys
import zipfile


def check_elf(data):
    assert data[:6] == b'\x7fELF\x02\x01', 'expected little-endian ELF64'
    offset = struct.unpack_from('<Q', data, 32)[0]
    size, count = struct.unpack_from('<HH', data, 54)
    loads = 0
    for index in range(count):
        kind, _, file_offset, address, _, _, _, alignment = struct.unpack_from(
            '<IIQQQQQQ', data, offset + index * size)
        if kind == 1:
            loads += 1
            assert alignment >= 16384 and alignment & (alignment - 1) == 0, alignment
            assert file_offset % 16384 == address % 16384, 'incongruent PT_LOAD'
    assert loads, 'missing PT_LOAD'


if __name__ == '__main__':
    assert len(sys.argv) > 1, 'provide APK/AAB path(s)'
    expected = {'arm64-v8a', 'x86_64'}
    for path in sys.argv[1:]:
        with zipfile.ZipFile(path) as archive:
            libraries = [name for name in archive.namelist() if name.endswith('.so')]
            assert libraries, 'no native libraries'
            assert {name.split('/')[-2] for name in libraries} == expected
            for abi in expected:
                assert any(name.endswith(f'{abi}/libsudoku_ffi.so') for name in libraries)
            for name in libraries:
                data = archive.read(name)
                check_elf(data)
                machine = {'arm64-v8a': 183, 'x86_64': 62}[name.split('/')[-2]]
                assert struct.unpack_from('<H', data, 18)[0] == machine, 'wrong ABI'
                print(f'PASS {path}: {name} ELF LOAD alignment >= 16 KB')
