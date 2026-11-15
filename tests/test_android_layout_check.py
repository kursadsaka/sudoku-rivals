import unittest
import xml.etree.ElementTree as ET
from android_layout_check import check, nodes_by_tag


def node(tag, x1, y1, x2, y2, text=''):
    return f'<node resource-id="com.sudokurivals:id/{tag}" text="{text}" bounds="[{x1},{y1}][{x2},{y2}]"/>'


def screen(size=9, mode='race', width=1080, hide_digits=False, extra=''):
    """A 411dp-wide portrait phone at density 420 (2.625 px/dp): nav, board, digits, tools."""
    body = [node('back', 20, 100, 146, 226), node('mode-label', 300, 100, 800, 160), node('timer', 450, 160, 650, 220),
            node('game-board', 40, 300, 1040, 1300)]
    if size == 9:
        first, second, y1, y2 = 5, 4, 1340, 1480
        xs = lambda count: [40 + (1000 - count * 190 - (count - 1) * 16) // 2 + i * 206 for i in range(count)]
        for i, x in enumerate(xs(5)): body.append(node(f'number-{i + 1}', x, y1, x + 190, y1 + 126))
        for i, x in enumerate(xs(4)): body.append(node(f'number-{i + 6}', x, y2, x + 190, y2 + 126))
    else:
        for i in range(6): body.append(node(f'number-{i + 1}', 40 + i * 168, 1340, 40 + i * 168 + 156, 1466))
    tools = {'race': ['notes-toggle', 'erase', 'undo', 'redo'], 'solo': ['notes-toggle', 'erase', 'undo', 'redo', 'hint'], 'duel': []}[mode]
    for i, tag in enumerate(tools): body.append(node(tag, 40 + i * 200, 1640, 40 + i * 200 + 190, 1766))
    if mode == 'solo': body.append(node('pause', 880, 100, 1040, 226))
    return nodes_by_tag(ET.fromstring('<hierarchy>' + ''.join(body) + extra + '</hierarchy>'))


class LayoutCheckTests(unittest.TestCase):
    def test_valid_layouts(self):
        for mode in ('solo', 'race', 'duel'):
            for size in (6, 9):
                with self.subTest(mode=mode, size=size):
                    problems, metrics = check(screen(size, mode), 1080, 2400, 420, mode, size)
                    self.assertEqual(problems, [])
                    self.assertGreaterEqual(metrics['board_dp'], 280)

    def test_debug_text_is_flagged_unless_debug_ui(self):
        tags = screen(9, 'solo', extra=node('app-title', 0, 0, 10, 10) + node('selected-cell', 0, 0, 10, 10))
        self.assertTrue(any('app-title' in p for p in check(tags, 1080, 2400, 420, 'solo', 9)[0]))
        self.assertEqual(check(tags, 1080, 2400, 420, 'solo', 9, debug_ui=True)[0], [])
        self.assertTrue(any('debug text' in p for p in check(screen(9, 'solo'), 1080, 2400, 420, 'solo', 9, texts=['Oyun devam ediyor'])[0]))

    def test_clipped_or_small_controls_and_wrong_digit_layout(self):
        tags = screen(9, 'race')
        self.assertTrue(any('outside the screen' in p for p in check(tags, 1080, 1700, 420, 'race', 9)[0]))
        tags = screen(6, 'race')
        tags['number-3'] = (tags['number-3'][0], tags['number-3'][1], tags['number-3'][2], tags['number-3'][1] + 100, {})
        self.assertTrue(any('under 48dp' in p for p in check(tags, 1080, 2400, 420, 'race', 6)[0]))
        tags = screen(9, 'race')
        x1, y1, x2, y2, attrib = tags['number-9']; tags['number-9'] = (x1 + 150, y1, x2 + 150, y2, attrib)
        self.assertTrue(any('centred' in p for p in check(tags, 1080, 2400, 420, 'race', 9)[0]))

    def test_duel_has_no_tools_and_missing_tags_reported(self):
        tags = screen(6, 'race')
        self.assertTrue(any('duel must not show' in p for p in check(tags, 1080, 2400, 420, 'duel', 6)[0]))
        tags = screen(6, 'duel'); del tags['timer']
        self.assertEqual(check(tags, 1080, 2400, 420, 'duel', 6)[0], ['missing timer'])


if __name__ == '__main__':
    unittest.main()
