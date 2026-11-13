import fs from 'node:fs';
import assert from 'node:assert/strict';
import * as core from '../apps/web/pkg/sudoku_wasm.js';
import { SoloSession, readSavedGames, saveKey, formatTime } from '../apps/web/session.js';

await core.default({module_or_path:fs.readFileSync(new URL('../apps/web/pkg/sudoku_wasm_bg.wasm',import.meta.url))});
class MemoryStorage {
  values = new Map();
  denyRead = false;
  denyWrite = false;
  getItem(key) { if (this.denyRead) throw new Error('blocked'); return this.values.get(key) ?? null; }
  setItem(key,value) { if (this.denyWrite) throw new Error('quota'); this.values.set(key,String(value)); }
}
let now = 0;
const storage = new MemoryStorage();
const session = new SoloSession(core,storage,() => now);
assert.deepEqual(readSavedGames(storage,core).map(s => s.status),['missing','missing']);
session.start(6,42);
const first = session.board.givens.findIndex(value => value === 0);
const given = session.board.givens.findIndex(value => value !== 0);
now = 1525;
session.apply({type:'toggle_note',index:first,value:6});
assert.equal(session.board.cells[first].notes,32);
session.apply({type:'set',index:first,value:1});
assert.equal(session.board.cells[first].notes,0);
session.apply({type:'undo'});
assert.equal(session.board.cells[first].notes,32);
session.apply({type:'redo'});
assert.equal(session.board.cells[first].value,1);
session.apply({type:'erase',index:first});
assert.equal(session.board.cells[first].value,0);
const beforeError = session.game;
assert.throws(() => session.apply({type:'erase',index:given}));
assert.equal(session.game,beforeError);
assert.throws(() => session.start(6,1),/ReplaceConfirmationRequired/);
assert.throws(() => session.start(7,1),/InvalidSize/);
now = 2000;
session.pause();
assert.equal(session.time,2000);
now = 900000;
assert.equal(session.time,2000);
const restore = new SoloSession(core,storage,() => now);
restore.restore(6);
assert.equal(restore.time,2000);
assert.equal(restore.running,false);
restore.resume();
now += 1000;
assert.equal(restore.time,3000);
restore.pause();
assert.equal(JSON.parse(storage.getItem(saveKey(6))).elapsedMs,3000);

const savedBeforeFailure = storage.getItem(saveKey(6));
restore.resume();
storage.denyWrite = true;
restore.apply({type:'toggle_note',index:first,value:2});
assert.equal(restore.saveError,true);
assert.equal(restore.board.cells[first].notes,2);
assert.equal(storage.getItem(saveKey(6)),savedBeforeFailure);
storage.denyWrite = false;
assert.equal(restore.save(),true);
assert.equal(restore.saveError,false);
assert.notEqual(storage.getItem(saveKey(6)),savedBeforeFailure);

session.start(9,42);
assert.equal(readSavedGames(storage,core).filter(s => s.status === 'ready').length,2);
const active = session.game;
const valid9 = storage.getItem(saveKey(9));
for (const corrupt of ['{','null','{}',JSON.stringify({...JSON.parse(valid9),version:2}),
  JSON.stringify({...JSON.parse(valid9),elapsedMs:-1}),JSON.stringify({...JSON.parse(valid9),selected:999}),
  JSON.stringify({...JSON.parse(valid9),game:'{}'})]) {
  storage.setItem(saveKey(9),corrupt);
  assert.equal(readSavedGames(storage,core).find(s => s.size === 9).status,'corrupt');
  assert.throws(() => session.restore(9));
  assert.equal(session.game,active);
  assert.equal(storage.getItem(saveKey(9)),corrupt);
  assert.throws(() => session.start(9,3),/ReplaceConfirmationRequired/);
}
storage.setItem(saveKey(9),valid9);
storage.denyRead = true;
assert.deepEqual(readSavedGames(storage,core).map(s => s.status),['unavailable','unavailable']);
assert.throws(() => session.start(6,1),/ReplaceConfirmationRequired/);
storage.denyRead = false;

const fixtures = JSON.parse(fs.readFileSync(new URL('../vectors/boards.json',import.meta.url)));
for (const fixture of fixtures) {
  storage.setItem(saveKey(fixture.size),JSON.stringify({version:1,game:core.new_solo(Uint8Array.from(fixture.uniquePuzzle),fixture.size),elapsedMs:0,selected:0,notesMode:false}));
  session.restore(fixture.size);
  session.resume();
  for (let index = 0; index < fixture.solution.length; index++) {
    if (fixture.uniquePuzzle[index] === 0) session.apply({type:'set',index,value:fixture.solution[index]});
  }
  assert(session.complete);
  assert.equal(session.running,false);
  assert(readSavedGames(storage,core).find(s => s.size === fixture.size).complete);
  const complete = session.game;
  assert.equal(session.apply({type:'undo'}),false);
  assert.equal(session.game,complete);
}
assert.equal(formatTime(0),'0:00');
assert.equal(formatTime(61500),'1:01');
assert.equal(formatTime(3661000),'1:01:01');
console.log('Web solo: real WASM commands, independent saves, timers, atomic storage failures, corrupt saves and completion passed');

if (process.argv.includes('--browser')) {
  const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
  const { createServer } = await import('node:http');
  const { fileURLToPath } = await import('node:url');
  const path = await import('node:path');
  const root = fileURLToPath(new URL('../apps/web/',import.meta.url));
  const mime = {'.html':'text/html','.js':'text/javascript','.css':'text/css','.wasm':'application/wasm','.svg':'image/svg+xml','.webmanifest':'application/manifest+json'};
  const server = createServer((req,res) => {
    const requestPath = decodeURIComponent(new URL(req.url,'http://localhost').pathname);
    const file = path.resolve(root,`.${requestPath === '/' ? '/index.html':requestPath}`);
    if (!file.startsWith(root)) { res.writeHead(403); res.end(); return; }
    fs.readFile(file,(error,data) => {
      if (error) { res.writeHead(404); res.end(); return; }
      res.writeHead(200,{'content-type':mime[path.extname(file)] || 'application/octet-stream','cache-control':'no-cache'});
      res.end(data);
    });
  });
  await new Promise(resolve => server.listen(0,'127.0.0.1',resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  let browser;
  try {
    browser = await chromium.launch({executablePath:process.env.CHROMIUM_PATH || '/usr/bin/chromium',headless:true,args:['--no-sandbox']});
    const context = await browser.newContext({viewport:{width:375,height:812},locale:'tr-TR'});
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror',error => errors.push(error.message));
    page.on('dialog',dialog => dialog.accept());
    await page.goto(origin);
    await page.getByTestId('solo-start').click();
    await page.getByTestId('game-start').click();
    await page.getByTestId('game-board').waitFor();
    assert.equal(await page.getByRole('gridcell').count(),36);
    await page.waitForFunction(() => navigator.serviceWorker.controller !== null);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),true);
    const editable = page.locator('[role="gridcell"][aria-readonly="false"]').first();
    await editable.click();
    const id = await editable.getAttribute('data-testid');
    await page.getByTestId('notes-toggle').click();
    await page.getByTestId('number-6').click();
    assert.match(await page.getByTestId(id).getAttribute('aria-label'),/notlar: 6/);
    await page.getByTestId('undo').click();
    assert.match(await page.getByTestId(id).getAttribute('aria-label'),/notlar: yok/);
    await page.getByTestId('redo').click();
    await page.getByTestId(id).focus();
    await page.keyboard.press('n');
    await page.keyboard.press('2');
    assert.match(await page.getByTestId(id).getAttribute('aria-label'),/girilebilir, 2/);
    await page.keyboard.press('Control+z');
    assert.match(await page.getByTestId(id).getAttribute('aria-label'),/notlar: 6/);
    const beforeReload = await page.evaluate(() => localStorage.getItem('sudoku-rivals.solo.v1.6'));
    await page.getByTestId('pause').click();
    assert.equal(await page.getByTestId('game-board').count(),0);
    await page.getByTestId('save-exit').click();
    await context.setOffline(true);
    await page.reload();
    await page.getByTestId('solo-resume-6').click();
    await page.getByTestId('game-board').waitFor();
    assert.match(await page.getByTestId(id).getAttribute('aria-label'),/notlar: 6/);
    assert(beforeReload);
    await page.getByTestId('back').click();
    await page.getByTestId('solo-start').click();
    await page.locator('input[name="size"][value="9"]').check();
    await page.getByTestId('game-start').click();
    await page.getByTestId('game-board').waitFor();
    assert.equal(await page.getByRole('gridcell').count(),81);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),true);
    await page.getByTestId('back').click();
    await page.evaluate(() => localStorage.setItem('sudoku-rivals.solo.v1.9','{corrupt'));
    await page.reload();
    await page.getByTestId('solo-start').waitFor();
    assert.match(await page.locator('main').textContent(),/kayıt|kaydı/i);
    assert.equal(await page.evaluate(() => localStorage.getItem('sudoku-rivals.solo.v1.9')),'{corrupt');
    // A denied write must be visible while the new live board remains usable.
    await page.getByTestId('solo-resume-6').click();
    await page.evaluate(() => { Storage.prototype.setItem = () => { throw new DOMException('QuotaExceededError'); }; });
    await page.locator('[role="gridcell"][aria-readonly="false"]').first().click();
    await page.getByTestId('number-1').click();
    assert.match(await page.locator('main').textContent(),/kaydedilemedi/i);
    assert.equal(await page.getByTestId('game-board').count(),1);
    await page.emulateMedia({colorScheme:'dark',reducedMotion:'reduce'});
    await page.setViewportSize({width:320,height:740});
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),true);
    assert.deepEqual(errors,[]);
    await context.close();
    console.log('Web browser: 375px/320px, keyboard/notes/undo/redo, pause, offline cold reload/new game, corrupt save and write failure passed');
  } finally {
    if (browser) await browser.close();
    await new Promise(resolve => server.close(resolve));
  }
}
