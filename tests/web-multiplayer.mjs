import fs from 'node:fs';
import assert from 'node:assert/strict';
import * as core from '../apps/web/pkg/sudoku_wasm.js';
import { MultiplayerTransport, LIVE_KEY } from '../apps/web/live.js';
await core.default({module_or_path:fs.readFileSync(new URL('../apps/web/pkg/sudoku_wasm_bg.wasm',import.meta.url))});
assert.equal(typeof core.MultiplayerClient,'function','Rebuild the WASM package with the shared multiplayer client');
class Storage {
  map=new Map(); fail=false;
  getItem(key){return this.map.get(key)??null;}
  setItem(key,value){if(this.fail)throw new Error('quota');this.map.set(key,value);}
}
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function until(test,timeout=3000){const end=performance.now()+timeout;while(!test()){if(performance.now()>end)throw new Error('Timed out');await sleep(10);}}
function snapshot(generation=1){return {protocol:1,room_code:'ABC234',match_id:'c'.repeat(64),generation,mode:'race',size:6,phase:'lobby',server_ms:100,event_order:generation,start_ms:null,end_ms:null,own:{seat:0,name:'A',ready:false,connected:true,next_sequence:1,board_id:0,board_version:0,givens:[],cells:[],score:0,combo:0,wrong:0,locked_until_ms:0,rematch:false},opponent:null,result:null};}
const session={protocol:1,guest:'a'.repeat(64),token:'b'.repeat(64),invite_path:'/r/ABC234',snapshot:snapshot()};
const response=(body,status=200)=>({ok:status>=200&&status<300,status,text:async()=>typeof body==='string'?body:JSON.stringify(body)});
const storage=new Storage();let now=0;const calls=[];const errors=[];
let pendingPoll;
const adapter=new MultiplayerTransport(core,storage,{now:()=>now,origin:'http://localhost',onError:e=>errors.push(e),fetcher:async(url,options)=>{
  calls.push({url,options});
  if(url.endsWith('/v1/rooms'))return response(session);
  if(url.endsWith('/resume'))return response(snapshot(2));
  if(url.endsWith('/state'))return new Promise((resolve,reject)=>{pendingPoll={resolve,reject};});
  if(url.endsWith('/leave'))return response({protocol:1,left:true});
  throw new Error('unexpected route');
}});
adapter.dispatch({type:'create',guest:session.guest,name:'A',mode:'race',size:6});
await until(()=>adapter.view().snapshot!==null);
assert.equal(adapter.view().connection,'connected');
assert.equal(calls[0].options.cache,'no-store');
assert.equal(calls[0].options.method,'POST');
assert.equal(calls[0].options.headers['Content-Type'],'application/json');
assert(!JSON.stringify(adapter.view()).includes(session.token));
assert(storage.getItem(LIVE_KEY).includes(session.token));
now=10;adapter.dispatch({type:'poll'});
await until(()=>!!pendingPoll);
now=20;adapter.dispatch({type:'resume'});
await until(()=>adapter.view().snapshot.generation===2);
pendingPoll.reject(new Error('old connection failed'));
await sleep(10);
assert.equal(adapter.view().connection,'connected','Old request failure must not disconnect resumed generation');
let left=false;adapter.onLeft=()=>{left=true;};
now=30;adapter.dispatch({type:'leave'});await until(()=>left);
assert.equal(adapter.view().connection,'session_lost');
adapter.stop();
assert.deepEqual(errors,[]);

const malformed=new Storage();
malformed.setItem(LIVE_KEY,'{broken');
const blocked=new MultiplayerTransport(core,malformed,{origin:'http://localhost',fetcher:async()=>response(session)});
assert.equal(blocked.restoreStatus,'corrupt');
blocked.tick();assert.equal(malformed.getItem(LIVE_KEY),'{broken');
assert.equal(blocked.persist(),false);
blocked.stop();assert.equal(malformed.getItem(LIVE_KEY),'{broken');
const future=new Storage();future.setItem(LIVE_KEY,JSON.stringify({...JSON.parse(storage.getItem(LIVE_KEY)),version:255}));
const futureAdapter=new MultiplayerTransport(core,future,{origin:'http://localhost'});
assert(futureAdapter.protectedSave);futureAdapter.stop();

const failedStorage=new Storage();failedStorage.fail=true;
const retained=new MultiplayerTransport(core,failedStorage,{origin:'http://localhost',fetcher:async()=>response(session)});
retained.dispatch({type:'create',guest:session.guest,name:'A',mode:'race',size:6});await until(()=>retained.view().snapshot!==null);
assert(retained.storageError);assert.equal(retained.view().snapshot.room_code,'ABC234');
failedStorage.fail=false;assert(retained.persist());retained.stop();

const invalidHTTP=new MultiplayerTransport(core,new Storage(),{origin:'http://localhost',onError:()=>{},fetcher:async()=>response('<html>gateway error</html>',503)});
invalidHTTP.dispatch({type:'create',guest:session.guest,name:'A',mode:'race',size:6});
await until(()=>invalidHTTP.view().connection==='disconnected'&&invalidHTTP.transportError!==null);
assert.equal(invalidHTTP.view().snapshot,null);assert.equal(invalidHTTP.transportError,'invalid_response');invalidHTTP.stop();
let aborted=false;
const timedOut=new MultiplayerTransport(core,new Storage(),{origin:'http://localhost',timeoutMs:20,fetcher:(_url,{signal})=>new Promise((_resolve,reject)=>signal.addEventListener('abort',()=>{aborted=true;reject(new Error('aborted'));}))});
timedOut.dispatch({type:'create',guest:session.guest,name:'A',mode:'race',size:6});await until(()=>aborted);
await until(()=>timedOut.transportError==='network');assert.equal(timedOut.view().snapshot,null);timedOut.stop();
// Browsers require Window.fetch to be invoked with its native receiver.
const originalFetch=globalThis.fetch;
let defaultAdapter;
try {
  globalThis.fetch=function(){assert(this===undefined||this===globalThis);return Promise.resolve(response(session));};
  defaultAdapter=new MultiplayerTransport(core,new Storage(),{origin:'http://localhost'});
  defaultAdapter.dispatch({type:'create',guest:session.guest,name:'A',mode:'race',size:6});
  await until(()=>defaultAdapter.view().snapshot!==null);
} finally {globalThis.fetch=originalFetch;defaultAdapter?.stop();}
let resolveOld;let creates=0;
const newSession={...session,token:'d'.repeat(64),invite_path:'/r/ABC235',snapshot:{...snapshot(),room_code:'ABC235'}};
const resetAdapter=new MultiplayerTransport(core,new Storage(),{origin:'http://localhost',fetcher:()=>++creates===1?new Promise(resolve=>{resolveOld=resolve;}):Promise.resolve(response(newSession))});
resetAdapter.dispatch({type:'create',guest:session.guest,name:'A',mode:'race',size:6});await until(()=>!!resolveOld);
resetAdapter.replaceSession();resetAdapter.dispatch({type:'create',guest:session.guest,name:'A',mode:'race',size:6});
await until(()=>resetAdapter.view().snapshot?.room_code==='ABC235');resolveOld(response(session));await sleep(20);
assert.equal(resetAdapter.view().snapshot.room_code,'ABC235','A replaced client must ignore callbacks from the previous epoch');resetAdapter.stop();
const {runInNewContext}=await import('node:vm');
const handlers=new Map();
runInNewContext(fs.readFileSync(new URL('../apps/web/sw.js',import.meta.url),'utf8'),{
  self:{location:{origin:'http://localhost'},addEventListener:(name,fn)=>handlers.set(name,fn)},URL,
  caches:{open:async()=>({match:async key=>({cached:key})})},fetch:async()=>{throw new Error('offline');},
});
for(const request of [{method:'POST',url:'http://localhost/v1/rooms'},
  {method:'GET',url:'http://localhost/v1/rooms/ABC234/state'},
  {method:'GET',url:'https://other.example/live.js'}]){
  let intercepted=false;handlers.get('fetch')({request,respondWith:()=>{intercepted=true;}});assert.equal(intercepted,false,'Live/foreign traffic must never enter app cache');
}
let offlineNavigation;
handlers.get('fetch')({request:{method:'GET',url:'http://localhost/r/ABC234',mode:'navigate'},respondWith:value=>{offlineNavigation=value;}});
assert.deepEqual(await offlineNavigation,{cached:'/index.html'});
console.log('Web multiplayer adapter: real WASM, private atomic save, malformed/future records, stale failure, timeout, malformed HTTP and explicit exit passed');

if(process.argv.includes('--http')){
  const origin=process.env.SUDOKU_SERVER_URL||'http://127.0.0.1:3000';
  const clients=[];
  function client(){const a=new MultiplayerTransport(core,new Storage(),{origin,onError:error=>{a.testError=error;}});clients.push(a);return a;}
  async function settle(a){await until(()=>a.active.size===0,10000);}
  async function action(a,action){const effect=a.dispatch(action);assert(effect,`Action rejected: ${action.type} ${a.testError || ''}`);await settle(a);return a.view();}
  async function poll(a){return action(a,{type:'poll'});}
  // A solver belongs only to this bot test, never to the application UI.
  function solve(cells,size){const n=size,rows=size===6?2:3,board=[...cells];function search(){const i=board.indexOf(0);if(i<0)return true;const r=Math.floor(i/n),c=i%n;for(let v=1;v<=n;v++){if(board.slice(r*n,(r+1)*n).includes(v)||Array.from({length:n},(_,j)=>board[j*n+c]).includes(v))continue;let used=false;for(let rr=Math.floor(r/rows)*rows;rr<Math.floor(r/rows)*rows+rows;rr++)for(let cc=Math.floor(c/3)*3;cc<Math.floor(c/3)*3+3;cc++)if(board[rr*n+cc]===v)used=true;if(used)continue;board[i]=v;if(search())return true;board[i]=0;}return false;}assert(search());return board;}
  try{
    async function lostSetup(action,pathSuffix){
      const savedStorage=new Storage();let firstSession;let lost=false;
      const original=new MultiplayerTransport(core,savedStorage,{origin,onError:()=>{},fetcher:async(url,options)=>{
        const result=await fetch(url,options);
        if(!lost&&url.endsWith(pathSuffix)&&result.ok){firstSession=await result.clone().json();lost=true;throw new Error('test lost setup response');}
        return result;
      }});clients.push(original);
      original.dispatch(action);await until(()=>lost&&original.active.size===0);
      assert.equal(original.view().snapshot,null);
      original.stop();
      const restored=new MultiplayerTransport(core,savedStorage,{origin,onError:()=>{}});clients.push(restored);
      assert.equal(restored.restoreStatus,'restored');
      const interval=setInterval(()=>restored.tick(),20);
      try{await until(()=>restored.view().snapshot!==null,5000);}finally{clearInterval(interval);}
      assert(restored.view().snapshot.room_code===firstSession.snapshot.room_code,'Lost setup retry must retain the original room');
      assert(restored.client.save().includes(firstSession.token),'Lost setup retry must retain the original seat credential');
      return restored;
    }
    const recoveredCreator=await lostSetup({type:'create',name:'Recovered A',mode:'race',size:6},'/v1/rooms');
    const recoveredJoiner=await lostSetup({type:'join',invite:recoveredCreator.view().snapshot.room_code,name:'Recovered B'},'/join');
    assert.equal(recoveredCreator.view().snapshot.match_id,recoveredJoiner.view().snapshot.match_id);
    await action(recoveredCreator,{type:'leave'});await action(recoveredJoiner,{type:'leave'});
    console.log('HTTP lost create/join response: opaque cold restore retries retained room and seat identity');
    for(const mode of ['race','duel'])for(const size of [6,9]){
      const a=client(),b=client();
      await action(a,{type:'create',name:'Web A',mode,size});
      const code=a.view().snapshot.room_code;
      await action(b,{type:'join',invite:`${origin}/r/${code}`,name:'Web B'});
      await poll(a);await action(a,{type:'command',command:{type:'ready',ready:true}});await action(b,{type:'command',command:{type:'ready',ready:true}});
      assert.equal(b.view().snapshot.phase,'countdown');
      assert.deepEqual(b.view().snapshot.own.givens,[],'Future board must stay secret during countdown');
      assert.deepEqual(b.view().snapshot.own.cells,[]);
      await sleep(3050);await poll(a);await poll(b);
      assert.equal(a.view().snapshot.phase,'playing');assert.equal(a.view().snapshot.match_id,b.view().snapshot.match_id);
      if(mode==='race'){
        const solution=solve(a.view().snapshot.own.givens,size);
        const first=solution.findIndex((_v,i)=>a.view().snapshot.own.givens[i]===0);
        await action(a,{type:'edit',command:{type:'toggle_note',index:first,value:1}});
        const actualFetch=a.fetcher;
        a.fetcher=async()=>{throw new Error('test network loss');};
        await action(a,{type:'poll'});assert.equal(a.view().connection,'disconnected');
        const version=a.view().snapshot.own.board_version;
        for(let edit=0;edit<40;edit++)await action(a,{type:'edit',command:{type:'set',index:first,value:edit%2?solution[first]:0}});
        assert.equal(a.view().race.cells[first],solution[first]);
        let syncCount=0;
        a.fetcher=(url,options)=>{if(JSON.parse(options.body).command?.type==='sync')syncCount++;return actualFetch(url,options);};
        await sleep(Math.max(0,(a.view().retry_at_ms||0)-Math.floor(performance.now()))+5);
        await action(a,{type:'resume'});a.tick();await settle(a);
        assert.equal(syncCount,1,'Forty disconnected edits must use one board synchronization');
        assert.equal(a.view().snapshot.own.board_version,version+1);
        const local=a.client.save();
        const restored=core.MultiplayerClient.restore(local);assert.equal(JSON.parse(restored.view(0)).race.cells[first],solution[first]);restored.free();
        for(let index=0;index<solution.length;index++)if(a.view().snapshot.own.givens[index]===0)await action(a,{type:'edit',command:{type:'set',index,value:solution[index]}});
        await action(a,{type:'sync'});await poll(b);
        assert.equal(a.view().snapshot.phase,'finished');assert.equal(a.view().snapshot.result.winner,a.view().snapshot.own.seat);
      }else{
        const solution=solve(a.view().snapshot.own.givens,size);const index=solution.findIndex((_v,i)=>a.view().snapshot.own.givens[i]===0);
        const before=a.view().snapshot.own.score;
        await action(a,{type:'command',command:{type:'move',board_id:0,index,value:solution[index]}});
        assert(a.view().snapshot.own.score>before);
        await action(b,{type:'command',command:{type:'resign'}});await poll(a);assert.equal(a.view().snapshot.phase,'finished');
      }
      await action(a,{type:'command',command:{type:'rematch'}});await action(b,{type:'command',command:{type:'rematch'}});await poll(a);
      assert.equal(a.view().snapshot.phase,'countdown');
      await sleep(3050);await poll(a);await poll(b);
      assert.equal(a.view().snapshot.phase,'playing');
      assert.equal(a.view().reconcile_required,false,'A clean finished race must not require reconciliation on rematch');
      if(mode==='race')assert.deepEqual(a.view().race.solo.givens,a.view().snapshot.own.givens);
      await action(a,{type:'resume'});assert(a.view().snapshot.generation>=2);
      await action(a,{type:'leave'});await action(b,{type:'leave'});
      console.log(`HTTP ${mode} ${size}x${size}: ready/countdown/play/result/rematch/resume/leave passed`);
    }
  }finally{for(const a of clients)a.stop();}
}

// A failed poll followed by terminal room loss must not restart via tick/online resume.
{
  let clock=0, requests=0;
  const saved=new Storage();
  const options={now:()=>clock,origin:'http://localhost',onError:()=>{},fetcher:async()=>{
    requests++;
    if(requests===1)return response(session);
    if(requests===2)throw new Error('offline');
    return response({protocol:1,error:'room_missing'},404);
  }};
  const lost=new MultiplayerTransport(core,saved,options);
  assert(lost.dispatch({type:'create',guest:session.guest,name:'A',mode:'race',size:6}));
  await until(()=>lost.active.size===0);
  clock=100;lost.dispatch({type:'poll'});
  await until(()=>lost.active.size===0);
  assert.notEqual(lost.view().retry_at_ms,null);
  clock=10000;lost.tick();
  await until(()=>lost.active.size===0);
  assert.equal(lost.view().connection,'session_lost');
  assert.equal(lost.view().retry_at_ms,null);
  const calls=requests;
  const restored=new MultiplayerTransport(core,saved,options);
  for(const client of [lost,restored]){
    for(let i=0;i<8;i++){
      clock+=250;client.tick();
      assert(!client.dispatch({type:'resume'}));
      assert.equal(client.view().connection,'session_lost');
    }
    client.stop();
  }
  assert.equal(requests,calls,'Lost sessions issued more HTTP requests');
  console.log('Web WASM terminal session loss: retry cleared, tick/online resume and cold restore stay terminal');
}

// An online callback must not replace a new setup with a resume of the old room.
{
  let clock=0, setupAttempts=0, rejectSetup;
  const saved=new Storage(), bodies=[];
  const nextSession=structuredClone(session);
  nextSession.snapshot.match_id='d'.repeat(64);
  nextSession.snapshot.room_code='DEF567';nextSession.invite_path='/r/DEF567';
  const client=new MultiplayerTransport(core,saved,{now:()=>clock,origin:'http://localhost',onError:()=>{},fetcher:async(url,options)=>{
    if(url.endsWith('/leave'))return response({protocol:1,left:true});
    assert(url.endsWith('/v1/rooms'),'Old-room resume escaped while setup pending');
    setupAttempts++;
    if(setupAttempts===1)return response(session);
    bodies.push(options.body);
    if(setupAttempts===2)return new Promise((_,reject)=>{rejectSetup=reject;});
    return response(nextSession);
  }});
  client.dispatch({type:'create',guest:session.guest,name:'A',mode:'race',size:6});
  await until(()=>client.active.size===0);
  clock=10;client.dispatch({type:'leave'});await until(()=>client.active.size===0);
  clock=20;client.dispatch({type:'create',name:'A',mode:'race',size:6});
  await until(()=>!!rejectSetup);
  const pending=client.client.save();
  assert(!client.dispatch({type:'resume'}));
  assert.equal(client.client.save(),pending,'In-flight setup was overwritten');
  rejectSetup(new Error('offline'));await until(()=>client.active.size===0);
  clock=10000;
  const failed=client.client.save();
  assert(!client.dispatch({type:'resume'}));
  assert.equal(client.client.save(),failed,'Failed setup was overwritten');
  client.tick();await until(()=>client.active.size===0);
  assert.equal(client.view().snapshot.match_id,nextSession.snapshot.match_id);
  assert.equal(client.view().voluntarily_left,false);
  assert.equal(bodies.length,2);assert.equal(bodies[0],bodies[1]);
  client.stop();
  console.log('Web WASM pending setup: online resume rejected before/after failure, exact new-room retry succeeds');
}

// G14: rejected setup retry after leave must never resume the departed room,
// including an online event; use the real WASM state and adapter.
{
  let time=0, failJoin=true;
  const paths=[];
  const t=new MultiplayerTransport(core,new Storage(),{now:()=>time,origin:'http://localhost',fetcher:async(url)=>{
    paths.push(new URL(url).pathname);
    if(url.endsWith('/leave'))return response({protocol:1,left:true});
    if(url.endsWith('/join')){if(failJoin){failJoin=false;throw Error('lost');}return response({protocol:1,error:'room_missing'},404);}
    if(url.endsWith('/v1/rooms'))return response(session);
    throw Error('Unexpected resume of departed room');
  }});
  t.dispatch({type:'create',name:'A',mode:'race',size:6});await until(()=>t.active.size===0);
  t.dispatch({type:'leave'});await until(()=>t.active.size===0);
  t.dispatch({type:'join',invite:'222222',name:'A'});await until(()=>t.active.size===0);
  time=t.view().retry_at_ms;t.tick();await until(()=>t.active.size===0);
  assert.equal(t.view().last_error,'room_missing');
  assert.equal(t.view().retry_at_ms,null);
  time+=10000;t.tick();await sleep(20);
  let online;
  const handler=fs.readFileSync(new URL('../apps/web/main.js',import.meta.url),'utf8').split('\n').find(line=>line.startsWith("window.addEventListener('online'"));
  assert(handler);
  runInNewContext(handler,{window:{addEventListener:(_event,callback)=>online=callback},screen:'live',live:{view:()=>t.view(),transport:t}});
  online();await sleep(20);
  assert.equal(paths.filter(p=>p.endsWith('/resume')).length,0);
  assert.equal(t.transportError,'room_missing');
  t.stop();
}
// Known results take precedence over lost room connectivity in actual screen rendering.
{
  const {LiveScreens}=await import('../apps/web/live.js');
  const oldDocument=globalThis.document,oldWindow=globalThis.window;
  globalThis.document={activeElement:null};globalThis.window={scrollY:0};
  try {
    for(const result of [{winner:0,reason:'resigned',at_ms:4000},null]) {
      let screen;
      LiveScreens.prototype.render.call({view:()=>({connection:'session_lost',snapshot:{...snapshot(),phase:result?'finished':'closed',result}}),
        transport:{protectedSave:false},content:{},renderResult:()=>screen='result',renderLost:()=>screen='lost'});
      assert.equal(screen,result?'result':'lost');
    }
  } finally {globalThis.document=oldDocument;globalThis.window=oldWindow;}
}
console.log('G14 departed-room and verified-result regressions passed');
