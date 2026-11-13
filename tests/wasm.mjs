import fs from 'node:fs';
import assert from 'node:assert/strict';
import init, { validate_board, new_solo, update_solo, solo_is_complete } from '../apps/web/pkg/sudoku_wasm.js';
await init({module_or_path: fs.readFileSync(new URL('../apps/web/pkg/sudoku_wasm_bg.wasm', import.meta.url))});
for (const fixture of JSON.parse(fs.readFileSync(new URL('../vectors/boards.json', import.meta.url)))) {
  for (const [key, expected] of [['uniquePuzzle', true], ['solution', true], ['invalidDuplicate', false], ['multipleSolutions', true]]) {
    assert.equal(validate_board(Uint8Array.from(fixture[key]), fixture.size), expected, `${fixture.size} ${key}`);
  }
}
console.log('WASM fixture checks passed');

for (const size of [6, 9]) {
  let saved = new_solo(new Uint8Array(size * size), size);
  const command = (type, extra = {}) => JSON.stringify({type, ...extra});
  saved = update_solo(saved, command('toggle_note', {index: 0, value: size}));
  assert.equal(JSON.parse(saved).cells[0].notes, 1 << (size - 1));
  const notes = saved;
  saved = update_solo(saved, command('set', {index: 0, value: 1}));
  assert.equal(JSON.parse(saved).cells[0].notes, 0);
  const filled = saved;
  saved = update_solo(saved, command('undo'));
  assert.equal(JSON.parse(saved).cells[0].notes, JSON.parse(notes).cells[0].notes);
  saved = update_solo(saved, command('redo'));
  assert.equal(saved, filled);
  assert.equal(solo_is_complete(saved), false);
  assert.throws(() => update_solo(saved, command('set', {index: 0, value: size + 1})));
  assert.throws(() => update_solo('{}', command('undo')));
  assert.throws(() => update_solo(saved, '{'));
  const givens = new Uint8Array(size * size);
  givens[0] = 1;
  assert.throws(() => update_solo(new_solo(givens, size), command('erase', {index: 0})));
}
console.log('WASM solo checks passed');
