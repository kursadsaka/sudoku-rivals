import { LiveScreens } from '/live.js';
import init, * as core from '/pkg/sudoku_wasm.js';
import { SoloSession, readSavedGames, formatTime } from '/session.js';
import { cue as soundCue, preloadSound, setSound } from '/sound.js';

const text = {
  tr: {
    skip:'İçeriğe geç', home:'Ana sayfa', back:'Geri', settings:'Ayarlar', loading:'Oyun hazırlanıyor…',
    eyebrow:'SUDOKU, KENDİ HIZINDA', title:'Biraz odaklan.', intro:'Bir tahta seç. Notlarını al, hamleni yap. İstediğin zaman kaldığın yerden devam et.',
    solo:'Tek başına oyna', soloDescription:'6×6 veya 9×9. İnternetsiz de yanında.', friends:'Arkadaşınla yarış', friendsDescription:'Klasik yarış ve 90 saniyelik düello.',
    friendsPending:'Arkadaş maçları bu sürümde henüz hazır değil. Şimdilik tek başına oynayabilirsin.', continue:'Kaldığın yerden', resume:'Devam et', completed:'Tamamlandı', savedGame:'Kayıtlı oyun',
    choose:'Tahtanı seç', chooseIntro:'Kısa bir mola ya da biraz daha uzun bir oyun.', boardSize:'Tahta boyutu', six:'Daha küçük bir tahta', nine:'Klasik Sudoku', start:'Oyunu başlat',
    generatorNote:'Her oyun tek çözümlü, tahmin gerektirmeyen yeni bir bulmacayla başlar.', difficulty:'Zorluk', easy:'Kolay', medium:'Orta', hard:'Zor', soloShort:'Solo', preparing:'Bulmaca hazırlanıyor…',
    easyHelp:'Her adımda bir sayının satır, sütun veya kutuda girebileceği tek yer vardır.', mediumHelp:'Tek yer aramak yetmez; bazı hücrelerde adayları sayıp kalan tek sayıyı bulursun.', hardHelp:'Tekliler tıkanır; önce aday elemen gerekir (kilitli adaylar, çiftler).',
    replace:'Bu boyuttaki kayıtlı oyun yeni oyunla değiştirilecek. Devam edilsin mi?', replaceCorrupt:'Bu boyuttaki kayıt açılamıyor. Yeni oyun başlatırsan bu kayıt değiştirilecek. Devam edilsin mi?',
    paused:'Kısa bir mola.', pausedIntro:'Tahtan ve notların burada. Hazır olduğunda devam et.', pause:'Duraklat', saveExit:'Kaydedip çık', discardExit:'Kaydetmeden çık', confirmDiscard:'Kaydedilemeyen değişiklikler kaybolacak. Önceki kayıt korunacak. Yine de çıkılsın mı?', elapsed:'Geçen süre',
    numbers:'Bir sayı seç', notes:'Kalem', erase:'Sil', undo:'Geri al', redo:'Yinele', hint:'İpucu', hintApply:'Bu sayıyı yaz', hintDismiss:'Kapat', cancel:'Vazgeç',
    hintNone:'Bu konumda tekli tekniklerle açıklanabilir bir ipucu bulunamadı.', hintConflict:'İpucu için önce tahtadaki çakışmaları düzelt.', hintUnavailable:'Bu tahta için açıklanabilir ipucu bulunamadı.',
    hintNaked:'{row}. satır, {column}. sütunda yalnızca {value} kullanılabilir. Aynı satır, sütun ve kutudaki sayılar diğer adayları eliyor.',
    hintHidden:'{unit} içinde {value} yalnızca {row}. satır, {column}. sütuna yerleşebilir.',
    hintLocked:'{unit} içinde {value} yalnızca {target} ile kesişen hücrelere girebilir; {target} içindeki diğer hücrelerden {value} elenir.', hintNakedPair:'{unit} içinde iki hücre yalnızca {a} ve {b} alabilir; bu sayılar {unit} içindeki diğer hücrelerden elenir.', hintHiddenPair:'{unit} içinde {a} ve {b} yalnızca aynı iki hücreye girebilir; o hücrelerin diğer adayları elenir.',
    hintNakedThen:'Böylece {row}. satır, {column}. sütuna yalnızca {value} kalır.', hintHiddenThen:'Böylece {unit} içinde {value} yalnızca {row}. satır, {column}. sütuna girebilir.', row:'{number}. satır', column:'{number}. sütun', box:'{number}. kutu',
    inProgress:'Oyun devam ediyor', conflict:'Çakışan hücreleri kontrol et.', conflictShort:'Çakışma var', saved:'Bu cihazda kaydedildi', savingFailed:'Oyun kaydedilemedi. Açık oyunun korunuyor.', retry:'Tekrar dene',
    corrupt:'{size} kaydı açılamadı. Kaydın korunuyor; tekrar deneyebilir veya farklı boyutta oynayabilirsin.', storageUnavailable:'Yerel kayıtlara erişilemiyor. Bu oturumda oynayabilirsin; kalıcı kayıt garanti edilemiyor.',
    cell:'Satır {row}, sütun {column}, {kind}, {value}, notlar: {notes}{conflict}', given:'verilen', locked:'kilitli', editable:'girilebilir', empty:'boş', noNotes:'yok', cellConflict:', çakışma var',
    enterDigit:'{digit} yaz', selected:'{row}. satır · {column}. sütun', givenSelected:'Verilen hücre · değiştirilemez', chooseCell:'Tahtadan bir hücre seç.', notesList:'Notlar: {notes}',
    keyHelp:'Klavye: oklar ile gezin · 1–{size} sayı · N kalem · Delete sil · Ctrl/⌘ Z geri al',
    resultTitle:'Tahta tamamlandı.', resultIntro:'Her satır, sütun ve kutu yerli yerinde.', newGame:'Yeni oyun', offlinePreparing:'Çevrimdışı kullanım hazırlanıyor', offlineReady:'Çevrimdışı oynamaya hazır', offlineUnavailable:'Çevrimdışı açılış henüz hazır değil',
    offlineLocal:'Solo oyun için bağlantı gerekmez', sound:'Ses', haptics:'Titreşim', motion:'Hareketi azalt', language:'Dil', storage:'Yerel kayıtlar', preferencesNote:'Kayıtlar yalnızca bu tarayıcıda saklanır. Tarayıcı verilerini temizlemek kayıtlarını da kaldırır.',
    soundNote:'Tuş, hata ve bitiş sesleri. Varsayılan olarak kapalıdır.', hapticsNote:'Sayı girişinde, hatada ve bitişte kısa titreşim.', theme:'Tema', themeSystem:'Sistem', themeLight:'Açık', themeDark:'Koyu', settingsSaved:'Tercihler kaydedildi', settingsFailed:'Tercihler bu oturum için uygulandı; kaydedilemedi.', loadFailed:'Oyun yüklenemedi. Sayfayı yenileyip tekrar deneyebilirsin.', loadAgain:'Sayfayı yenile',
    genericError:'Bu işlem tamamlanamadı. Açık oyunun korunuyor.', badCell:'Verilen hücreler değiştirilemez.', filledNotes:'Not eklemek için önce hücredeki sayıyı sil.',
  },
  en: {
    skip:'Skip to content', home:'Home', back:'Back', settings:'Settings', loading:'Getting your game ready…',
    eyebrow:'SUDOKU, AT YOUR OWN PACE', title:'A moment to focus.', intro:'Choose a board. Make a note, make your move. Pick up where you left off whenever you like.',
    solo:'Play solo', soloDescription:'6×6 or 9×9. Yours to play offline, too.', friends:'Play with a friend', friendsDescription:'Classic races and 90-second duels.',
    friendsPending:'Friend matches are not ready in this version. You can play solo for now.', continue:'Pick up where you left off', resume:'Resume', completed:'Complete', savedGame:'Saved game',
    choose:'Choose your board', chooseIntro:'A short break, or a little more time to think.', boardSize:'Board size', six:'A smaller board', nine:'Classic Sudoku', start:'Start game',
    generatorNote:'Every game starts with a new puzzle that has one solution and needs no guessing.', difficulty:'Difficulty', easy:'Easy', medium:'Medium', hard:'Hard', soloShort:'Solo', preparing:'Preparing your puzzle…',
    easyHelp:'Every step is a digit with only one place left in a row, column or box.', mediumHelp:'Looking for the one place is not enough; some cells need their candidates counted until one digit is left.', hardHelp:'Singles run out; candidates must be eliminated first (locked candidates, pairs).',
    replace:'The saved game of this size will be replaced by a new game. Continue?', replaceCorrupt:'The save of this size cannot be opened. Starting a new game will replace it. Continue?',
    paused:'Take your time.', pausedIntro:'Your board and notes are here. Continue whenever you are ready.', pause:'Pause', saveExit:'Save and exit', discardExit:'Exit without saving', confirmDiscard:'Unsaved changes will be lost. The previous save will remain. Exit anyway?', elapsed:'Elapsed',
    numbers:'Choose a number', notes:'Notes', erase:'Erase', undo:'Undo', redo:'Redo', hint:'Hint', hintApply:'Enter this number', hintDismiss:'Dismiss', cancel:'Cancel',
    hintNone:'No explainable hint using singles was found here.', hintConflict:'Resolve the board conflicts before asking for a hint.', hintUnavailable:'No explainable hint was found for this board.',
    hintNaked:'Only {value} can go in row {row}, column {column}. The numbers in its row, column and box rule out the other candidates.',
    hintHidden:'In {unit}, {value} can only go in row {row}, column {column}.',
    hintLocked:'In {unit}, {value} fits only where it crosses {target}, so {value} is removed from the rest of {target}.', hintNakedPair:'In {unit}, two cells can only hold {a} and {b}, so both are removed from the other cells of {unit}.', hintHiddenPair:'In {unit}, {a} and {b} fit only the same two cells, so those cells lose their other candidates.',
    hintNakedThen:'That leaves only {value} for row {row}, column {column}.', hintHiddenThen:'So in {unit}, {value} can only go in row {row}, column {column}.', row:'row {number}', column:'column {number}', box:'box {number}',
    inProgress:'Game in progress', conflict:'Check the conflicting cells.', conflictShort:'Conflict', saved:'Saved on this device', savingFailed:'Could not save. Your open game is preserved.', retry:'Retry',
    corrupt:'The {size} save could not be opened. It is preserved; you can retry or play the other size.', storageUnavailable:'Local saves are unavailable. You can play this session, but saving cannot be guaranteed.',
    cell:'Row {row}, column {column}, {kind}, {value}, notes: {notes}{conflict}', given:'given', locked:'locked', editable:'editable', empty:'empty', noNotes:'none', cellConflict:', conflict',
    enterDigit:'Enter {digit}', selected:'Row {row} · Column {column}', givenSelected:'Given cell · cannot be changed', chooseCell:'Choose a cell on the board.', notesList:'Notes: {notes}',
    keyHelp:'Keyboard: arrows to move · 1–{size} enter · N notes · Delete erase · Ctrl/⌘ Z undo',
    resultTitle:'Board complete.', resultIntro:'Every row, column and box is in its place.', newGame:'New game', offlinePreparing:'Preparing offline play', offlineReady:'Ready to play offline', offlineUnavailable:'Offline opening is not ready yet',
    offlineLocal:'Solo play needs no connection', sound:'Sound', haptics:'Haptics', motion:'Reduce motion', language:'Language', storage:'Local saves', preferencesNote:'Saves stay in this browser. Clearing browser data also removes your saves.',
    soundNote:'Key, error and finish cues. Off by default.', hapticsNote:'A short buzz on entries, mistakes and finishes.', theme:'Theme', themeSystem:'System', themeLight:'Light', themeDark:'Dark', settingsSaved:'Preferences saved', settingsFailed:'Preferences applied for this session, but could not be saved.', loadFailed:'The game could not load. Refresh the page to try again.', loadAgain:'Refresh page',
    genericError:'This action could not be completed. Your open game is preserved.', badCell:'Given cells cannot be changed.', filledNotes:'Erase the number in this cell before adding notes.',
  },
};

const icons = {
  arrow:'<path d="M5 12h14m-5-5 5 5-5 5"/>', back:'<path d="m14 6-6 6 6 6"/>', pause:'<path d="M9 5v14M15 5v14"/>',
  solo:'<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M9.5 4v16M15 4v16M4 9.5h16M4 15h16"/>',
  friends:'<path d="M9 20v-2a4 4 0 0 1 8 0v2M3 20v-2a4 4 0 0 1 4-4"/><circle cx="13" cy="7" r="4"/><path d="M5 4a4 4 0 0 0 0 7"/>',
  notes:'<path d="m15 4 5 5M4 20l5-1L21 7a2 2 0 0 0-4-4L5 15l-1 5Z"/>',
  erase:'<path d="m4 12 7-8a2 2 0 0 1 3 0l6 6a2 2 0 0 1 0 3l-7 7H9l-5-5a2 2 0 0 1 0-3ZM9 7l9 9M13 20h8"/>',
  undo:'<path d="M4 10h10a6 6 0 0 1 0 12M4 10l5-5M4 10l5 5"/>', redo:'<path d="M20 10H10a6 6 0 0 0 0 12M20 10l-5-5M20 10l-5 5"/>',
  hint:'<path d="M9 18h6M10 21h4M8 13a6 6 0 1 1 8 0c-1 1-1 2-1 3H9c0-1 0-2-1-3Z"/>', check:'<path d="m5 12 4 4L19 6"/>',
  warning:'<path d="m12 3 10 18H2L12 3ZM12 9v5M12 17v1"/>', play:'<path d="m9 5 11 7-11 7V5Z"/>',
  trophy:'<path d="M7 3h10v6a5 5 0 0 1-10 0V3ZM7 5H3v1a4 4 0 0 0 4 4M17 5h4v1a4 4 0 0 1-4 4M12 14v4M7.5 21h9M9.5 18h5"/>', flag:'<path d="M6 21V4m0 0h11l-2.5 4L17 12H6"/>', equal:'<path d="M6 9h12M6 15h12"/>',
  share:'<path d="M12 15V4m0 0L8 8m4-4 4 4M5 13v5a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-5"/>', close:'<path d="M6 6l12 12M18 6 6 18"/>',
};
const svg = name => `<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${icons[name] || ''}</svg>`;
const escape = value => String(value).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
let storage;
try { storage = window.localStorage; }
catch { storage = { getItem() { throw new Error('StorageUnavailable'); }, setItem() { throw new Error('StorageUnavailable'); } }; }
// Developer-only diagnostics (selection readout, save note, keyboard help, page chrome in games).
// Enable with ?debug=1 (remembered in this browser); ?debug=0 turns it off again.
let debug = false;
try {
  const flag = new URLSearchParams(location.search).get('debug');
  if (flag === '1') storage.setItem('sudoku-rivals.debug','1');
  else if (flag === '0') storage.setItem('sudoku-rivals.debug','0');
  debug = storage.getItem('sudoku-rivals.debug') === '1';
} catch { debug = new URLSearchParams(location.search).get('debug') === '1'; }
const LEVELS = ['easy','medium','hard'];
const THEMES = ['system','light','dark'];
// The standard Vibration API exists on Android browsers, not on iOS Safari; the setting is offered only
// where it can work (a touch device that has the API).
const canVibrate = typeof navigator.vibrate === 'function' && matchMedia('(pointer: coarse)').matches;
const BUZZ = {tap:8,type:6,toggle:10,select:8,ready:12,error:[18,50,18],success:[12,70,24],attention:[30,60,30]};
// Board size defaults to 9×9 everywhere until the player picks one; the last pick is remembered.
let preferences = { language:navigator.language.startsWith('tr') ? 'tr' : 'en', reduceMotion:matchMedia('(prefers-reduced-motion: reduce)').matches, sound:false, haptics:false, theme:'system', soloSize:9, soloDifficulty:'easy', liveSize:9, liveMode:'race', liveDifficulty:'easy', displayName:'' };
try {
  const stored = JSON.parse(storage.getItem('sudoku-rivals.preferences.v1'));
  if (stored && ['tr','en'].includes(stored.language) && typeof stored.reduceMotion === 'boolean') preferences = { language:stored.language, reduceMotion:stored.reduceMotion, sound:stored.sound === true, haptics:stored.haptics === true,
    theme:THEMES.includes(stored.theme) ? stored.theme : 'system',
    soloSize:[6,9].includes(stored.soloSize) ? stored.soloSize : 9, soloDifficulty:LEVELS.includes(stored.soloDifficulty) ? stored.soloDifficulty : 'easy', liveSize:[6,9].includes(stored.liveSize) ? stored.liveSize : 9,
    liveMode:['race','duel'].includes(stored.liveMode) ? stored.liveMode : 'race', liveDifficulty:LEVELS.includes(stored.liveDifficulty) ? stored.liveDifficulty : 'easy', displayName:typeof stored.displayName === 'string' ? stored.displayName.slice(0,32) : '' };
} catch { /* Defaults remain usable when storage is blocked or malformed. */ }
const t = (key, values = {}) => Object.entries(values).reduce((message,[name,value]) => message.replaceAll(`{${name}}`,String(value)),text[preferences.language][key] || key);
const content = document.querySelector('#content');
const announcer = document.querySelector('#announcer');
const offline = document.querySelector('#offline-status');
const settingsButton = document.querySelector('#settings-button');
let session;
let live;
let screen = 'home';
let selectedSize = preferences.soloSize;
let selectedDifficulty = preferences.soloDifficulty;
let homeNotice = '';
let gameNotice = '';
let hint = null;
let offlineState = 'offlinePreparing';
let settingsReturn = 'home';
let settingsNotice = '';
let ready = false;
let liveSeen = null;
// Sound is opt-in: off unless this browser has it switched on in Settings.
if (preferences.sound) setSound(true);

function announce(message) { announcer.textContent = message; }
// One call for both kinds of feedback; each is opt-in and off by default.
function cue(name,options) {
  soundCue(name,options);
  if (preferences.haptics && canVibrate && BUZZ[name]) { try { navigator.vibrate(BUZZ[name]); } catch { /* Not worth surfacing. */ } }
}
function savePreferences() {
  try { storage.setItem('sudoku-rivals.preferences.v1',JSON.stringify(preferences)); return true; }
  catch { return false; }
}
function setOffline(state) {
  offlineState = state;
  offline.textContent = t(state);
  offline.parentElement.dataset.state = state;
}
function applyPreferences() {
  document.documentElement.lang = preferences.language;
  document.documentElement.dataset.reduceMotion = String(preferences.reduceMotion);
  // The same rule as the inline script in index.html, which applies it before the first paint.
  if (preferences.theme === 'system') delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = preferences.theme;
  for (const meta of document.querySelectorAll('meta[name="theme-color"]')) {
    meta.content = preferences.theme === 'system' ? meta.dataset.color : preferences.theme === 'dark' ? '#121214':'#f4f1ea';
  }
  if (debug) document.documentElement.dataset.debug = 'true';
  document.querySelector('.skip-link').textContent = t('skip');
  document.querySelector('#brand-home').setAttribute('aria-label',`Sudoku Rivals · ${t('home')}`);
  settingsButton.setAttribute('aria-label',t('settings'));
  settingsButton.title = t('settings');
  setOffline(offlineState);
}
function focusContent() { content.focus({ preventScroll:true }); window.scrollTo({top:0,behavior:'instant'}); }
function navigate(next) { screen = next; hint = null; gameNotice = ''; render(); focusContent(); }
function backButton(action = 'home', label = 'back') { return `<button class="back-button" data-action="${action}" data-testid="back" aria-label="${t(label)}">${svg('back')}<span>${t(label)}</span></button>`; }
function storageMessage() {
  return session.saveError ? `<div class="notice error" data-testid="storage-status" role="alert">${t('savingFailed')} <button data-action="save" data-testid="retry">${t('retry')}</button></div>` : '';
}
// Mode art: one player for solo; two players for a friend match, you in blue and your rival in amber.
const art = {
  solo:'<svg viewBox="0 0 40 40" aria-hidden="true"><circle cx="20" cy="13" r="7" fill="currentColor"/><path d="M6 36c0-8.3 6.3-14 14-14s14 5.7 14 14Z" fill="currentColor"/></svg>',
  friends:'<svg viewBox="0 0 40 40" aria-hidden="true"><circle class="rival" cx="27.5" cy="14" r="5.5"/><path class="rival" d="M17.5 34c0-6.5 4.2-10.5 10-10.5s10 4 10 10.5Z"/><circle class="you" cx="13.5" cy="13" r="6.5" stroke-width="2.5"/><path class="you" d="M1.5 35c0-7.6 5.2-12.5 12-12.5s12 4.9 12 12.5Z" stroke-width="2.5"/></svg>',
};
function renderHome() {
  const saves = readSavedGames(storage, core);
  const readySaves = saves.filter(slot => slot.status === 'ready');
  const problems = saves.filter(slot => ['corrupt','unavailable'].includes(slot.status));
  const card = (extra,action,testid,art,title,description) => `<button class="mode-card ${extra}" data-action="${action}" data-testid="${testid}"><span class="mode-art" aria-hidden="true">${art}</span><span class="mode-text"><span class="mode-title">${t(title)}</span><span class="mode-description">${t(description)}</span></span><span class="mode-go" aria-hidden="true">${svg('arrow')}</span></button>`;
  content.innerHTML = `<section class="home" data-screen="home">
    <div class="hero"><p class="eyebrow">${t('eyebrow')}</p><h1>${t('title')}</h1><p class="lede">${t('intro')}</p></div>
    ${homeNotice ? `<p class="notice" role="status">${escape(homeNotice)}</p>` : ''}
    <div class="mode-options">
      ${card('primary','setup','solo-start',art.solo,'solo','soloDescription')}
      ${card('','friends','friend-start',art.friends,'friends','friendsDescription')}
    </div>
    ${live?.view()?.snapshot && !live.view().voluntarily_left ? `<button class="resume-card live-resume" data-action="live-resume" data-testid="live-resume"><span>${preferences.language==='tr'?'Maçına dön':'Return to your match'}</span><span class="resume-go" aria-hidden="true">${svg('arrow')}</span></button>`:''}
    ${readySaves.length ? `<p class="section-label">${t('continue')}</p><div class="resume-list" data-testid="solo-resume">${readySaves.map(slot => `<button class="resume-card" data-action="restore" data-size="${slot.size}" data-testid="solo-resume-${slot.size}"><span class="resume-main"><span class="size-badge">${slot.size}×${slot.size}</span><span><strong>${slot.complete ? t('completed') : t('savedGame')}</strong><small>${[levelOf(slot.saved.game) ? t(levelOf(slot.saved.game)) : '',formatTime(slot.saved.elapsedMs)].filter(Boolean).join(' · ')}</small></span></span><span class="resume-go">${slot.complete ? svg('arrow') : t('resume') + svg('play')}</span></button>`).join('')}</div>` : ''}
    ${problems.map(slot => `<p class="notice error" data-testid="storage-status" role="alert">${escape(slot.status === 'corrupt' ? t('corrupt',{size:`${slot.size}×${slot.size}`}) : t('storageUnavailable'))} <button data-action="refresh-saves">${t('retry')}</button></p>`).join('')}
  </section>`;
}
// The level is derived from the puzzle's givens by the engine, so saves carry no extra field and
// older or unrated puzzles simply show no level.
const rated = new Map();
function levelOf(game) {
  if (!core.solo_difficulty || !game) return null;
  const key = JSON.stringify(JSON.parse(game).givens);
  if (!rated.has(key)) {
    try { rated.set(key,JSON.parse(core.solo_difficulty(game)).level); } catch { rated.set(key,null); }
  }
  return rated.get(key);
}
const sizeAndLevel = (size,game) => { const level = levelOf(game); return `${size}×${size}${level ? ` · ${t(level)}`:''}`; };
function renderSetup() {
  const art = size => `<svg class="size-art" viewBox="0 0 36 36" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><rect x="1.5" y="1.5" width="33" height="33" rx="6"/><path d="${size === 6 ? 'M18 1.5v33M1.5 12.5h33M1.5 23.5h33':'M12.5 1.5v33M23.5 1.5v33M1.5 12.5h33M1.5 23.5h33'}"/></svg>`;
  content.innerHTML = `<section class="panel" data-screen="solo-setup">${backButton()}<p class="eyebrow">${t('solo')}</p><h1>${t('choose')}</h1><p class="lede">${t('chooseIntro')}</p>
    <fieldset data-testid="board-size"><legend>${t('boardSize')}</legend><div class="size-options">${[6,9].map(size => `<label class="size-option"><input type="radio" name="size" value="${size}" ${selectedSize === size ? 'checked' : ''}>${art(size)}<strong>${size}×${size}</strong><small>${t(size === 6 ? 'six':'nine')}</small></label>`).join('')}</div></fieldset>
    <fieldset data-testid="difficulty"><legend>${t('difficulty')}</legend><div class="segmented">${LEVELS.map(level => `<label class="segment"><input type="radio" name="difficulty" value="${level}" data-testid="difficulty-${level}" ${selectedDifficulty === level ? 'checked' : ''}>${t(level)}</label>`).join('')}</div>
    </fieldset>
    <button class="primary-button full" data-action="start" data-testid="game-start">${t('start')}</button>
    ${gameNotice ? `<p class="notice error" role="alert">${escape(gameNotice)}</p>` : ''}
  </section>`;
}
function cellLabel(index, board, conflicts) {
  const cell = board.cells[index];
  const notes = Array.from({length:board.size},(_,digit) => digit + 1).filter(digit => cell.notes & (1 << (digit - 1)));
  return t('cell',{row:Math.floor(index / board.size) + 1,column:index % board.size + 1,
    kind:t(board.givens[index] ? 'given':board.locked?.[index] ? 'locked':'editable'),value:cell.value || t('empty'),
    notes:notes.length ? notes.join(', ') : t('noNotes'),conflict:conflicts.has(index) ? t('cellConflict') : ''});
}
function renderBoard(board, conflicts, selected = session.selected, marked = new Set()) {
  const size = board.size;
  const boxRows = size === 6 ? 2 : 3;
  const selectedRow = Math.floor(selected / size);
  const selectedColumn = selected % size;
  const selectedValue = board.cells[selected].value;
  return `<div class="board" role="grid" aria-label="${size}×${size} Sudoku" aria-rowcount="${size}" aria-colcount="${size}" aria-describedby="keyboard-help" data-testid="game-board" style="--size:${size}">${Array.from({length:size},(_,row) => `<div class="board-row" role="row">${Array.from({length:size},(_,column) => {
    const index = row * size + column;
    const cell = board.cells[index];
    const related = row === selectedRow || column === selectedColumn || (Math.floor(row / boxRows) === Math.floor(selectedRow / boxRows) && Math.floor(column / 3) === Math.floor(selectedColumn / 3));
    const classes = ['cell',board.givens[index] ? 'given':'',related ? 'related':'',selectedValue && cell.value === selectedValue ? 'same':'',index === selected ? 'selected':'',conflicts.has(index) ? 'conflicting':'',marked.has(index) ? 'hint-pattern':'',column % 3 === 2 ? 'box-right':'',row % boxRows === boxRows - 1 ? 'box-bottom':'',column === size - 1 ? 'last-column':'',row === size - 1 ? 'last-row':''].filter(Boolean).join(' ');
    const notes = `<span class="cell-notes" aria-hidden="true">${Array.from({length:size},(_,v) => `<span>${cell.notes & (1 << v) ? v + 1 : ''}</span>`).join('')}</span>`;
    return `<button class="${classes}" role="gridcell" aria-rowindex="${row + 1}" aria-colindex="${column + 1}" aria-selected="${index === selected}" aria-readonly="${Boolean(board.givens[index] || board.locked?.[index])}" ${conflicts.has(index) ? 'aria-invalid="true"' : ''} tabindex="${index === selected ? 0 : -1}" data-action="select" data-index="${index}" data-testid="cell-${row + 1}-${column + 1}" aria-label="${escape(cellLabel(index,board,conflicts))}">${cell.value ? `<span aria-hidden="true">${cell.value}</span>` : cell.notes ? notes : ''}${conflicts.has(index) ? '<span class="conflict-mark" aria-hidden="true">!</span>' : ''}</button>`;
  }).join('')}</div>`).join('')}</div>`;
}
// Share of the empty cells that now hold a number (the same measure the server reports for a rival).
function progress(board) {
  let blanks = 0, filled = 0;
  board.cells.forEach((cell,index) => { if (!board.givens[index]) { blanks += 1; if (cell.value) filled += 1; } });
  return blanks ? filled / blanks : 0;
}
// Each key shows how many of that digit are still missing; a finished digit steps back.
function numberPad(board,{action,group,label,disabled = false,notes = false}) {
  const size = board.size;
  const placed = Array(size + 1).fill(0);
  for (const cell of board.cells) placed[cell.value] += 1;
  return `<div class="number-pad" data-size="${size}" data-notes="${notes}" role="group" aria-label="${group}">${Array.from({length:size},(_,index) => {
    const digit = index + 1, left = Math.max(0,size - placed[digit]);
    return `<button class="number-button ${left ? '':'is-done'}" data-action="${action}" data-value="${digit}" data-testid="number-${digit}" aria-label="${label(digit)}" ${disabled ? 'disabled':''}>${digit}${left ? `<small aria-hidden="true">${left}</small>`:''}</button>`;
  }).join('')}</div>`;
}
// One sentence per elimination the placement depends on, then the placement itself.
function hintLines() {
  const size = session.board.size;
  const unit = value => t(value.kind,{number:value.index + 1});
  const steps = (hint.eliminations || []).map(step => step.technique === 'locked_candidates'
    ? t('hintLocked',{unit:unit(step.unit),target:unit(step.target),value:step.digits[0]})
    : t(step.technique === 'naked_pair' ? 'hintNakedPair':'hintHiddenPair',{unit:unit(step.unit),a:step.digits[0],b:step.digits[1]}));
  const naked = hint.technique === 'naked_single';
  const last = t(`${naked ? 'hintNaked':'hintHidden'}${steps.length ? 'Then':''}`,
    {row:Math.floor(hint.index / size) + 1,column:hint.index % size + 1,value:hint.value,unit:hint.unit ? unit(hint.unit) : ''});
  return [...steps,last];
}
function renderHint() {
  if (!hint) return '';
  // Laid over the keys, so the board and the controls keep their place and size while the hint is read.
  return `<aside class="hint-card hint-notice" aria-label="${t('hint')}" data-testid="hint-card"><p>${svg('hint')}<span class="hint-lines">${hintLines().map(line => `<span>${escape(line)}</span>`).join(' ')}</span></p><div class="hint-actions"><button class="secondary-button" data-action="hint-dismiss" data-testid="hint-dismiss">${t('hintDismiss')}</button><button class="primary-button" data-action="hint-apply" data-testid="hint-apply">${t('hintApply')}</button></div></aside>`;
}
function renderGame(focusCell = false) {
  const board = session.board;
  const size = board.size;
  let conflicts = new Set();
  try { if (core.solo_conflicts) conflicts = new Set(core.solo_conflicts(session.game)); } catch { /* Valid live state is retained on a presentation failure. */ }
  const valid = core.validate_board(Uint8Array.from(board.cells.map(cell => cell.value)),size);
  const editable = board.givens[session.selected] === 0;
  const selectedNotes = Array.from({length:size},(_,v)=>v+1).filter(v=>board.cells[session.selected].notes & (1 << (v-1)));
  const tool = (action,icon,label,disabled = false,extra = '') => `<button class="tool" data-action="${action}" data-testid="${action === 'notes' ? 'notes-toggle':action}" ${disabled ? 'disabled' : ''} ${extra}>${svg(icon)}<span>${t(label)}</span></button>`;
  // A failed save stays in the flow until it is fixed; passing notices float over the header instead of pushing the board.
  const messages = storageMessage();
  const toast = gameNotice ? `<div class="game-toast" data-testid="game-notice"><p>${escape(gameNotice)}</p><button class="toast-close" data-action="notice-dismiss" aria-label="${t('hintDismiss')}">${svg('close')}</button></div>` : '';
  const info = debug ? `<p class="selection-info debug-only">${editable ? t('selected',{row:Math.floor(session.selected / size) + 1,column:session.selected % size + 1}) : t('givenSelected')}${selectedNotes.length ? `<br>${t('notesList',{notes:selectedNotes.join(', ')})}`:''}</p>` : '';
  content.innerHTML = `<section class="game" data-screen="solo-game" data-size="${size}">
    <div class="game-top">${backButton('home','home')}<div class="game-heading"><h1>${t('soloShort')} · ${sizeAndLevel(size,session.game)}</h1><span class="timer" role="timer" aria-label="${t('elapsed')}" data-testid="timer">${formatTime(session.time)}</span></div><div class="game-trailing"><button class="icon-button" data-action="pause" data-testid="pause" aria-label="${t('pause')}" title="${t('pause')}">${svg('pause')}</button></div></div>${toast}
    <div class="meter" aria-hidden="true"><i style="--p:${progress(board).toFixed(3)}"></i></div>
    <div class="status-row"><p class="game-status ${valid ? '' : 'conflict'}" data-testid="game-status" role="status">${valid ? `<span class="sr-only">${t('inProgress')}</span>` : `${svg('warning')}<span>${t('conflictShort')}</span>`}</p></div>
    <div class="game-layout"><div class="board-wrap">${renderBoard(board,conflicts,session.selected,new Set((hint?.eliminations || []).flatMap(step => step.cells)))}</div>
    <div class="input-panel"><div class="input-keys">${numberPad(board,{action:'digit',group:t('numbers'),label:digit => t('enterDigit',{digit}),disabled:!editable,notes:session.notesMode})}
      <div class="tools">${tool('notes','notes','notes',false,`role="switch" aria-checked="${session.notesMode}"`)}${tool('erase','erase','erase',!editable)}${tool('undo','undo','undo',board.undo.length === 0)}${tool('redo','redo','redo',board.redo.length === 0)}${tool('hint','hint','hint',!core.solo_hint)}</div>${renderHint()}</div>
      ${messages ? `<div class="game-messages">${messages}</div>` : ''}
      ${info}${session.saveError ? '' : `<p class="storage-note ${debug ? '' : 'sr-only'}" data-testid="storage-status">${t('saved')}</p>`}
      <p class="keyboard-help" id="keyboard-help">${t('keyHelp',{size})}</p>
    </div></div>
  </section>`;
  if (focusCell) content.querySelector(`[data-index="${session.selected}"]`)?.focus({preventScroll:true});
}
function renderPaused() {
  content.innerHTML = `<section class="paused" data-screen="solo-paused"><div class="pause-symbol">${svg('pause')}</div><h1>${t('paused')}</h1><p class="lede">${t('pausedIntro')}</p><p class="stat timer"><small>${sizeAndLevel(session.board.size,session.game)}</small><span class="sr-only"> · </span>${formatTime(session.time)}</p>${storageMessage()}<div class="action-stack"><button class="primary-button" data-action="resume" data-testid="resume">${t('resume')}</button><button class="secondary-button" data-action="home" data-testid="save-exit">${t('saveExit')}</button>${session.saveError ? `<button class="text-button" data-action="discard-exit">${t('discardExit')}</button>`:''}</div></section>`;
}
function renderResult() {
  content.innerHTML = `<section class="result" data-screen="solo-result"><div class="pause-symbol">${svg('check')}</div><p class="eyebrow">${t('completed')}</p><h1 data-testid="result-heading">${t('resultTitle')}</h1><p class="lede">${t('resultIntro')}</p><p class="stat timer" data-testid="result-details"><small>${sizeAndLevel(session.board.size,session.game)}</small><span class="sr-only"> · </span>${formatTime(session.time)}</p>${storageMessage()}<div class="action-stack"><button class="primary-button" data-action="setup">${t('newGame')}</button><button class="secondary-button" data-action="home" data-testid="home">${t('home')}</button>${session.saveError ? `<button class="text-button" data-action="discard-exit">${t('discardExit')}</button>`:''}</div></section>`;
}
function renderSettings() {
  const saves = readSavedGames(storage,core);
  preloadSound(); // so the switch can start audio inside the tap that turns it on
  content.innerHTML = `<section class="panel" data-screen="settings">${backButton('settings-back')}<h1>${t('settings')}</h1>
    <div class="settings-list">
      <label class="settings-row"><span>${t('language')}</span><select id="language-select" name="language" data-testid="language"><option value="tr" ${preferences.language === 'tr' ? 'selected':''}>Türkçe</option><option value="en" ${preferences.language === 'en' ? 'selected':''}>English</option></select></label>
      <label class="settings-row"><span>${t('sound')}<small>${t('soundNote')}</small></span><input id="sound-toggle" type="checkbox" role="switch" data-testid="sound-toggle" ${preferences.sound ? 'checked':''}></label>
      <label class="settings-row"><span>${t('motion')}</span><input id="motion-toggle" type="checkbox" role="switch" data-testid="motion-toggle" ${preferences.reduceMotion ? 'checked':''}></label>
      ${canVibrate ? `<label class="settings-row"><span>${t('haptics')}<small>${t('hapticsNote')}</small></span><input id="haptics-toggle" type="checkbox" role="switch" data-testid="haptics-toggle" ${preferences.haptics ? 'checked':''}></label>`:''}
      <div class="settings-row" role="radiogroup" aria-label="${t('theme')}" data-testid="theme"><span>${t('theme')}</span><div class="segmented">${THEMES.map(theme => `<label class="segment"><input type="radio" name="theme" value="${theme}" data-testid="theme-${theme}" ${preferences.theme === theme ? 'checked':''}>${t('theme' + theme[0].toUpperCase() + theme.slice(1))}</label>`).join('')}</div></div>
    </div>
    <h2 class="section-label">${t('storage')}</h2><p class="note">${t('preferencesNote')}</p><div class="storage-list" data-testid="storage-status">${saves.map(slot => `<p>${slot.size}×${slot.size} · ${slot.status === 'ready' ? `${t(slot.complete ? 'completed':'savedGame')} · ${formatTime(slot.saved.elapsedMs)}` : slot.status === 'missing' ? '—' : escape(t(slot.status === 'corrupt' ? 'corrupt':'storageUnavailable',{size:`${slot.size}×${slot.size}`}))}</p>`).join('')}</div>
    ${settingsNotice ? `<p class="notice" role="status">${escape(settingsNotice)}</p>`:''}
  </section>`;
}
// A layer that arrives over the game (hint card from below, notice from above); nothing under it moves.
function appear(layer,offset) {
  if (!layer?.animate) return;
  const still = preferences.reduceMotion || matchMedia('(prefers-reduced-motion: reduce)').matches;
  layer.animate([{opacity:0,transform:still ? 'none':`translateY(${offset}px)`},{opacity:1,transform:'none'}],{duration:still ? 120 : 200,easing:'cubic-bezier(.23,1,.32,1)'});
}
// A new screen eases in once; re-renders of the same screen (every move, every poll) never animate.
function enter(section) {
  if (!section?.animate) return;
  // The board is a working surface: a game screen only fades, as does everything under reduced motion.
  if (section.classList.contains('game') || preferences.reduceMotion || matchMedia('(prefers-reduced-motion: reduce)').matches) {
    section.animate([{opacity:0},{opacity:1}],{duration:160,easing:'ease-out'});
    return;
  }
  const easing = 'cubic-bezier(.23,1,.32,1)';
  const delays = [0,40,75,105,130,150]; // gaps shrink; the last items arrive together
  [...section.children].forEach((child,index) => child.animate([{opacity:0,transform:`translateY(${index ? 6 : 10}px)`},{opacity:1,transform:'none'}],
    {duration:260,delay:delays[Math.min(index,delays.length - 1)],easing,fill:'backwards'}));
  if (section.classList.contains('result')) section.querySelector('.pause-symbol svg')?.animate([{opacity:0,transform:'scale(.9)'},{opacity:1,transform:'scale(1)'}],{duration:300,delay:120,easing,fill:'backwards'});
}
// Match events the player did not tap for: start, a friend arriving, a wrong duel move, the result.
function liveCues() {
  const view = live?.view();
  const s = view?.snapshot;
  const receipt = view?.last_receipt;
  const now = s && !view.voluntarily_left ? {phase:s.phase,opponent:Boolean(s.opponent),receipt:receipt ? `${receipt.match_id}:${receipt.sequence}:${receipt.code}`:''} : null;
  const before = liveSeen;
  liveSeen = now;
  if (!now || !before) return; // the first look at a session (page load, room just created) stays silent
  if (now.phase === 'playing' && before.phase !== 'playing') cue('attention');
  else if (now.phase === 'finished' && before.phase !== 'finished') {
    if (s.result && s.result.winner === s.own.seat) cue('success',{emphasis:'strong'}); else cue('close');
  } else if (now.phase === 'lobby' && now.opponent && !before.opponent) cue('ready');
  if (now.receipt !== before.receipt && receipt?.code === 'wrong') cue('error');
}
function render(focusCell = false) {
  const previous = content.firstElementChild?.dataset.screen;
  applyPreferences();
  settingsButton.hidden = ['solo-game','solo-paused','live'].includes(screen);
  if (screen === 'home') renderHome();
  else if (screen === 'solo-setup') renderSetup();
  else if (screen === 'solo-game') renderGame(focusCell);
  else if (screen === 'solo-paused') renderPaused();
  else if (screen === 'solo-result') renderResult();
  else if (screen === 'settings') renderSettings();
  else if (screen === 'live') { live.render(); liveCues(); }
  const section = content.firstElementChild;
  const current = section?.dataset.screen || '';
  document.body.dataset.view = content.querySelector('[data-screen$="-game"]') ? 'game' : 'page';
  document.body.dataset.screen = current;
  if (current !== previous) enter(section);
}
function applyCommand(command) {
  hint = null;
  gameNotice = '';
  try {
    session.apply(command);
    if (session.complete) {
      screen = 'solo-result';
      render();
      focusContent();
      announce(t('resultTitle'));
      cue('success',{emphasis:'strong'});
    } else {
      renderGame(true);
      if (command.type === 'toggle_note') cue('type');
      else if (command.type === 'set') { if (content.querySelector(`[data-index="${command.index}"]`)?.classList.contains('conflicting')) cue('error',{emphasis:'subtle'}); else cue('tap'); }
      else cue('tap',{emphasis:'subtle'});
    }
  } catch (error) {
    gameNotice = String(error).includes('GivenCell') ? t('badCell') : String(error).includes('FilledCell') ? t('filledNotes') : t('genericError');
    renderGame(true);
    appear(content.querySelector('.game-toast'),-8);
    announce(gameNotice);
    cue('error',{emphasis:'subtle'});
  }
}
function enterDigit(value, keyboard = false) {
  if (session.board.givens[session.selected]) return;
  applyCommand({type:session.notesMode ? 'toggle_note':'set',index:session.selected,value},keyboard);
}
function goHome() {
  if (session.game && !session.pause()) {
    navigate(session.complete ? 'solo-result':'solo-paused');
    return;
  }
  homeNotice = '';
  cue('navigate',{direction:'back',emphasis:'subtle'});
  navigate('home');
}
// In-app confirmation sheet (native <dialog>: focus stays inside, Escape and the backdrop cancel).
function confirmDialog(message,{ok,cancel = t('cancel')} = {}) {
  const dialog = document.querySelector('#confirm');
  if (!dialog?.showModal) return Promise.resolve(window.confirm(message));
  if (dialog.open) return Promise.resolve(false);
  dialog.querySelector('#confirm-text').textContent = message;
  dialog.querySelector('[value="cancel"]').textContent = cancel;
  dialog.querySelector('[value="ok"]').textContent = ok;
  dialog.returnValue = '';
  dialog.showModal();
  cue('open',{emphasis:'subtle'});
  return new Promise(resolve => dialog.addEventListener('close',() => {
    if (document.activeElement === document.body) content.focus({preventScroll:true});
    resolve(dialog.returnValue === 'ok');
  },{once:true}));
}
document.querySelector('#confirm')?.addEventListener('click',event => { if (event.target === event.currentTarget) event.currentTarget.close('cancel'); });
async function discardOpenGame() {
  if (!await confirmDialog(t('confirmDiscard'),{ok:t('discardExit')})) return false;
  session.game = null;
  session.startedAt = null;
  session.saveError = false;
  return true;
}
async function handleAction(button) {
  const action = button.dataset.action;
  if (action === 'home') return goHome();
  if (action === 'setup') {
    if (session.game && session.saveError && !session.save() && !await discardOpenGame()) return;
    cue('navigate',{emphasis:'subtle'});
    return navigate('solo-setup');
  }
  if (action === 'discard-exit') { if (await discardOpenGame()) navigate('home'); return; }
  if (action === 'friends' || action === 'live-resume') { cue('navigate',{emphasis:'subtle'});screen='live';live.enter();focusContent();return; }
  if (action === 'refresh-saves') return render();
  if (action === 'settings-back') { cue('navigate',{direction:'back',emphasis:'subtle'}); return navigate(settingsReturn); }
  if (action === 'start') {
    const slot = readSavedGames(storage,core).find(slot => slot.size === selectedSize);
    const exists = slot.status !== 'missing';
    if (exists && !await confirmDialog(t(slot.status === 'corrupt' ? 'replaceCorrupt':'replace'),{ok:t('start')})) return;
    // Digging a rated puzzle is synchronous and can take a moment on a phone: say so first.
    button.disabled = true;
    button.textContent = t('preparing');
    await new Promise(resolve => requestAnimationFrame(() => setTimeout(resolve,0)));
    if (screen !== 'solo-setup') return;
    for (let attempt = 0; attempt < 3; attempt += 1) {
      try {
        session.start(selectedSize,crypto.getRandomValues(new Uint32Array(1))[0],exists,selectedDifficulty);
        cue('open');
        return navigate('solo-game');
      } catch (error) { if (!String(error).includes('CannotReachDifficulty')) break; }
    }
    gameNotice = t('genericError');
    renderSetup();
    return;
  }
  if (action === 'restore') {
    try {
      session.restore(Number(button.dataset.size));
      cue('open');
      if (session.complete) navigate('solo-result');
      else { session.resume(); navigate('solo-game'); }
    } catch { homeNotice = t('corrupt',{size:`${button.dataset.size}×${button.dataset.size}`}); renderHome(); }
    return;
  }
  if (action === 'pause') { session.pause(); cue('close',{emphasis:'subtle'}); return navigate('solo-paused'); }
  if (action === 'resume') { session.resume(); cue('open',{emphasis:'subtle'}); return navigate('solo-game'); }
  if (action === 'save') { session.save(); render(); return; }
  if (screen !== 'solo-game') return;
  if (action === 'select') {
    session.selected = Number(button.dataset.index);
    hint = null;
    gameNotice = '';
    renderGame(true);
  } else if (action === 'digit') enterDigit(Number(button.dataset.value));
  else if (action === 'notes') {
    session.notesMode = !session.notesMode;
    session.save();
    gameNotice = '';
    renderGame();
    cue('toggle');
    content.querySelector('[data-action="notes"]')?.focus({preventScroll:true});
  } else if (['undo','redo'].includes(action)) applyCommand({type:action});
  else if (action === 'erase') applyCommand({type:'erase',index:session.selected});
  else if (action === 'hint') {
    try {
      hint = JSON.parse((core.solo_technique_hint || core.solo_hint)(session.game));
      gameNotice = hint ? '' : t('hintNone');
      if (hint) session.selected = hint.index;
    } catch {
      gameNotice = t(core.validate_board(Uint8Array.from(session.board.cells.map(cell => cell.value)),session.board.size) ? 'hintUnavailable':'hintConflict');
    }
    renderGame();
    const layer = content.querySelector(hint ? '.hint-card':'.game-toast');
    if (hint) {
      cue('ready');
      // Focus lands on the hint's main action; the explanation is read from the live region.
      layer?.querySelector('[data-action="hint-apply"]')?.focus({preventScroll:true});
    }
    appear(layer,hint ? 8 : -8);
    announce(gameNotice || (hint ? hintLines().join(' ') : ''));
  } else if (action === 'hint-apply' && hint) {
    const {index,value} = hint;
    applyCommand({type:'set',index,value});
  } else if (action === 'hint-dismiss' || action === 'notice-dismiss') { hint = null; gameNotice = ''; renderGame(true); }
}

content.addEventListener('click',event => {
  const button = event.target.closest('[data-action]');
  if (!ready || !button || button.disabled) return;
  if (screen==='live') live.handle(button.dataset.action,button);
  else handleAction(button);
});
content.addEventListener('input',event => { if(screen==='live' && event.target.dataset.liveField) live.input(event.target.dataset.liveField,event.target.value,event.target); });
content.addEventListener('change',event => {
  if(screen==='live' && event.target.dataset.liveField) {live.input(event.target.dataset.liveField,event.target.value,event.target,true);return;}
  if (event.target.name === 'size') { selectedSize = preferences.soloSize = Number(event.target.value); savePreferences(); cue('select'); }
  if (event.target.name === 'difficulty' && LEVELS.includes(event.target.value)) {
    selectedDifficulty = preferences.soloDifficulty = event.target.value;
    savePreferences();
    cue('select');
    renderSetup();
    content.querySelector(`input[name="difficulty"][value="${selectedDifficulty}"]`)?.focus({preventScroll:true});
  }
  if (['language-select','motion-toggle','sound-toggle','haptics-toggle'].includes(event.target.id) || event.target.name === 'theme') {
    if (event.target.id === 'language-select') { preferences.language = event.target.value; cue('select'); }
    else if (event.target.name === 'theme') { if (THEMES.includes(event.target.value)) preferences.theme = event.target.value; cue('select'); }
    else if (event.target.id === 'haptics-toggle') { preferences.haptics = event.target.checked; cue('toggle'); }
    else if (event.target.id === 'sound-toggle') { preferences.sound = event.target.checked; setSound(preferences.sound,{confirm:true}); }
    else { preferences.reduceMotion = event.target.checked; cue('toggle'); }
    settingsNotice = t(savePreferences() ? 'settingsSaved':'settingsFailed');
    renderSettings();
    applyPreferences();
    (event.target.name === 'theme' ? content.querySelector(`input[name="theme"][value="${preferences.theme}"]`) : content.querySelector(`#${event.target.id}`))?.focus();
  }
});
content.addEventListener('keydown',event => {
  if(screen==='live') {live.keydown(event);return;}
  if (screen === 'solo-game' && event.key === 'Escape' && (hint || gameNotice)) { event.preventDefault(); hint = null; gameNotice = ''; renderGame(true); return; }
  if (screen !== 'solo-game' || !event.target.closest('[role="grid"]')) return;
  const size = session.board.size;
  let index = session.selected;
  if (event.key === 'ArrowLeft') index = index % size ? index - 1 : index;
  else if (event.key === 'ArrowRight') index = index % size < size - 1 ? index + 1 : index;
  else if (event.key === 'ArrowUp') index = index >= size ? index - size : index;
  else if (event.key === 'ArrowDown') index = index < size * (size - 1) ? index + size : index;
  else if (/^[1-9]$/.test(event.key) && Number(event.key) <= size && !event.ctrlKey && !event.metaKey && !event.altKey) {
    event.preventDefault(); if (!event.repeat) enterDigit(Number(event.key),true); return;
  } else if (['Backspace','Delete'].includes(event.key)) {
    event.preventDefault(); if (!event.repeat && !session.board.givens[index]) applyCommand({type:'erase',index},true); return;
  } else if (event.key.toLowerCase() === 'n' && !event.ctrlKey && !event.metaKey && !event.altKey) {
    event.preventDefault(); if (!event.repeat) { session.notesMode = !session.notesMode; session.save(); renderGame(true); cue('toggle'); } return;
  } else if (event.key.toLowerCase() === 'z' && (event.ctrlKey || event.metaKey)) {
    event.preventDefault(); if (!event.repeat) applyCommand({type:event.shiftKey ? 'redo':'undo'},true); return;
  } else return;
  event.preventDefault();
  session.selected = index;
  hint = null;
  gameNotice = '';
  renderGame(true);
});
settingsButton.addEventListener('click',() => {
  if (!ready) return;
  if (session.running) { session.pause(); settingsReturn = 'solo-paused'; }
  else settingsReturn = screen === 'solo-result' ? 'solo-result':'home';
  settingsNotice = '';
  cue('navigate',{emphasis:'subtle'});
  navigate('settings');
});
document.querySelector('#brand-home').addEventListener('click',event => { event.preventDefault(); if (ready) {if(screen==='live')live.handle('live-home',event.target);else goHome();} });
document.addEventListener('visibilitychange',() => {
  if (document.hidden && ready && screen === 'solo-game') { session.pause(); navigate('solo-paused'); }
});
window.addEventListener('pagehide',() => { if (session?.game) session.pause();live?.persist(); });
// Best effort only: a not-yet-due resume is retried by tick(), so it must not show an error.
window.addEventListener('online',()=>{ if(screen==='live' && live?.view()?.snapshot && !live.view().voluntarily_left && !live.view().pending_http && live.view().connection!=='session_lost')live.transport?.dispatch({type:'resume'},{quiet:true}); });
setInterval(()=>live?.tick(),250);
setInterval(() => {
  if (screen === 'solo-game' && session?.running) {
    const timer = content.querySelector('[data-testid="timer"]');
    if (timer) timer.textContent = formatTime(session.time);
  }
},1000);
setInterval(() => { if (session?.running) session.save(); },10000);

applyPreferences();
try {
  await init();
  session = new SoloSession(core,storage);
  live = new LiveScreens({debug,engine:core,storage,content,renderBoard,numberPad,progress,cue,confirm:confirmDialog,svg,escape,
    remembered:{name:preferences.displayName,mode:preferences.liveMode,size:preferences.liveSize,difficulty:preferences.liveDifficulty},
    remember:(field,value) => { preferences[{name:'displayName',mode:'liveMode',size:'liveSize',difficulty:'liveDifficulty'}[field]] = value; savePreferences(); },language:()=>preferences.language,announce,
    onUpdate:()=>{if(screen==='live')render();},onHome:()=>{if(screen==='live'){screen='home';history.replaceState({},'', '/');render();focusContent();}}});
  ready = true;
  if(location.pathname.startsWith('/r/')) {screen='live';live.enter(location.pathname);}
  else render();
} catch {
  content.innerHTML = `<section class="panel"><h1>Sudoku Rivals</h1><p class="notice error" role="alert">${t('loadFailed')}</p><button class="primary-button" id="reload">${t('loadAgain')}</button></section>`;
  document.querySelector('#reload').addEventListener('click',() => location.reload());
}
if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('/sw.js').then(() => navigator.serviceWorker.ready).then(() => {
    setOffline('offlineReady');
  }).catch(() => setOffline('offlineUnavailable'));
} else setOffline('offlineUnavailable');
