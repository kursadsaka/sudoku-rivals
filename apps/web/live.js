export const LIVE_KEY = 'sudoku-rivals.live.v1';

// This adapter handles transport and storage only. The shared Rust client owns
// admission, generations, replay, reconciliation and all game state.
export class MultiplayerTransport {
  constructor(engine, storage, { now = () => Math.floor(performance.now()), fetcher = (...args) => fetch(...args),
    origin = location.origin, onChange = () => {}, onError = () => {}, onLeft = () => {},
    timeoutMs = 10000 } = {}) {
    this.engine = engine;
    this.storage = storage;
    this.now = now;
    this.fetcher = fetcher;
    this.origin = origin;
    this.onChange = onChange;
    this.onError = onError;
    this.onLeft = onLeft;
    this.timeoutMs = timeoutMs;
    this.active = new Map();
    this.currentId = null;
    this.storageError = false;
    this.protectedSave = false;
    this.restoreStatus = 'empty';
    this.transportError = null;
    this.stopped = false;
    this.epoch = 0;
    try {
      const raw = storage.getItem(LIVE_KEY);
      if (raw !== null) {
        try { this.client = engine.MultiplayerClient.restore(raw); this.restoreStatus = 'restored'; }
        catch { this.protectedSave = true; this.restoreStatus = 'corrupt'; }
      }
    } catch { this.storageError = true; this.protectedSave = true; this.restoreStatus = 'unavailable'; }
    this.client ??= new engine.MultiplayerClient(crypto.getRandomValues(new Uint32Array(1))[0]);
  }

  view() { return JSON.parse(this.client.view(this.now())); }

  persist() {
    if (this.protectedSave) return false;
    try { this.storage.setItem(LIVE_KEY,this.client.save()); this.storageError = false; return true; }
    catch { this.storageError = true; return false; }
  }

  replaceSession() {
    // Call only after the user has explicitly approved replacing an unusable save.
    this.epoch += 1;
    for (const attempt of this.active.values()) attempt.controller.abort();
    this.active.clear();
    this.client.free?.();
    this.client = new this.engine.MultiplayerClient(crypto.getRandomValues(new Uint32Array(1))[0]);
    this.currentId = null;
    this.protectedSave = false;
    this.restoreStatus = 'empty';
    this.transportError = null;
    this.persist();
    this.onChange(this.view());
  }

  dispatch(action, { quiet = false } = {}) {
    if (this.stopped) return null;
    try {
      if(['create','join'].includes(action.type) && !action.guest) {
        action={...action,guest:Array.from(crypto.getRandomValues(new Uint8Array(32)),byte=>byte.toString(16).padStart(2,'0')).join('')};
      }
      const effect = JSON.parse(this.client.dispatch(JSON.stringify(action),this.now()));
      if (effect.changed) this.persist();
      if (effect.request) this.send(effect.request);
      if (effect.changed) this.onChange(this.view());
      return effect;
    } catch (error) {
      if (!quiet) this.onError(String(error));
      return null;
    }
  }

  send(request) {
    if (this.active.has(request.id) || this.stopped) return;
    // Never send a credential-bearing descriptor to another origin or a URL query.
    if (request.method !== 'POST' || typeof request.body !== 'string'
        || !request.path.startsWith('/v1/') || request.path.includes('?') || request.path.includes('#')) {
      this.onError('InvalidRequest');
      return;
    }
    const url = new URL(request.path,this.origin);
    if (url.origin !== new URL(this.origin).origin) { this.onError('InvalidRequest'); return; }
    const controller = new AbortController();
    const epoch = this.epoch;
    const attempt = { controller };
    this.currentId = request.id;
    for (const [id,old] of this.active) if(id!==request.id) old.controller.abort();
    this.active.set(request.id,attempt);
    this.currentId = request.id;
    const timer = setTimeout(() => controller.abort(),this.timeoutMs);
    Promise.resolve().then(() => this.fetcher(url.href,{
      method:'POST',headers:{'Content-Type':'application/json'},body:request.body,
      signal:controller.signal,cache:'no-store',credentials:'same-origin',
    })).then(async response => {
      const body = await response.text();
      if (this.stopped || this.epoch !== epoch) return;
      const effect = this.dispatch({type:'receive',request_id:request.id,status:response.status,response:body});
      if (!effect) {
        if (this.currentId === request.id) {
          this.transportError = 'invalid_response';
          this.dispatch({type:'network_failed',request_id:request.id},{quiet:true});
        }
        return;
      }
      if (effect.changed) {
        this.transportError = response.ok ? null : safeError(body);
        this.onChange(this.view());
        if (response.ok && request.path.endsWith('/leave') && this.view().connection === 'session_lost') this.onLeft();
      }
    }).catch(() => {
      // A superseded request's failure must not disconnect the new generation.
      if (this.stopped || this.epoch !== epoch || this.currentId !== request.id || this.active.get(request.id) !== attempt) return;
      this.transportError = 'network';
      this.dispatch({type:'network_failed',request_id:request.id},{quiet:true});
      this.onChange(this.view());
    }).finally(() => {
      clearTimeout(timer);
      if (this.active.get(request.id) === attempt) this.active.delete(request.id);
    });
  }

  tick() {
    if (this.stopped || this.protectedSave) return;
    const view = this.view();
    const now = this.now();
    if (view.voluntarily_left && !view.pending_http) return;
    if (view.retry_at_ms !== null && now >= view.retry_at_ms && this.active.size === 0) {
      const effect = this.dispatch({type:'retry'},{quiet:true});
      if (!effect && view.snapshot) this.dispatch({type:'resume'},{quiet:true});
      return;
    }
    if (view.pending_http || view.connection !== 'connected') return;
    if (view.race?.dirty && view.snapshot?.phase === 'playing' && !view.pending_command && !view.reconcile_required) {
      this.dispatch({type:'sync'},{quiet:true});
    } else if (view.poll_due) this.dispatch({type:'poll'},{quiet:true});
  }

  stop() {
    if(this.stopped)return;
    this.stopped = true;
    this.persist();
    for (const attempt of this.active.values()) attempt.controller.abort();
    this.active.clear();
  }
}

function safeError(body) {
  try {
    const {error} = JSON.parse(body);
    return ['unauthorized','room_missing','room_closed','room_full','rate_limited','capacity',
      'invalid_input','stale_generation','stale_match','sequence_gap','sequence_conflict','internal'].includes(error) ? error : 'server';
  } catch { return 'invalid_response'; }
}

const words = {
  tr: {
    title:'Arkadaşınla yarış', setup:'Birlikte bir tahta açın.', intro:'Bir oda kur veya arkadaşının davetine katıl.', name:'Görünen adın', mode:'Oyun modu', race:'Klasik yarış', duel:'90 saniye düello', size:'Tahta boyutu', create:'Oda kur', join:'Katıl', invite:'Oda kodu veya davet bağlantısı', or:'Davetin mi var?', back:'Geri', home:'Ana sayfa', waiting:'Arkadaşın bekleniyor', room:'Oda kodu', share:'Daveti paylaş', copied:'Davet bağlantısı kopyalandı.', copyFailed:'Bağlantıyı aşağıdan kopyalayabilirsin.', ready:'Hazırım', unready:'Hazır değilim', readyLabel:'Hazır', notReady:'Bekliyor', you:'Sen', opponent:'Rakip', noOpponent:'Henüz katılmadı', startIn:'Maç {seconds} saniye sonra başlayacak', starting:'Sunucunun başlangıç onayı bekleniyor', connected:'Bağlandı', disconnected:'Bağlantı kesildi. Yeniden deneniyor.', reconnecting:'Yeniden bağlanılıyor', lost:'Maç sürdürülemiyor. Sunucu oturumu veya sonucu kurtarılamadı.', savedLocal:'Yerel yarış tahtan korunuyor.', resume:'Maçına dön', reconnect:'Yeniden bağlan', pending:'Sunucu onayı bekleniyor', retry:'Tekrar dene', elapsed:'Geçen süre', remaining:'Kalan süre', score:'Puanın', opponentScore:'Rakip', combo:'Seri', progress:'Rakibin doldurduğu hücreler', inputLock:'Giriş kilidi: {seconds} saniye', notes:'Kalem', erase:'Sil', undo:'Geri al', redo:'Yinele', numbers:'Bir sayı seç', selected:'{row}. satır · {column}. sütun', given:'Verilen veya kilitli hücre', notesList:'Notlar: {notes}', resign:'Ayrıl', confirmLeave:'Bu maçtan ayrılmak istiyor musun? Oyun başladıysa rakibin kazanır.', leaving:'Ayrılma onayı bekleniyor; yerel oturumun korunuyor.', saved:'Oturum bu cihazda kaydedildi', saveError:'Oturum kaydedilemedi. Açık maç korunuyor; tarayıcıyı kapatırsan yeniden bağlanamayabilirsin.', corrupt:'Canlı maç kaydı açılamadı. Mevcut kayıt değiştirilmedi.', recover:'Yeni oturum aç', confirmRecover:'Saklanan canlı maç kaydı değiştirilecek. Henüz gönderilmemiş yerel değişiklikler kaybolabilir. Devam edilsin mi?', reconcile:'Yerel tahta ile sunucu görüntüsü farklı. Değişikliklerin henüz silinmedi.', keep:'Yerel tahtayı koru', accept:'Sunucu tahtasını kullan', confirmAccept:'Sunucu tahtası yerel değişikliklerinin yerini alacak. Devam edilsin mi?', conflict:'Çakışan hücreleri kontrol et.', playing:'Maç devam ediyor', notCorrect:'Tahta henüz doğru değil.', wrong:'Yanlış hamle. Seri sıfırlandı.', rateLimited:'Çok hızlı giriş. Kısa bir süre bekle.', win:'Maçı kazandın', loss:'Rakibin kazandı', draw:'Berabere', noContest:'Maç sonuçsuz bitti', resultPending:'Kesin sonuç sunucudan bekleniyor.', rematch:'Rövanş iste', rematchWaiting:'Rövanş için arkadaşın bekleniyor', rematchOpponent:'Arkadaşın rövanş istiyor', opponentLeft:'Arkadaşın odadan ayrıldı; rövanş istenemez.', reason_completed:'İlk doğru bitiş sunucuda doğrulandı.', reason_time:'90 saniye doldu.', reason_resigned:'Oyunculardan biri ayrıldı.', reason_disconnect_timeout:'Yeniden bağlanma süresi doldu.', reason_both_disconnected:'İki bağlantının süresi birlikte doldu.', reason_resource_limit:'Sunucu bu maçı güvenle sürdüremedi.', network:'Sunucuya ulaşılamıyor.', room_full:'Oda dolu.', room_missing:'Oda bulunamadı. Sunucu yeniden başlamış veya oda kapanmış olabilir.', room_closed:'Oda kapandı.', unauthorized:'Bu oturumun yetkisi artık geçerli değil.', capacity:'Sunucu şu anda dolu.', rate_limited:'Çok fazla istek gönderildi. Biraz sonra tekrar dene.', invalid_input:'Adı, oda kodunu ve seçimlerini kontrol et.', invalid_response:'Sunucu yanıtı doğrulanamadı; maç sonucu değiştirilmedi.', server:'Sunucu isteği tamamlanamadı.', internal:'Sunucu isteği tamamlanamadı.', busy:'Önceki isteğin yanıtı bekleniyor.', invalid:'Bu işlem şu anda uygulanamıyor.', apiUnavailable:'Canlı maç istemcisi yüklenemedi. Solo oynamaya devam edebilirsin.', modeHelp:'Yarışta ilk doğru bitiren kazanır. Düelloda 90 saniye boyunca doğru hamlelerle puan kazanırsın.', keyboard:'Oklar: hücre seç · 1–{size}: sayı', noNotesDuel:'Düelloda not, silme ve geri alma yoktur.', continueSaved:'Devam eden maçın var.', dirty:'Yerel değişiklikler sunucuya gönderilecek.',
  },
  en: {
    title:'Play with a friend', setup:'Open a board together.', intro:'Create a room or join your friend’s invite.', name:'Your display name', mode:'Game mode', race:'Classic race', duel:'90-second duel', size:'Board size', create:'Create room', join:'Join', invite:'Room code or invite link', or:'Have an invite?', back:'Back', home:'Home', waiting:'Waiting for your friend', room:'Room code', share:'Share invite', copied:'Invite link copied.', copyFailed:'You can copy the link below.', ready:'Ready', unready:'Not ready', readyLabel:'Ready', notReady:'Waiting', you:'You', opponent:'Opponent', noOpponent:'Not joined yet', startIn:'Match starts in {seconds} seconds', starting:'Waiting for the server to confirm the start', connected:'Connected', disconnected:'Disconnected. Retrying.', reconnecting:'Reconnecting', lost:'The match cannot continue. The server session or result could not be recovered.', savedLocal:'Your local race board is preserved.', resume:'Return to your match', reconnect:'Reconnect', pending:'Waiting for server confirmation', retry:'Retry', elapsed:'Elapsed', remaining:'Remaining', score:'Your score', opponentScore:'Opponent', combo:'Streak', progress:'Opponent filled', inputLock:'Input locked: {seconds} seconds', notes:'Notes', erase:'Erase', undo:'Undo', redo:'Redo', numbers:'Choose a number', selected:'Row {row} · Column {column}', given:'Given or locked cell', notesList:'Notes: {notes}', resign:'Leave', confirmLeave:'Leave this match? If play has started, your opponent wins.', leaving:'Waiting for exit confirmation; your local session is preserved.', saved:'Session saved on this device', saveError:'Could not save the session. Your open match is preserved, but closing the browser may prevent reconnecting.', corrupt:'The live match save could not be opened. It has not been changed.', recover:'Start a new session', confirmRecover:'The saved live match session will be replaced. Unsent local changes may be lost. Continue?', reconcile:'Your local board differs from the server. Your changes have not been deleted.', keep:'Keep local board', accept:'Use server board', confirmAccept:'The server board will replace your local changes. Continue?', conflict:'Check the conflicting cells.', playing:'Match in progress', notCorrect:'The board is not correct yet.', wrong:'Wrong move. Your streak was reset.', rateLimited:'Input is too fast. Wait a moment.', win:'You won', loss:'Your opponent won', draw:'Draw', noContest:'Match ended without a result', resultPending:'Waiting for the server’s final result.', rematch:'Request rematch', rematchWaiting:'Waiting for your friend to accept the rematch', rematchOpponent:'Your friend wants a rematch', opponentLeft:'Your friend left the room; a rematch is not possible.', reason_completed:'The first correct finish was verified by the server.', reason_time:'The 90 seconds ran out.', reason_resigned:'A player left.', reason_disconnect_timeout:'The reconnect window ran out.', reason_both_disconnected:'Both reconnect windows ran out together.', reason_resource_limit:'The server could not safely continue this match.', network:'Cannot reach the server.', room_full:'The room is full.', room_missing:'Room not found. The server may have restarted or the room may have closed.', room_closed:'The room has closed.', unauthorized:'This session is no longer authorized.', capacity:'The server is full right now.', rate_limited:'Too many requests. Try again later.', invalid_input:'Check your name, room code and selections.', invalid_response:'The server response could not be verified; the match result was not changed.', server:'The server request could not complete.', internal:'The server request could not complete.', busy:'Waiting for the previous request.', invalid:'This action is not available right now.', apiUnavailable:'The live match client could not load. You can still play solo.', modeHelp:'In a race, the first correct finish wins. In a duel, score points with correct moves for 90 seconds.', keyboard:'Arrows: select a cell · 1–{size}: enter a number', noNotesDuel:'Duels have no notes, erase or undo.', continueSaved:'You have an ongoing match.', dirty:'Local changes will be sent to the server.',
  },
};

export class LiveScreens {
  constructor({engine,storage,content,renderBoard,svg,escape,language,onHome,onUpdate,announce}) {
    Object.assign(this,{engine,storage,content,renderBoard,svg,escape,language,onHome,onUpdate,announce});
    this.selected = 0;
    this.notesMode = false;
    this.form = {name:language() === 'tr' ? 'Misafir':'Guest',mode:'race',size:6,invite:''};
    this.notice = '';
    this.shareLink = '';
    this.entered = false;
    if (engine.MultiplayerClient) this.transport = new MultiplayerTransport(engine,storage,{
      onChange:() => { if (this.entered) this.onUpdate(); },
      onError:error => { this.notice = this.t(error.includes('Busy') ? 'busy':error.includes('ReconcileRequired') ? 'reconcile':'invalid'); if (this.entered) this.onUpdate(); },
      onLeft:() => { this.leaving=false;this.entered = false; this.onHome(); },
    });
  }
  t(key,vars = {}) {
    return Object.entries(vars).reduce((s,[k,v]) => s.replaceAll(`{${k}}`,String(v)),words[this.language()][key] || words[this.language()].server);
  }
  enter(invite = '') {
    this.entered = true;
    if (invite) this.form.invite = invite;
    const view = this.transport?.view();
    if (this.transport?.restoreStatus === 'restored' && view.snapshot && !view.voluntarily_left
        && view.connection !== 'session_lost' && !view.pending_http) this.transport.dispatch({type:'resume'});
    this.onUpdate();
  }
  view() { return this.transport?.view() || null; }
  persist() { return this.transport?.persist(); }
  connection(view) {
    const state = view.connection;
    const label = state === 'connected' ? 'connected':state === 'reconnecting' ? 'reconnecting':state === 'session_lost' ? 'lost':'disconnected';
    return `<p class="connection-strip ${state === 'connected' ? '':'is-offline'}" data-testid="connection-status" role="status"><span aria-hidden="true">${state === 'connected' ? '●':'○'}</span> ${this.t(label)}</p>`;
  }
  storageNotice() {
    if (!this.transport?.storageError) return '';
    return `<p class="notice error" data-testid="storage-status" role="alert">${this.t('saveError')} <button data-action="live-save">${this.t('retry')}</button></p>`;
  }
  noticeHTML() {
    const notice = this.transport?.transportError ? this.t(this.transport.transportError):this.notice;
    return notice ? `<p class="notice error" role="status">${this.escape(notice)}</p>`:'';
  }
  render() {
    const focused = document.activeElement?.dataset.testid;
    const scrollY = window.scrollY;
    const view = this.view();
    if(view?.snapshot?.room_code!==this.lastRoomCode){this.shareLink='';this.lastRoomCode=view?.snapshot?.room_code;}
    if (!view) {
      this.content.innerHTML = `<section class="panel"><h1>${this.t('title')}</h1><p class="notice">${this.t('apiUnavailable')}</p><button class="secondary-button" data-action="live-home">${this.t('home')}</button></section>`;
      return;
    }
    if (this.transport.protectedSave) {
      this.content.innerHTML = `<section class="panel" data-screen="live-recovery"><h1>${this.t('title')}</h1><p class="notice error" role="alert">${this.t('corrupt')}</p><button class="primary-button" data-action="live-recover">${this.t('recover')}</button><button class="text-button" data-action="live-home">${this.t('home')}</button></section>`;
    } else if (!view.snapshot || view.voluntarily_left) this.renderSetup(view);
    else if (view.snapshot.phase === 'finished' && view.snapshot.result) this.renderResult(view);
    else if (view.connection === 'session_lost' || view.snapshot.phase === 'closed') this.renderLost(view);
    else if (['lobby','countdown'].includes(view.snapshot.phase)) this.renderLobby(view);
    else this.renderGame(view);
    if (focused) this.content.querySelector(`[data-testid="${CSS.escape(focused)}"]`)?.focus({preventScroll:true});
    if (window.scrollY !== scrollY) window.scrollTo({top:scrollY,behavior:'instant'});
  }
  renderSetup(view) {
    const busy = view.pending_http;
    this.content.innerHTML = `<section class="panel" data-screen="friend-setup"><button class="back-button" data-action="live-home" data-testid="back">${this.svg('back')}${this.t('back')}</button><p class="eyebrow">${this.t('title')}</p><h1>${this.t('setup')}</h1><p class="lede">${this.t('intro')}</p>${this.noticeHTML()}${this.storageNotice()}${this.transport.transportError&&busy?`<button class="text-button" data-action="live-recover">${this.t('recover')}</button>`:''}
      <label class="form-label" for="live-name">${this.t('name')}</label><input id="live-name" class="text-input" data-live-field="name" value="${this.escape(this.form.name)}" maxlength="32" autocomplete="nickname" ${busy?'disabled':''}>
      <fieldset data-testid="game-mode"><legend>${this.t('mode')}</legend><div class="size-options">${['race','duel'].map(mode=>`<label class="size-option compact-option"><input type="radio" name="live-mode" data-live-field="mode" value="${mode}" ${this.form.mode===mode?'checked':''} ${busy?'disabled':''}><strong>${this.t(mode)}</strong></label>`).join('')}</div></fieldset>
      <fieldset data-testid="board-size"><legend>${this.t('size')}</legend><div class="size-options">${[6,9].map(size=>`<label class="size-option compact-option"><input type="radio" name="live-size" data-live-field="size" value="${size}" ${this.form.size===size?'checked':''} ${busy?'disabled':''}><strong>${size}×${size}</strong></label>`).join('')}</div></fieldset>
      <p class="note">${this.t('modeHelp')}</p><button class="primary-button full" data-action="live-create" data-testid="room-create" ${busy?'disabled':''}>${busy?this.t('pending'):this.t('create')}</button>
      <div class="join-section"><h2>${this.t('or')}</h2><label class="form-label" for="live-invite">${this.t('invite')}</label><input class="text-input" id="live-invite" data-live-field="invite" data-testid="room-code" value="${this.escape(this.form.invite)}" autocapitalize="characters" autocomplete="off" spellcheck="false" ${busy?'disabled':''}><button class="secondary-button full" data-action="live-join" data-testid="room-join" ${busy?'disabled':''}>${this.t('join')}</button></div>
    </section>`;
  }
  leaveButton(view) {
    return `<button class="back-button" data-action="live-leave" data-testid="back" aria-label="${this.t('resign')}">${this.svg('back')}<span>${this.t('resign')}</span></button>`;
  }
  renderLobby(view) {
    const snapshot = view.snapshot;
    const countdown = snapshot.phase === 'countdown';
    this.content.innerHTML = `<section class="panel" data-screen="lobby">${this.leaveButton(view)}<p class="eyebrow">${this.t(snapshot.mode)} · ${snapshot.size}×${snapshot.size}</p><h1>${this.t(countdown?'starting':'waiting')}</h1>${this.connection(view)}${this.noticeHTML()}${this.storageNotice()}
      <div class="invite-card"><span class="section-label">${this.t('room')}</span><strong class="room-code" data-testid="room-code">${this.escape(snapshot.room_code)}</strong><button class="secondary-button" data-action="live-share" data-testid="invite-share">${this.t('share')}</button>${this.shareLink?`<input class="text-input" value="${this.escape(this.shareLink)}" readonly aria-label="${this.t('invite')}">`:''}</div>
      <ul class="players" data-testid="players"><li><span>${this.escape(snapshot.own.name)} <small>${this.t('you')}</small></span><span>${this.t(snapshot.own.ready?'readyLabel':'notReady')}</span></li><li><span>${snapshot.opponent?this.escape(snapshot.opponent.name):this.t('noOpponent')}</span><span>${snapshot.opponent?this.t(snapshot.opponent.ready?'readyLabel':'notReady'):'—'}</span></li></ul>
      ${countdown?`<p class="countdown" data-testid="timer" role="timer">${this.countdown(view)}</p>`:''}
      <button class="primary-button full" data-action="live-ready" data-testid="ready-toggle" role="switch" aria-checked="${snapshot.own.ready}" ${!snapshot.opponent||view.pending_command||view.connection!=='connected'?'disabled':''}>${this.t(snapshot.own.ready?'unready':'ready')}</button>
    </section>`;
  }
  countdown(view) {
    if(view.estimated_server_ms===null)return this.t('starting');
    const ms = Math.max(0,(view.snapshot.start_ms || 0)-(view.estimated_server_ms || 0));
    return ms>0?this.t('startIn',{seconds:Math.ceil(ms/1000)}):this.t('starting');
  }
  board(view) {
    if (view.snapshot.mode === 'race') return view.race?.solo || null;
    return {size:view.snapshot.size,givens:view.snapshot.own.givens,cells:view.snapshot.own.cells.map(value=>({value,notes:0})),locked:view.snapshot.own.cells.map(value=>value!==0)};
  }
  canInput(view) {
    if (!view.snapshot || view.snapshot.phase !== 'playing' || view.reconcile_required || this.leaving) return false;
    if (view.snapshot.mode === 'race') return !!view.race && view.connection !== 'session_lost';
    return view.connection === 'connected' && !view.pending_command
      && (view.locked_remaining_ms || 0)===0 && (view.remaining_ms ?? 0)>0;
  }
  renderGame(view) {
    const s=view.snapshot;
    const race=s.mode==='race';
    const board=this.board(view);
    if (!board || board.cells.length!==s.size*s.size) { this.renderLobby(view); return; }
    const boardIdentity=`${s.match_id}:${s.own.board_id}`;
    if (this.lastBoardIdentity!==boardIdentity) { this.selected=Math.max(0,board.givens.findIndex(value=>value===0));this.lastBoardIdentity=boardIdentity; }
    this.selected=Math.min(this.selected,board.cells.length-1);
    const fixed=board.givens[this.selected]!==0 || (!race && board.cells[this.selected].value!==0);
    const canInput=this.canInput(view) && !fixed;
    const tool=(action,label,disabled=false,extra='')=>`<button class="tool" data-action="live-${action}" data-testid="${action==='notes'?'notes-toggle':action}" ${disabled?'disabled':''} ${extra}>${this.svg(action)}<span>${this.t(label)}</span></button>`;
    const receipt=view.last_receipt?.code;
    const feedback=receipt==='wrong'?this.t('wrong'):receipt==='not_correct'?this.t('notCorrect'):receipt==='rate_limited'?this.t('rateLimited'):this.notice;
    const conflicts=new Set(race?(view.race?.conflicts||[]):[]);
    this.content.innerHTML=`<section class="game" data-screen="${race?'race-game':'duel-game'}"><div class="game-top">${this.leaveButton(view)}<div class="game-heading"><h1>${this.t(s.mode)} · ${s.size}×${s.size}</h1><span class="timer" data-testid="timer" role="timer" aria-label="${this.t(race?'elapsed':'remaining')}">${this.timer(view)}</span></div><span class="opponent-name">${this.escape(s.opponent?.name||this.t('opponent'))}</span></div>${this.connection(view)}${this.storageNotice()}${this.noticeHTML()}
      ${view.reconcile_required?`<div class="notice error" role="alert"><p>${this.t('reconcile')}</p><button data-action="live-keep">${this.t('keep')}</button> <button data-action="live-accept">${this.t('accept')}</button></div>`:''}
      <div class="live-scoreline">${race?`<span data-testid="opponent-progress">${this.t('progress')}: ${s.opponent?.progress_percent||0}%</span>`:`<span data-testid="own-score">${this.t('score')}: <strong>${s.own.score}</strong></span><span data-testid="opponent-score">${this.t('opponentScore')}: <strong>${s.opponent?.score||0}</strong></span><span data-testid="combo">${this.t('combo')}: ${s.own.combo}</span>`}</div>
      <div class="game-layout"><div class="board-wrap">${this.renderBoard(board,conflicts,this.selected)}<p class="game-status ${conflicts.size?'conflict':''}" data-testid="game-status" role="status">${this.svg(conflicts.size?'warning':'check')}${conflicts.size?this.t('conflict'):view.pending_command?this.t('pending'):this.t('playing')}</p></div>
      <div class="input-panel"><p class="input-label">${this.t('numbers')}</p><div class="number-pad">${Array.from({length:s.size},(_,i)=>`<button class="number-button" data-action="live-digit" data-value="${i+1}" data-testid="number-${i+1}" aria-label="${i+1}" ${canInput?'':'disabled'}>${i+1}</button>`).join('')}</div>
      ${race?`<div class="tools live-tools">${tool('notes','notes',!this.canInput(view),`role="switch" aria-checked="${this.notesMode}"`)}${tool('erase','erase',!canInput)}${tool('undo','undo',!this.canInput(view)||!view.race.can_undo)}${tool('redo','redo',!this.canInput(view)||!view.race.can_redo)}</div>`:`<p class="note">${this.t('noNotesDuel')}</p><p data-testid="input-lock" class="input-lock" role="status">${this.lockLabel(view)}</p>`}
      <p class="selection-info">${fixed?this.t('given'):this.t('selected',{row:Math.floor(this.selected/s.size)+1,column:this.selected%s.size+1})}${race&&view.race.notes[this.selected]?.length?`<br>${this.t('notesList',{notes:view.race.notes[this.selected].join(', ')})}`:''}</p>
      ${feedback?`<p class="notice" role="status">${this.escape(feedback)}</p>`:''}<p class="note">${race&&view.race.dirty?this.t('dirty'):this.t('saved')}</p><p class="keyboard-help">${this.t('keyboard',{size:s.size})}</p>
      </div></div></section>`;
  }
  timer(view) {
    if(view.estimated_server_ms===null)return '—:—';
    if (view.snapshot.mode==='duel') return formatLiveTime(view.remaining_ms??90000,true);
    return formatLiveTime(Math.max(0,(view.estimated_server_ms||view.snapshot.server_ms)-(view.snapshot.start_ms||view.snapshot.server_ms)));
  }
  lockLabel(view) { return view.locked_remaining_ms>0?this.t('inputLock',{seconds:Math.ceil(view.locked_remaining_ms/1000)}):''; }
  renderResult(view) {
    const s=view.snapshot;
    const result=s.result;
    const noContest=!result || (result.winner===null && ['both_disconnected','resource_limit'].includes(result.reason));
    const key=noContest?'noContest':result.winner===null?'draw':result.winner===s.own.seat?'win':'loss';
    this.content.innerHTML=`<section class="result" data-screen="match-result"><p class="eyebrow">${this.t(s.mode)} · ${s.size}×${s.size}</p><h1 data-testid="result-heading">${this.t(key)}</h1>${view.connection==='session_lost'?`<p role="status">${this.t('room_closed')}</p>`:this.connection(view)}${this.noticeHTML()}${this.storageNotice()}<p data-testid="result-details">${s.mode==='duel'?`${this.escape(s.own.name)} ${s.own.score}${s.opponent?` — ${s.opponent.score} ${this.escape(s.opponent.name)}`:''}`:this.escape(s.opponent?.name||'')}</p>${result?`<p data-testid="result-reason">${this.t('reason_'+result.reason)}</p>`:''}
      ${!s.opponent?`<p role="status">${this.t('opponentLeft')}</p>`:s.own.rematch?`<p role="status">${this.t('rematchWaiting')}</p>`:s.opponent?.rematch?`<p role="status">${this.t('rematchOpponent')}</p>`:''}
      <div class="action-stack"><button class="primary-button" data-action="live-rematch" data-testid="rematch" ${!s.opponent||s.own.rematch||view.pending_command||view.connection!=='connected'?'disabled':''}>${this.t('rematch')}</button><button class="secondary-button" data-action="live-leave" data-testid="home">${this.t('home')}</button></div></section>`;
  }
  renderLost(view) {
    this.content.innerHTML=`<section class="panel" data-screen="live-error"><h1>${this.t('title')}</h1>${this.connection(view)}${this.noticeHTML()}<p>${this.t('savedLocal')}</p><div class="action-stack"><button class="secondary-button" data-action="live-leave">${this.t('home')}</button><button class="text-button" data-action="live-recover">${this.t('recover')}</button></div></section>`;
  }
  updateClock() {
    if (!this.entered || !this.transport) return;
    const view=this.view();
    const timer=this.content.querySelector('[data-testid="timer"]');
    if (timer && view.snapshot) timer.textContent=view.snapshot.phase==='countdown'?this.countdown(view):this.timer(view);
    const lock=this.content.querySelector('[data-testid="input-lock"]');
    if (lock) lock.textContent=this.lockLabel(view);
    if (this.content.querySelector('[data-screen="duel-game"]')) {
      const board=this.board(view);
      const fixed=board.givens[this.selected] || board.cells[this.selected].value;
      for (const button of this.content.querySelectorAll('.number-button')) button.disabled=!this.canInput(view)||Boolean(fixed);
      if (view.remaining_ms===0 && view.snapshot.phase==='playing') this.content.querySelector('[data-testid="game-status"]').textContent=this.t('resultPending');
    }
  }
  tick() { if (this.transport) { this.transport.tick(); this.updateClock(); } }
  input(field,value) { if (field in this.form) this.form[field]=field==='size'?Number(value):value; }
  dispatch(action) { this.notice=''; return this.transport?.dispatch(action); }
  async handle(action,button) {
    const view=this.view();
    if (action==='live-home') {
      if (view?.snapshot && !view.voluntarily_left && view.connection!=='session_lost') return this.handle('live-leave',button);
      this.entered=false;this.onHome();return;
    }
    if (action==='live-recover') {
      if (window.confirm(this.t('confirmRecover'))) { this.transport.replaceSession();this.notice='';this.onUpdate(); }return;
    }
    if (action==='live-save') { this.transport.persist();this.onUpdate();return; }
    if (!this.transport || this.transport.protectedSave) return;
    if (action==='live-create') { this.dispatch({type:'create',name:this.form.name,mode:this.form.mode,size:this.form.size});return; }
    if (action==='live-join') { this.dispatch({type:'join',invite:this.form.invite,name:this.form.name});return; }
    if (action==='live-ready') { this.dispatch({type:'command',command:{type:'ready',ready:!view.snapshot.own.ready}});return; }
    if (action==='live-rematch') { this.dispatch({type:'command',command:{type:'rematch'}});return; }
    if (action==='live-reconnect') { this.dispatch({type:'resume'});return; }
    if (action==='live-leave') {
      if (!view.snapshot) { this.entered=false;this.onHome();return; }
      if (view.snapshot.phase!=='finished' && !view.voluntarily_left && !window.confirm(this.t('confirmLeave'))) return;
      if(this.dispatch({type:'leave'})){this.leaving=true;this.notice=this.t('leaving');this.onUpdate();}return;
    }
    if (action==='live-share') {
      const url=new URL(`/r/${view.snapshot.room_code}`,location.origin).href;
      try {
        if (navigator.share) await navigator.share({title:'Sudoku Rivals',url});
        else { await navigator.clipboard.writeText(url);this.notice=this.t('copied'); }
      } catch(error) { if (error.name!=='AbortError') {this.shareLink=url;this.notice=this.t('copyFailed');} }
      this.onUpdate();return;
    }
    if (action==='live-keep') { this.dispatch({type:'reconcile',keep_local:true});return; }
    if (action==='live-accept') { if(window.confirm(this.t('confirmAccept')))this.dispatch({type:'reconcile',keep_local:false});return; }
    if (!view.snapshot || view.snapshot.phase!=='playing') return;
    if (action==='select') { this.selected=Number(button.dataset.index);this.onUpdate();this.focusCell();return; }
    if (action==='live-notes' && view.snapshot.mode==='race') { this.notesMode=!this.notesMode;this.onUpdate();return; }
    if (['live-undo','live-redo','live-erase'].includes(action) && view.snapshot.mode==='race' && this.canInput(view)) {
      this.dispatch({type:'edit',command:action==='live-erase'?{type:'erase',index:this.selected}:{type:action.slice(5)}});this.focusCell();return;
    }
    if (action==='live-digit') this.enterDigit(Number(button.dataset.value));
  }
  enterDigit(value) {
    const view=this.view();
    if (!this.canInput(view)) return;
    const board=this.board(view);
    if (board.givens[this.selected] || (view.snapshot.mode==='duel' && board.cells[this.selected].value)) return;
    if (view.snapshot.mode==='race') this.dispatch({type:'edit',command:{type:this.notesMode?'toggle_note':'set',index:this.selected,value}});
    else this.dispatch({type:'command',command:{type:'move',board_id:view.snapshot.own.board_id,index:this.selected,value}});
    this.focusCell();
  }
  focusCell() { this.content.querySelector(`[data-index="${this.selected}"]`)?.focus({preventScroll:true}); }
  keydown(event) {
    if (!event.target.closest('[role="grid"]')) return;
    const view=this.view();if(!view?.snapshot)return;
    const size=view.snapshot.size;
    let index=this.selected;
    if(event.key==='ArrowLeft')index=index%size?index-1:index;
    else if(event.key==='ArrowRight')index=index%size<size-1?index+1:index;
    else if(event.key==='ArrowUp')index=index>=size?index-size:index;
    else if(event.key==='ArrowDown')index=index<size*(size-1)?index+size:index;
    else if(/^[1-9]$/.test(event.key)&&Number(event.key)<=size&&!event.ctrlKey&&!event.metaKey&&!event.altKey){event.preventDefault();if(!event.repeat)this.enterDigit(Number(event.key));return;}
    else if(view.snapshot.mode==='race'&&['Backspace','Delete'].includes(event.key)){event.preventDefault();if(!event.repeat)this.handle('live-erase',event.target);return;}
    else if(view.snapshot.mode==='race'&&event.key.toLowerCase()==='n'&&!event.ctrlKey&&!event.metaKey&&!event.altKey){event.preventDefault();if(!event.repeat)this.handle('live-notes',event.target);return;}
    else if(view.snapshot.mode==='race'&&event.key.toLowerCase()==='z'&&(event.ctrlKey||event.metaKey)){event.preventDefault();if(!event.repeat)this.handle(event.shiftKey?'live-redo':'live-undo',event.target);return;}
    else return;
    event.preventDefault();this.selected=index;this.onUpdate();this.focusCell();
  }
}
function formatLiveTime(ms,roundUp=false) {
  const seconds=Math.max(0,roundUp?Math.ceil(ms/1000):Math.floor(ms/1000));
  return `${Math.floor(seconds/60)}:${String(seconds%60).padStart(2,'0')}`;
}
