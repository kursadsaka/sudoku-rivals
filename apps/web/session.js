export const SAVE_VERSION = 1;
export const saveKey = size => `sudoku-rivals.solo.v1.${size}`;

function validateEnvelope(raw, size, engine) {
  const value = JSON.parse(raw);
  if (!value || value.version !== SAVE_VERSION || typeof value.game !== 'string'
      || !Number.isSafeInteger(value.elapsedMs) || value.elapsedMs < 0
      || !Number.isInteger(value.selected) || value.selected < 0 || value.selected >= size * size
      || typeof value.notesMode !== 'boolean') throw new Error('InvalidSave');
  engine.solo_is_complete(value.game); // All board/history invariants stay in Rust.
  if (JSON.parse(value.game).size !== size) throw new Error('InvalidSave');
  return value;
}

export function readSavedGames(storage, engine) {
  return [6, 9].map(size => {
    try {
      const raw = storage.getItem(saveKey(size));
      if (raw === null) return { size, status: 'missing' };
      try {
        const saved = validateEnvelope(raw, size, engine);
        return { size, status: 'ready', saved, complete: engine.solo_is_complete(saved.game) };
      } catch { return { size, status: 'corrupt' }; }
    } catch { return { size, status: 'unavailable' }; }
  });
}

export class SoloSession {
  constructor(engine, storage, now = () => performance.now()) {
    this.engine = engine;
    this.storage = storage;
    this.now = now;
    this.game = null;
    this.elapsedMs = 0;
    this.startedAt = null;
    this.selected = 0;
    this.notesMode = false;
    this.saveError = false;
  }

  get board() { return this.game ? JSON.parse(this.game) : null; }
  get complete() { return this.game !== null && this.engine.solo_is_complete(this.game); }
  get running() { return this.startedAt !== null; }
  get time() {
    return this.elapsedMs + (this.running ? Math.max(0, Math.floor(this.now() - this.startedAt)) : 0);
  }

  // difficulty: 'easy' | 'medium' | 'hard' — the engine digs a puzzle that needs exactly that level's techniques.
  start(size, seed, replaceConfirmed = false, difficulty = 'easy') {
    if (![6, 9].includes(size)) throw new Error('InvalidSize');
    let existing;
    try { existing = this.storage.getItem(saveKey(size)); }
    catch { existing = 'unavailable'; }
    if (existing !== null && !replaceConfirmed) throw new Error('ReplaceConfirmationRequired');
    const game = this.engine.new_rated_solo(size, seed, difficulty);
    this.game = game;
    this.elapsedMs = 0;
    this.startedAt = this.now();
    this.notesMode = false;
    this.selected = Math.max(0, this.board.givens.indexOf(0));
    this.save();
  }

  restore(size) {
    const raw = this.storage.getItem(saveKey(size));
    const saved = validateEnvelope(raw, size, this.engine);
    // Replace the open game only after every persisted field has been checked.
    this.game = saved.game;
    this.elapsedMs = saved.elapsedMs;
    this.selected = saved.selected;
    this.notesMode = saved.notesMode;
    this.startedAt = null;
    this.saveError = false;
  }

  resume() {
    if (this.game && !this.complete && !this.running) this.startedAt = this.now();
  }

  pause() {
    if (this.running) {
      this.elapsedMs = this.time;
      this.startedAt = null;
    }
    return this.save();
  }

  apply(command) {
    if (!this.game || !this.running || this.complete) return false;
    const next = this.engine.update_solo(this.game, JSON.stringify(command));
    const changed = next !== this.game;
    this.game = next;
    if (this.complete) this.pause();
    else if (changed) this.save();
    return changed;
  }

  save() {
    if (!this.game) return true;
    try {
      const elapsedMs = Math.min(Number.MAX_SAFE_INTEGER, this.time);
      this.storage.setItem(saveKey(this.board.size), JSON.stringify({
        version: SAVE_VERSION, game: this.game, elapsedMs,
        selected: this.selected, notesMode: this.notesMode,
      }));
      this.saveError = false;
      return true;
    } catch {
      // localStorage.setItem is atomic: a failed write leaves the previous save intact.
      this.saveError = true;
      return false;
    }
  }
}

export function formatTime(milliseconds) {
  const seconds = Math.floor(milliseconds / 1000);
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor(seconds / 60) % 60;
  const remainder = seconds % 60;
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, '0')}:${String(remainder).padStart(2, '0')}`
    : `${minutes}:${String(remainder).padStart(2, '0')}`;
}
