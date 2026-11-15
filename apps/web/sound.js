// Opt-in interaction sounds (Cuelume, synthesized with Web Audio; no audio files).
// Silent by default: the library is fetched only once sound is switched on, and
// nothing plays before the person has touched or typed on the page.
let library = null;
let loading = null;
let enabled = false;
let interacted = false;

if (typeof window !== 'undefined') {
  const mark = event => { if (event.isTrusted) interacted = true; };
  window.addEventListener('pointerdown',mark,{capture:true,passive:true});
  window.addEventListener('keydown',mark,{capture:true,passive:true});
}

export function preloadSound() {
  loading ??= import('/cuelume.js').then(module => { library = module; library.setEnabled(enabled); return module; })
    .catch(() => { loading = null; return null; });
  return loading;
}
export function soundEnabled() { return enabled; }
// confirm: the switch itself was just flipped by the person, so start audio in that gesture and let them hear it.
export function setSound(on,{confirm = false} = {}) {
  enabled = on;
  library?.setEnabled(on);
  if (!on) return;
  const start = module => { if (module && enabled && confirm && interacted) { module.unlock(); module.play('toggle'); } };
  if (library) start(library);
  else preloadSound().then(start);
}
// Cue names: tap, type, select, toggle, open, close, success, error, navigate, warning, loading, ready, attention, count.
export function cue(name,options) {
  if (enabled && interacted && library) library.play(name,options);
}
