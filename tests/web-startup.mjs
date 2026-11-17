// Run with PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs node tests/web-startup.mjs.
// Optional --baseline /path/to/base/apps/web compares identical WASM and controlled asset delays.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = fileURLToPath(new URL('../apps/web/', import.meta.url));
let webRoot = root, delayed = false;
const requests = [];
const server = http.createServer(async (req, res) => {
  const pathname = new URL(req.url, 'http://localhost').pathname;
  requests.push(pathname);
  const name = pathname === '/' || pathname.startsWith('/r/') ? 'index.html' : pathname.slice(1);
  if (name.includes('..')) { res.writeHead(404).end(); return; }
  try {
    const bytes = await fs.readFile(path.join(webRoot, name));
    if (delayed) await new Promise(resolve => setTimeout(resolve, name.endsWith('.wasm') ? 600 : name.endsWith('.js') ? 80 : name === 'index.html' ? 100 : 0));
    res.setHeader('Content-Type', name.endsWith('.wasm') ? 'application/wasm' : name.endsWith('.js') ? 'text/javascript' : name.endsWith('.css') ? 'text/css' : name.endsWith('.html') ? 'text/html' : 'application/octet-stream');
    res.end(bytes);
  } catch { res.writeHead(404).end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ executablePath:process.env.CHROMIUM_PATH || '/usr/bin/chromium', headless:true, args:['--no-sandbox'] });
const errors = [];
async function pageWith(storage = {}, options = {}) {
  const context = await browser.newContext({ locale:'tr-TR', serviceWorkers:'block', ...options });
  await context.addInitScript(values => {
    for (const [key,value] of Object.entries(values)) localStorage.setItem(key,value);
    new MutationObserver(() => {
      if (document.querySelector('[data-screen="home"]') && !performance.getEntriesByName('test-home').length) {
        performance.mark('test-home');
        requestAnimationFrame(() => requestAnimationFrame(() => performance.mark('test-home-frame')));
      }
      if (document.querySelector('[data-testid="solo-start"]:enabled') && !performance.getEntriesByName('test-ready').length) performance.mark('test-ready');
    }).observe(document, { childList:true, subtree:true });
  }, storage);
  const page = await context.newPage();
  page.on('pageerror', error => errors.push(String(error)));
  return {page,context};
}
try {
  const core = await import('../apps/web/pkg/sudoku_wasm.js');
  await core.default({module_or_path:await fs.readFile(path.join(root,'pkg/sudoku_wasm_bg.wasm'))});
  const { SoloSession, saveKey } = await import('../apps/web/session.js');
  const { LIVE_KEY } = await import('../apps/web/live.js');
  const values = new Map();
  const storage = {getItem:key=>values.get(key)??null,setItem:(key,value)=>values.set(key,value)};
  new SoloSession(core,storage).start(6,42);
  values.set(saveKey(9),'{broken');
  values.set(LIVE_KEY,'{protected-live-save');
  const saved = Object.fromEntries(values);
  for (const invite of [false,true]) {
    const initial = invite ? Object.fromEntries(Object.entries(saved).filter(([key])=>key!==LIVE_KEY)) : saved;
    const {page,context} = await pageWith(initial);
    let release;
    const gate = new Promise(resolve=>{release=resolve;});
    await page.route('**/*.wasm', async route=>{await gate;await route.continue();});
    await page.goto(origin + (invite ? '/r/ABC234' : '/'),{waitUntil:'domcontentloaded'});
    await page.locator('[data-testid="engine-status"]').waitFor();
    assert.equal(await page.locator('[data-testid="solo-start"]').isDisabled(),true);
    assert.equal(await page.locator('#settings-button').isDisabled(),true);
    await page.locator('[data-testid="solo-start"]').dispatchEvent('click');
    await page.locator('[data-testid="friend-start"]').dispatchEvent('click');
    await page.locator('#brand-home').click();
    await page.keyboard.press('Enter');
    assert.equal(await page.locator('#content [data-screen="home"]').count(),1);
    assert.equal(await page.locator('[data-testid="solo-resume"]').count(),0);
    assert.deepEqual(await page.evaluate(()=>Object.fromEntries(Object.entries(localStorage))),initial);
    release();
    await page.waitForFunction(()=>performance.getEntriesByName('sudoku-engine-ready').length);
    if (invite) {
      assert.equal(new URL(page.url()).pathname,'/r/ABC234');
      assert.equal(await page.locator('input[data-live-field="invite"]').inputValue(),'ABC234');
    } else {
      assert.equal(await page.locator('[data-testid="solo-start"]').isEnabled(),true);
      await page.locator('[data-testid="solo-resume-6"]').click();
      await page.locator('#content [data-screen="solo-game"]').waitFor();
    }
    assert.equal(await page.evaluate(key=>localStorage.getItem(key),saveKey(9)),saved[saveKey(9)]);
    assert.equal(await page.evaluate(key=>localStorage.getItem(key),LIVE_KEY),initial[LIVE_KEY] ?? null);
    await context.close();
  }
  for (const failure of ['**/*.wasm','**/pkg/sudoku_wasm.js']) {
    const initial = Object.fromEntries(Object.entries(saved).filter(([key])=>key!==LIVE_KEY));
    const {page,context} = await pageWith(initial);
    await page.route(failure,route=>route.abort());
    await page.goto(origin+'/r/ABC234');
    await page.locator('#reload').waitFor();
    assert.equal(await page.locator('#settings-button').isDisabled(),true);
    assert.deepEqual(await page.evaluate(()=>Object.fromEntries(Object.entries(localStorage))),initial);
    await page.unroute(failure);
    await page.locator('#reload').click();
    await page.waitForFunction(()=>performance.getEntriesByName('sudoku-engine-ready').length);
    assert.equal(await page.locator('input[data-live-field="invite"]').inputValue(),'ABC234');
    await context.close();
  }
  const blocked = await pageWith();
  await blocked.context.addInitScript(()=>Object.defineProperty(window,'localStorage',{get(){throw new Error('blocked');}}));
  await blocked.page.goto(origin);
  await blocked.page.waitForFunction(()=>performance.getEntriesByName('sudoku-engine-ready').length);
  assert.match(await blocked.page.locator('[data-testid="storage-status"]').first().innerText(),/erişilemiyor/);
  await blocked.context.close();
  assert.deepEqual(errors,[]);
  console.log('PASS: pending actions, invite retention, valid/corrupt/protected saves, blocked storage, WASM/glue failure and reload');
  const offline = await pageWith({}, {serviceWorkers:'allow'});
  await offline.page.goto(origin);
  await offline.page.evaluate(()=>navigator.serviceWorker.ready);
  await offline.page.reload();
  await offline.page.waitForFunction(()=>navigator.serviceWorker.controller && performance.getEntriesByName('sudoku-engine-ready').length);
  await offline.context.setOffline(true);
  await offline.page.reload();
  await offline.page.waitForFunction(()=>performance.getEntriesByName('sudoku-engine-ready').length);
  assert.equal(await offline.page.locator('[data-testid="solo-start"]').isEnabled(),true);
  await offline.context.close();
  console.log('PASS: installed service worker opens home and engine offline');
  delayed = true;
  const baseline = process.argv.indexOf('--baseline');
  for (const [label,directory] of [...(baseline>=0 ? [['baseline',process.argv[baseline+1]]] : []),['candidate',root]]) {
    webRoot = directory;
    const samples = [];
    for (let i=0;i<5;i++) {
      const {page,context} = await pageWith();
      requests.length=0;
      await page.goto(origin);
      await page.waitForFunction(()=>performance.getEntriesByName('test-ready').length && performance.getEntriesByName('test-home-frame').length);
      samples.push(await page.evaluate(()=>({
        home:performance.getEntriesByName('test-home')[0].startTime,
        homeFrame:performance.getEntriesByName('test-home-frame')[0].startTime,
        ready:performance.getEntriesByName('test-ready')[0].startTime,
        wasm:performance.getEntriesByType('resource').filter(x=>x.name.endsWith('.wasm')).map(x=>({start:x.startTime,end:x.responseEnd,initiator:x.initiatorType}))
      })));
      assert.equal(requests.filter(x=>x.endsWith('.wasm')).length,1,'preload must reuse the single WASM request (SW blocked)');
      await context.close();
    }
    console.log(JSON.stringify({label,delays:{html:100,js:80,wasm:600},samples}));
  }
} finally { await browser.close(); await new Promise(resolve=>server.close(resolve)); }
