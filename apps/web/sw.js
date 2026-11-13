// Bump this version when shipping changed app or WASM assets.
const CACHE = 'sudoku-rivals-solo-v1';
const ASSETS = ['./', './index.html', './main.js', './session.js', './styles.css', './icon.svg',
  './manifest.webmanifest', './pkg/sudoku_wasm.js', './pkg/sudoku_wasm_bg.wasm'];
self.addEventListener('install', event => {
  event.waitUntil(caches.open(CACHE).then(cache => cache.addAll(ASSETS)));
});
self.addEventListener('activate', event => {
  event.waitUntil(caches.keys().then(keys => Promise.all(keys
    .filter(key => key.startsWith('sudoku-rivals-solo-') && key !== CACHE)
    .map(key => caches.delete(key)))).then(() => self.clients.claim()));
});
self.addEventListener('fetch', event => {
  const url = new URL(event.request.url);
  if (event.request.method !== 'GET' || url.origin !== self.location.origin) return;
  event.respondWith(caches.open(CACHE).then(async cache => {
    const cached = await cache.match(event.request);
    if (cached) return cached;
    // Navigation stays within this app's scope; never cache unrelated room/API data.
    if (event.request.mode === 'navigate') {
      try { return await fetch(event.request); }
      catch { return cache.match('./index.html'); }
    }
    return fetch(event.request);
  }));
});
