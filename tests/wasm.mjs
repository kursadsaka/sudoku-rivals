import fs from 'node:fs';
import assert from 'node:assert/strict';
import init, { validate_board, new_solo, update_solo, solo_is_complete, new_generated_solo, solo_conflicts, solo_hint } from '../apps/web/pkg/sudoku_wasm.js';
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

for (const fixture of JSON.parse(fs.readFileSync(new URL('../vectors/boards.json', import.meta.url)))) {
  const {seed, blanks, givens} = fixture.generated;
  const saved = JSON.parse(new_generated_solo(fixture.size, seed, blanks));
  assert.deepEqual(saved.givens, givens, `${fixture.size} shared seed vector`);
}
assert.throws(() => new_generated_solo(6, 0, 37));
console.log('WASM generation vectors passed');

for (const fixture of JSON.parse(fs.readFileSync(new URL('../vectors/boards.json', import.meta.url)))) {
  const cells = Uint8Array.from(fixture.solution);
  const value = cells[0]; cells[0] = 0;
  const saved = new_solo(cells, fixture.size);
  assert.deepEqual(JSON.parse(solo_hint(saved)), {index: 0, value, technique: 'naked_single', unit: null});
  assert.deepEqual(Array.from(solo_conflicts(saved)), []);
  const empty = new_solo(new Uint8Array(fixture.size * fixture.size), fixture.size);
  assert.equal(JSON.parse(solo_hint(empty)), null);
  const conflicting = update_solo(update_solo(empty, JSON.stringify({type:'set',index:0,value:1})), JSON.stringify({type:'set',index:1,value:1}));
  assert.deepEqual(Array.from(solo_conflicts(conflicting)), [0,1]);
  assert.throws(() => solo_hint(conflicting));
}
console.log('WASM shared conflicts and logical hints passed');

for (const size of [262, 4294967302, -250, 6.5, NaN, Infinity]) {
  assert.equal(validate_board(new Uint8Array(36), size), false);
  assert.throws(() => new_solo(new Uint8Array(36), size));
}
for (const blanks of [266, 4294967314, -238, 18.5, NaN, Infinity]) {
  assert.throws(() => new_generated_solo(6, 0, blanks));
}
assert.throws(() => new_generated_solo(6, -1, 18));
assert.throws(() => new_generated_solo(6, 1.5, 18));
console.log('WASM numeric boundary checks passed');
