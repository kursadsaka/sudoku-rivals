import fs from 'node:fs';
import assert from 'node:assert/strict';
import init, { validate_board } from '../apps/web/pkg/sudoku_wasm.js';
await init({module_or_path: fs.readFileSync(new URL('../apps/web/pkg/sudoku_wasm_bg.wasm', import.meta.url))});
for (const fixture of JSON.parse(fs.readFileSync(new URL('../vectors/boards.json', import.meta.url)))) {
  for (const [key, expected] of [['uniquePuzzle', true], ['solution', true], ['invalidDuplicate', false], ['multipleSolutions', true]]) {
    assert.equal(validate_board(Uint8Array.from(fixture[key]), fixture.size), expected, `${fixture.size} ${key}`);
  }
}
console.log('WASM fixture checks passed');
