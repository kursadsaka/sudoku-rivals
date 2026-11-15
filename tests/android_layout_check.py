"""Pure checks for the one-screen Android game layout (no adb, no third-party packages).

Input is a `uiautomator dump` tree; sizes are physical pixels plus the display density.
Used by tests/android-game-layout.py (solo) and tests/android-multiplayer.py --layout-check (live).
"""
import re

TOOLS = {'solo': ['notes-toggle', 'erase', 'undo', 'redo', 'hint'], 'race': ['notes-toggle', 'erase', 'undo', 'redo'], 'duel': []}
DEFAULT_HIDDEN = ['app-title', 'selected-cell']
MIN_BOARD_DP = 280
TARGET_DP = 48


def nodes_by_tag(root):
    """tag -> (x1, y1, x2, y2, attrib). Later duplicates (e.g. dialog copies) are ignored."""
    found = {}
    for item in root.iter('node'):
        resource = item.attrib.get('resource-id', '')
        tag = resource.split(':id/')[-1] if resource else ''
        if not tag or tag in found: continue
        bounds = re.findall(r'\d+', item.attrib.get('bounds', ''))
        if len(bounds) == 4: found[tag] = (*map(int, bounds), item.attrib)
    return found


def check(tags, width, height, density, mode, size, debug_ui=False, texts=()):
    """Return a list of problems (empty = ok) and a metrics dict. `mode` is solo, race or duel."""
    dp = 160 / density
    problems = []
    required = ['back', 'mode-label', 'timer', 'game-board'] + [f'number-{d}' for d in range(1, size + 1)] + TOOLS[mode]
    if mode == 'solo': required.append('pause')
    for tag in required:
        if tag not in tags: problems.append('missing ' + tag)
    if mode == 'duel':
        for tag in ('notes-toggle', 'erase', 'undo', 'redo', 'hint'):
            if tag in tags: problems.append('duel must not show ' + tag)
    if problems: return problems, {}
    for tag in required:
        x1, y1, x2, y2, _ = tags[tag]
        if x1 < 0 or y1 < 0 or x2 > width or y2 > height: problems.append(f'{tag} is outside the screen')
    for tag in ['back'] + [f'number-{d}' for d in range(1, size + 1)] + TOOLS[mode] + (['pause'] if mode == 'solo' else []):
        x1, y1, x2, y2, _ = tags[tag]
        if min(x2 - x1, y2 - y1) * dp < TARGET_DP - 1: problems.append(f'{tag} target under {TARGET_DP}dp')
    bx1, by1, bx2, by2, _ = tags['game-board']
    board_dp = (bx2 - bx1) * dp
    if abs((bx2 - bx1) - (by2 - by1)) > 2: problems.append('board is not square')
    n1 = [tags[f'number-{d}'] for d in range(1, size + 1)]
    rows = sorted({t[1] for t in n1})
    landscape = width > height
    if size == 6:
        want_rows = 2 if (width * dp - 24 < 6 * TARGET_DP + 5 * 6 or landscape) else 1
        if len(rows) != want_rows: problems.append(f'6x6 digit rows {len(rows)} != {want_rows}')
    else:
        if len(rows) != 2 or sum(1 for t in n1 if t[1] == rows[0]) != 5: problems.append('9x9 digits are not 5+4')
        else:
            first = [t for t in n1 if t[1] == rows[0]]
            second = [t for t in n1 if t[1] == rows[1]]
            if abs((first[0][0] + first[-1][2]) - (second[0][0] + second[-1][2])) > 4: problems.append('9x9 second digit row is not centred')
    # Order: navigation → board → digits → tools (portrait); board beside the panel in landscape.
    if not landscape:
        if not tags['back'][3] <= by1: problems.append('navigation is not above the board')
        if not by2 <= min(t[1] for t in n1): problems.append('digits are not below the board')
        for tag in TOOLS[mode]:
            if not tags[tag][1] >= max(t[3] for t in n1) - 1: problems.append(f'{tag} is not below the digits')
    else:
        if not bx2 <= min(t[0] for t in n1): problems.append('landscape digits are not beside the board')
    for tag in DEFAULT_HIDDEN:
        if debug_ui and tag not in tags: problems.append(f'{tag} missing with debug_ui')
        if not debug_ui and tag in tags: problems.append(f'{tag} shown without debug_ui')
    if not debug_ui:
        for text in texts:
            if text in ('Oyun devam ediyor', 'Oyun cihazına kaydedildi'): problems.append('debug text visible: ' + text)
    return problems, {'board_dp': round(board_dp), 'rows': len(rows)}
