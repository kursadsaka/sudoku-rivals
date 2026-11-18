import assert from 'node:assert/strict';
import { transientNotice, LiveScreens } from '../apps/web/live.js';

const realSet = globalThis.setTimeout, realClear = globalThis.clearTimeout;
const pending = new Map();
let id = 0;
globalThis.setTimeout = (callback, delay) => { pending.set(++id,{callback,delay}); return id; };
globalThis.clearTimeout = timer => pending.delete(timer);
try {
  const element = {textContent:''};
  const notify = transientNotice(element);
  notify('Preferences saved');
  assert.equal(element.textContent,'Preferences saved');
  const first = id;
  notify('Link copied');
  assert.equal(pending.has(first),false,'replacement cancels the previous expiry');
  assert.equal(pending.size,1);
  assert.equal(pending.get(id).delay,5000);
  pending.get(id).callback();
  assert.equal(element.textContent,'');
  notify('Preferences saved');
  notify('');
  assert.equal(element.textContent,'','a failed save clears a previous success');

  const live = Object.create(LiveScreens.prototype);
  Object.assign(live,{transport:{},escape:text=>text,t:key=>key});
  for (const [message,tone] of [['save failed','error'],['leaving','info'],['reconcile','error']]) {
    live.say(message,tone);
    assert.ok(live.noticeHTML().includes(message),'persistent messages remain in the flow');
  }
  live.transport.transportError='disconnected';
  assert.ok(live.noticeHTML().includes('disconnected'));
  live.transport.storageError=true;
  assert.ok(live.storageNotice().includes('role="alert"'));

  // Connection dot: every state has its own data-state; text appears for anything but a healthy compact dot.
  const states = {connected:'connected',reconnecting:'reconnecting',disconnected:'disconnected',session_lost:'lost'};
  for (const [connection,state] of Object.entries(states)) {
    const html = live.connection({connection});
    assert.ok(html.includes(`data-state="${state}"`),state);
    assert.ok(html.includes('class="connection-dot" aria-hidden="true"'),'the dot is decoration; the label is the name');
    assert.equal(html.includes('is-offline'),connection!=='connected');
  }
  assert.ok(live.connection({connection:'connected'},true).includes('<span class="sr-only">connected</span>'));

  // Pending: a named image, never the old sentence.
  const icon = live.pendingIcon();
  assert.ok(icon.includes('role="img"') && icon.includes('aria-label="pending"'));
  assert.ok(!/>\s*pending\s*</.test(icon));

  // In-match passing errors: one appearance per distinct message, expiry, dismissal and recurrence after recovery.
  const announced = [];
  const clock = {now:1000};
  const realNow = Date.now;
  Date.now = () => clock.now;
  try {
    Object.assign(live,{transport:{transportError:'network'},notice:'',announce:text=>announced.push(text)});
    assert.ok(live.toastHTML().includes('game-toast is-error'));
    assert.deepEqual(announced,['network']);
    clock.now += 2000;
    assert.ok(live.toastHTML().includes('network'),'the same failing retry keeps one toast');
    assert.equal(announced.length,1,'repeated failures are not announced again');
    clock.now += 4500;
    assert.equal(live.toastHTML(),'','expires after six seconds while the error persists');
    clock.now += 10000;
    assert.equal(live.toastHTML(),'','still the same failure: stays hidden');
    live.transport.transportError = null;
    assert.equal(live.toastHTML(),'','recovered');
    live.transport.transportError = 'network';
    assert.ok(live.toastHTML().includes('network'),'a new failure after recovery shows again');
    assert.equal(announced.length,2);
    live.toastUntil = 0;
    assert.equal(live.toastHTML(),'','dismissed');
  } finally { Date.now = realNow; }
} finally {
  globalThis.setTimeout=realSet;
  globalThis.clearTimeout=realClear;
}
console.log('Transient expiry/replacement and persistent warning checks passed');
