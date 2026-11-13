import fs from 'node:fs';
import assert from 'node:assert/strict';
const contract = JSON.parse(fs.readFileSync(new URL('../docs/ui-contract.json', import.meta.url)));
assert.equal(contract.schemaVersion, 1);
const screens = new Map(contract.screens.map(screen => [screen.id, screen]));
assert.equal(screens.size, contract.screens.length);
for (const screen of screens.values()) {
  assert(screen.states.length > 0);
  for (const id of screen.elements) assert(contract.elements[id], `${screen.id}: ${id}`);
  for (const target of Object.values(screen.transitions)) assert(screens.has(target), target);
}
for (const [id, element] of Object.entries(contract.elements)) {
  assert(contract.layout.regions.includes(element.region), id);
  if (element.textKey) assert(contract.strings[element.textKey], `${id}: missing string`);
}
for (const [key, translations] of Object.entries(contract.strings)) {
  const placeholders = value => [...value.matchAll(/\{([^}]+)\}/g)].map(m => m[1]).sort();
  assert(translations.tr && translations.en, key);
  assert.deepEqual(placeholders(translations.tr), placeholders(translations.en), key);
}
const luminance = hex => {
  const rgb = hex.match(/[a-f\d]{2}/gi).map(v => parseInt(v, 16) / 255)
    .map(v => v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
  return rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
};
for (const theme of ['light', 'dark']) {
  for (const pair of contract.contrastPairs) {
    const fg = luminance(contract.tokens[theme][pair.foreground]);
    const bg = luminance(contract.tokens[theme][pair.background]);
    const ratio = (Math.max(fg, bg) + 0.05) / (Math.min(fg, bg) + 0.05);
    assert(ratio >= pair.minimum, `${theme}: ${pair.foreground}/${pair.background} ${ratio}`);
  }
}
for (const id of ['notes-toggle', 'erase', 'undo', 'redo', 'hint']) {
  assert(!screens.get('duel-game').elements.includes(id));
}
assert(!screens.get('race-game').elements.includes('hint'));
assert(!screens.get('solo-paused').elements.includes('game-board'));
console.log('UI contract: screen links, IDs, translations, mode controls and contrast passed');
