import pathlib
import runpy
import struct
import unittest

check_elf = runpy.run_path(str(pathlib.Path(__file__).with_name('android-native-alignment.py')))['check_elf']


def elf(alignment, offset=0, address=0, kind=1):
    data = bytearray(120)
    data[:6] = b'\x7fELF\x02\x01'
    struct.pack_into('<Q', data, 32, 64)
    struct.pack_into('<HH', data, 54, 56, 1)
    struct.pack_into('<IIQQQQQQ', data, 64, kind, 5, offset, address, 0, 1, 1, alignment)
    return data


class NativeAlignmentTests(unittest.TestCase):
    def test_aligned(self):
        for alignment in (16384, 65536):
            check_elf(elf(alignment))

    def test_rejects_unsafe_segments(self):
        for data in (elf(4096), elf(24576), elf(16384, 1, 0), elf(16384, kind=0), bytes(120)):
            with self.subTest(data=data), self.assertRaises(AssertionError):
                check_elf(data)


if __name__ == '__main__':
    unittest.main()
