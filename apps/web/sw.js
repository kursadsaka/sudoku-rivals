// Bump this version when shipping changed app or WASM assets.
const CACHE = 'sudoku-rivals-app-v2';
const ASSETS = ['/', '/index.html', '/main.js', '/session.js', '/live.js', '/styles.css',
  '/icon.svg', '/manifest.webmanifest', '/pkg/sudoku_wasm.js', '/pkg/sudoku_wasm_bg.wasm'];
self.addEventListener('install', event => {
  event.waitUntil(caches.open(CACHE).then(cache => cache.addAll(ASSETS)));
});
self.addEventListener('activate', event => {
  event.waitUntil(caches.keys().then(keys => Promise.all(keys
    .filter(key => (key.startsWith('sudoku-rivals-solo-') || key.startsWith('sudoku-rivals-app-')) && key !== CACHE)
    .map(key => caches.delete(key)))).then(() => self.clients.claim()));
});
self.addEventListener('fetch', event => {
  const url = new URL(event.request.url);
  if (event.request.method !== 'GET' || url.origin !== self.location.origin || url.pathname.startsWith('/v1/')) return;
  if (event.request.mode === 'navigate') {
    event.respondWith(fetch(event.request).catch(() => caches.open(CACHE).then(cache => cache.match('/index.html'))));
  } else if (ASSETS.includes(url.pathname) && !url.search) {
    event.respondWith(caches.open(CACHE).then(async cache => (await cache.match(event.request)) || fetch(event.request)));
  }
});
