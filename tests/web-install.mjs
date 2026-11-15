// Chromium's own installability check of a served web root: manifest parse errors, installability
// errors, the icons it resolves, and the service worker. Usage:
//   SUDOKU_WEB_ROOT=dist/web node tests/web-install.mjs [path/to/sudoku-server]
// Starts the given server (default dist/sudoku-server) on a free loopback port and stops it afterwards.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtempSync, mkdirSync, rmSync } from 'node:fs';
import { createServer } from 'node:net';
import { fileURLToPath } from 'node:url';

const repo = fileURLToPath(new URL('..', import.meta.url));
const binary = process.argv[2] || `${repo}dist/sudoku-server`;
const webRoot = process.env.SUDOKU_WEB_ROOT || `${repo}dist/web`;
const port = await new Promise(resolve => { const s = createServer().listen(0, '127.0.0.1', () => { const p = s.address().port; s.close(() => resolve(p)); }); });
const server = spawn(binary, [], { env: { ...process.env, SUDOKU_BIND: '127.0.0.1', PORT: String(port), SUDOKU_PORT: '0', SUDOKU_WEB_ROOT: webRoot }, stdio: 'ignore' });
const playwright = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const chromium = playwright.chromium || playwright.default.chromium; // CommonJS package
let context, profile;
try {
  const origin = `http://127.0.0.1:${port}`;
  for (let i = 0; i < 100; i++) { try { if ((await fetch(origin + '/health')).ok) break; } catch {} await new Promise(r => setTimeout(r, 50)); }
  // A persistent profile: Playwright's default (incognito-like) context always reports `in-incognito`.
  mkdirSync(`${repo}target/tmp`, { recursive: true });
  profile = mkdtempSync(`${repo}target/tmp/web-install-`);
  context = await chromium.launchPersistentContext(profile, { executablePath: process.env.CHROMIUM_PATH || '/usr/bin/chromium', headless: true, args: ['--no-sandbox'] });
  const page = context.pages()[0] || await context.newPage();
  await page.goto(origin + '/');
  await page.evaluate(() => navigator.serviceWorker.ready.then(() => true));
  const client = await context.newCDPSession(page);
  const { url, errors, data } = await client.send('Page.getAppManifest');
  assert.equal(url, origin + '/manifest.webmanifest');
  assert.deepEqual(errors.map(e => e.message), [], 'manifest parse errors/warnings');
  const manifest = JSON.parse(data);
  const { installabilityErrors } = await client.send('Page.getInstallabilityErrors');
  assert.deepEqual(installabilityErrors, [], 'Chromium installability errors');
  // Every icon is fetchable with its declared type, from the served origin.
  for (const icon of manifest.icons) {
    const response = await fetch(new URL(icon.src, origin));
    assert.equal(response.status, 200, icon.src);
    assert.equal(response.headers.get('content-type'), icon.type, icon.src);
  }
  const link = await page.evaluate(() => [...document.querySelectorAll('link[rel~=icon],link[rel=apple-touch-icon]')].map(l => `${l.rel}:${l.getAttribute('href')}`));
  assert.deepEqual(link, ['icon:/icon.svg', 'icon:/icon-192.png', 'apple-touch-icon:/apple-touch-icon.png']);
  const touch = await fetch(origin + '/apple-touch-icon.png');
  assert.equal(touch.headers.get('content-type'), 'image/png');
  console.log(`Web install check passed: ${manifest.icons.length} manifest icons (${manifest.icons.map(i => `${i.sizes}/${i.purpose}`).join(', ')}), no manifest errors, no installability errors, apple-touch-icon served`);
} finally {
  await context?.close();
  server.kill();
  if (profile) rmSync(profile, { recursive: true, force: true });
}
