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
} finally {
  globalThis.setTimeout=realSet;
  globalThis.clearTimeout=realClear;
}
console.log('Transient expiry/replacement and persistent warning checks passed');
