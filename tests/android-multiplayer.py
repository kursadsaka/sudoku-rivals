#!/usr/bin/env python3
"""Real Android UI ↔ HTTP ↔ WASM-bot integration on a disposable emulator.

Owns only a fresh server on --server-port (never 3000) and a fault proxy.
Credential-bearing Android records are read only into memory, never printed.
Existing private records and airplane mode are restored in finally.
"""
import argparse
import hashlib
import http.server
import json
import os
import pathlib
import re
import socket
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET

parser = argparse.ArgumentParser()
parser.add_argument('--adb', required=True)
parser.add_argument('--serial', default='emulator-5554')
parser.add_argument('--server-bin', default='/tmp/sudoku-shared-target/debug/sudoku-server')
parser.add_argument('--server-port', type=int, default=3001)
parser.add_argument('--proxy-port', type=int, default=3002)
args = parser.parse_args()
assert args.server_port != 3000 and args.proxy_port != 3000, 'Never replace the web worker server.'
ROOT = pathlib.Path(__file__).resolve().parent.parent
PKG = 'com.sudokurivals'
UPSTREAM = f'http://127.0.0.1:{args.server_port}'
ANDROID_ORIGIN = f'http://10.0.2.2:{args.proxy_port}'


def adb(*parts, data=None, check=True):
    result = subprocess.run([args.adb, '-s', args.serial, *parts], input=data,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=35, check=check)
    return result.stdout


def shell(*parts): return adb('shell', *parts).decode().strip()
def stop(): shell('am', 'force-stop', PKG)
def launch(inject=True):
    parts=['am','start','-W','-n',PKG+'/.MainActivity']
    if inject: parts += ['--es','dev_server_url',ANDROID_ORIGIN]
    shell(*parts)
def read(name): return adb('exec-out', 'run-as', PKG, 'cat', 'files/' + name)
def write(name, data): adb('shell', '-T', 'run-as', PKG, 'tee', 'files/' + name, data=data)


def private_files():
    return [name for name in adb('exec-out', 'run-as', PKG, 'ls', '-1p', 'files', check=False).decode().splitlines()
            if re.fullmatch(r'(?:live-session|live-result-score|solo-[\w.-]+)[\w.-]*', name)]


def saved_client():
    return json.loads(json.loads(read('live-session.json'))['client'])


def app_snapshot(): return ((saved_client() or {}).get('auth') or {}).get('snapshot')


def until(test, timeout=15, message='Condition did not become true'):
    end = time.monotonic() + timeout
    while True:
        try:
            value = test()
            if value: return value
        except (FileNotFoundError, json.JSONDecodeError, KeyError, subprocess.CalledProcessError):
            pass
        if time.monotonic() >= end: raise AssertionError(message)
        time.sleep(.1)


def hierarchy():
    shell('uiautomator', 'dump', '/sdcard/sudoku-live-check.xml')
    return ET.fromstring(adb('exec-out', 'cat', '/sdcard/sudoku-live-check.xml'))


def node(tag, scroll=False):
    # Dialogs and recompositions land asynchronously; re-read before scrolling.
    for attempt in range(10 if scroll else 6):
        for item in hierarchy().iter('node'):
            resource = item.attrib.get('resource-id', '')
            if resource == tag or resource.endswith(':id/' + tag): return item
        if scroll and attempt % 2: shell('input', 'swipe', '540', '1620', '540', '600', '160')
        else: time.sleep(.3)
    raise AssertionError('UI node missing: ' + tag)


def tap(tag, scroll=False):
    item = node(tag, scroll)
    for _ in range(10):
        if item.attrib.get('enabled') == 'true': break
        time.sleep(.3)
        item = node(tag, scroll)
    assert item.attrib.get('enabled') == 'true', 'UI node disabled: ' + tag
    x1, y1, x2, y2 = map(int, re.findall(r'\d+', item.attrib['bounds']))
    shell('input', 'tap', str((x1+x2)//2), str((y1+y2)//2))
    time.sleep(.15)


def text(tag, value):
    tap(tag, True)
    shell('input', 'keycombination', '113', '29')  # Ctrl+A.
    shell('input', 'text', value.replace(' ', '%s'))
    shell('input', 'keyevent', 'KEYCODE_BACK')


def select(index, size): tap(f'cell-{index//size+1}-{index%size+1}')


def solve(cells, size):
    board = cells.copy()
    rows = 2 if size == 6 else 3
    def search():
        best = None
        candidates = None
        for i, current in enumerate(board):
            if current: continue
            r, c = divmod(i, size)
            values = [v for v in range(1, size+1)
                      if v not in board[r*size:(r+1)*size]
                      and all(board[j*size+c] != v for j in range(size))
                      and all(board[rr*size+cc] != v for rr in range(r//rows*rows, r//rows*rows+rows)
                              for cc in range(c//3*3, c//3*3+3))]
            if not values: return False
            if candidates is None or len(values) < len(candidates): best, candidates = i, values
        if best is None: return True
        for v in candidates:
            board[best] = v
            if search(): return True
        board[best] = 0
        return False
    assert search(), 'Bot puzzle unsatisfiable'
    return board


class FaultProxy(http.server.ThreadingHTTPServer):
    daemon_threads = True
    offline = False
    drop_next_move = False
    calls = []


class ProxyHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_): pass
    def do_POST(self):
        if self.server.offline:
            self.close_connection = True
            return
        body = self.rfile.read(int(self.headers.get('Content-Length', '0')))
        parsed = json.loads(body)
        command = parsed.get('command', {})
        entry = {'path': self.path, 'type': command.get('type'), 'hash': hashlib.sha256(body).hexdigest()}
        self.server.calls.append(entry)
        try:
            request = urllib.request.Request(UPSTREAM + self.path, body, {'Content-Type': 'application/json'}, method='POST')
            try:
                with urllib.request.urlopen(request, timeout=10) as response:
                    status, content = response.status, response.read(131073)
            except urllib.error.HTTPError as error:
                status, content = error.code, error.read(131073)
            if self.server.drop_next_move and command.get('type') == 'move':
                self.server.drop_next_move = False
                self.close_connection = True
                return
            self.send_response(status)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(content)))
            self.end_headers()
            self.wfile.write(content)
        except (OSError, ValueError):
            self.close_connection = True


NODE_BOT = r'''
import fs from 'node:fs';
import readline from 'node:readline';
const core=await import(process.argv[2]);
await core.default({module_or_path:fs.readFileSync(process.argv[3])});
const {MultiplayerTransport}=await import(process.argv[4]);
class MemoryStorage {map=new Map();getItem(k){return this.map.get(k)??null;}setItem(k,v){this.map.set(k,v);}}
const adapter=new MultiplayerTransport(core,new MemoryStorage(),{origin:process.argv[5],onError:()=>{}});
const guest=Array.from(crypto.getRandomValues(new Uint8Array(32)),v=>v.toString(16).padStart(2,'0')).join('');
let busy=false,paused=false;
const timer=setInterval(()=>{if(!busy&&!paused)adapter.tick();},100);
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
async function settle(){const end=performance.now()+15000;while(adapter.active.size){if(performance.now()>end)throw Error('timeout');await sleep(10);}}
let chain=Promise.resolve();
readline.createInterface({input:process.stdin}).on('line',line=>{
 chain=chain.then(async()=>{busy=true;try{
  const message=JSON.parse(line);paused=Boolean(message.pause);await settle();
  if(message.op==='stop'){clearInterval(timer);adapter.stop();process.stdout.write('{"ok":true}\n');process.exit(0);}
  if(message.action&&['create','join'].includes(message.action.type))message.action.guest=guest;
  if(message.action&&!adapter.dispatch(message.action))throw Error('action rejected');
  await settle();process.stdout.write(JSON.stringify({ok:true,view:adapter.view()})+'\n');
 }catch{process.stdout.write('{"ok":false,"error":"bot_operation_failed"}\n');}finally{busy=false;}});
}).on('close',()=>{clearInterval(timer);adapter.stop();process.exit(0);});
'''


class Bot:
    def __init__(self, script):
        self.process = subprocess.Popen(['node', str(script), (ROOT/'apps/web/pkg/sudoku_wasm.js').as_uri(),
            str(ROOT/'apps/web/pkg/sudoku_wasm_bg.wasm'), (ROOT/'apps/web/live.js').as_uri(), UPSTREAM],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    def call(self, action=None, pause=False):
        self.process.stdin.write(json.dumps({'action': action, 'pause': pause}) + '\n'); self.process.stdin.flush()
        result = json.loads(self.process.stdout.readline())
        assert result.get('ok'), 'WASM bot operation failed'
        return result['view']
    def close(self):
        if self.process.poll() is None:
            self.process.stdin.close()
            try: self.process.wait(timeout=5)
            except subprocess.TimeoutExpired: self.process.kill(); self.process.wait()


def server_start():
    with socket.socket() as probe:
        assert probe.connect_ex(('127.0.0.1', args.server_port)) != 0, 'Test server port already in use; not replacing it.'
    env = dict(os.environ, SUDOKU_PORT=str(args.server_port))
    process = subprocess.Popen([args.server_bin], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    def ready():
        if process.poll() is not None: raise AssertionError('Owned test server exited during startup')
        with socket.socket() as probe: return probe.connect_ex(('127.0.0.1', args.server_port)) == 0
    until(ready, 10, 'Owned test server did not start')
    return process


def server_stop(process):
    if process and process.poll() is None:
        process.terminate()
        try: process.wait(timeout=10)
        except subprocess.TimeoutExpired: process.kill(); process.wait()


def ready_match(bot):
    until(lambda: app_snapshot() and app_snapshot()['opponent'] is not None)
    tap('ready-toggle', True)
    bot.call({'type': 'command', 'command': {'type': 'ready', 'ready': True}})
    until(lambda: app_snapshot()['phase'] == 'playing', 12, 'Native countdown did not start match')
    snapshot = app_snapshot()
    partner = bot.call()['snapshot']
    assert snapshot['match_id'] == partner['match_id'], 'Clients entered different matches'
    assert len(snapshot['own']['cells']) == snapshot['size']**2
    return snapshot


def leave_native():
    tap('back')
    tap('leave-confirm')
    until(lambda: saved_client().get('voluntarily_left'), 15, 'Explicit leave not acknowledged')
    node('friend-start', True)


def fill_native(snapshot, solution):
    # Physical key navigation exercises the native adapter, never edits its record.
    n = snapshot['size']
    current = 0
    select(0, n)
    # Android may consume the first navigation key after a touch to leave
    # touch mode. UP at cell 0 is a no-op either way, so later moves stay exact.
    shell('input', 'keyevent', 'KEYCODE_DPAD_UP')
    for index, given in enumerate(snapshot['own']['givens']):
        if given: continue
        row, column = divmod(current, n)
        target_row, target_column = divmod(index, n)
        keys = ['KEYCODE_DPAD_DOWN' if target_row > row else 'KEYCODE_DPAD_UP'] * abs(target_row-row)
        keys += ['KEYCODE_DPAD_RIGHT' if target_column > column else 'KEYCODE_DPAD_LEFT'] * abs(target_column-column)
        # One cell per injection, like a person typing; a single burst of
        # dozens of keys measures adb throughput rather than the app.
        shell('input', 'keyevent', *keys, f'KEYCODE_{solution[index]}')
        current = index


assert shell('getprop', 'ro.kernel.qemu') == '1', 'Only run on a disposable emulator.'
stop(); launch()
original = {name: read(name) for name in private_files()}
airplane = shell('settings', 'get', 'global', 'airplane_mode_on')
proxy = None
server = None
bots = []
with tempfile.TemporaryDirectory(prefix='android-live-bot-') as temporary:
    script = pathlib.Path(temporary)/'bot.mjs'; script.write_text(NODE_BOT)
    try:
        stop()
        for name in private_files():
            if name.startswith(('live-session', 'live-result-score')): shell('run-as', PKG, 'rm', '-f', 'files/'+name)
        shell('cmd', 'connectivity', 'airplane-mode', 'disable')
        server = server_start()
        proxy = FaultProxy(('127.0.0.1', args.proxy_port), ProxyHandler)
        threading.Thread(target=proxy.serve_forever, daemon=True).start()
        launch()
        for mode in ['race', 'duel']:
            for size in [6, 9]:
                bot = Bot(script); bots.append(bot)
                tap('friend-start', True)
                if app_snapshot() and not saved_client().get('voluntarily_left'):
                    tap('live-recover', True); tap('live-reset-confirm', True)
                if size == 9:
                    partner = bot.call({'type': 'create', 'name': 'WebBot', 'mode': mode, 'size': size})
                    code = partner['snapshot']['room_code']
                    # The explicit development HTTP deep link is parsed by Rust on Join.
                    shell('am', 'start', '-W', '-a', 'android.intent.action.VIEW', '-d', ANDROID_ORIGIN+'/r/'+code, '-p', PKG)
                    tap('room-join', True)
                else:
                    tap('mode-'+mode, True)
                    tap('board-size-'+str(size), True)
                    if mode == 'race':
                        old_record = read('live-session.json')
                        shell('run-as',PKG,'mkdir','files/live-session.json.new')
                        write('live-session.json.new/blocked',b'occupied')
                    calls_before = len(proxy.calls)
                    tap('room-create', True)
                    if mode == 'race':
                        assert read('live-session.json') == old_record, 'Failed save replaced prior session'
                        assert len(proxy.calls) == calls_before, 'Credential-bearing request sent before durable state'
                        node('live-storage-status')
                        shell('run-as',PKG,'rm','-rf','files/live-session.json.new')
                        tap('live-save-retry',True)
                    until(lambda: app_snapshot() is not None and app_snapshot()['mode'] == mode)
                    code = app_snapshot()['room_code']
                    bot.call({'type': 'join', 'invite': code, 'name': 'WebBot'})
                snapshot = ready_match(bot)
                solution = solve(snapshot['own']['givens'], size)
                blank = next(i for i, value in enumerate(snapshot['own']['givens']) if value == 0)
                if mode == 'race':
                    if size == 6:
                        proxy.offline = True
                        until(lambda: 'kesildi' in node('connection-status').attrib.get('text',''), 15, 'Native adapter did not report disconnect')
                        select(blank, size)
                        before_sync = sum(c['type'] == 'sync' for c in proxy.calls)
                        keys = ['KEYCODE_1','KEYCODE_2']*20
                        shell('input','keyevent',*keys)
                        until(lambda: len(json.loads(saved_client()['draft'])['undo']) == 40)
                        before = json.loads(saved_client()['draft'])
                        stop(); launch()
                        until(lambda: json.loads(saved_client()['draft'])['cells'] == before['cells'])
                        # Another authorized connection changes the same seat while this one is offline.
                        auth = saved_client()['auth']
                        foreign = auth['snapshot']['own']['cells'].copy()
                        foreign[blank] = 1
                        envelope = {'protocol':1,'token':auth['token'],'generation':auth['snapshot']['generation'],
                                    'match_id':auth['snapshot']['match_id'],'sequence':auth['snapshot']['own']['next_sequence'],
                                    'command':{'type':'sync','base_version':auth['snapshot']['own']['board_version'],'cells':foreign}}
                        request = urllib.request.Request(UPSTREAM+'/v1/rooms/'+auth['snapshot']['room_code']+'/command',json.dumps(envelope).encode(),{'Content-Type':'application/json'},method='POST')
                        with urllib.request.urlopen(request,timeout=10) as response: assert response.status == 200
                        proxy.offline = False
                        until(lambda: saved_client().get('reconcile'),20,'Divergent race board did not require explicit choice')
                        assert json.loads(saved_client()['draft'])['cells'] == before['cells'], 'Reconcile silently overwrote local work'
                        tap('reconcile-local',True)
                        until(lambda: app_snapshot()['generation'] >= 2 and app_snapshot()['own']['board_version'] >= 2, 20)
                        assert sum(c['type'] == 'sync' for c in proxy.calls)-before_sync == 1, 'Offline batch was not a single sync'
                        assert len(json.loads(saved_client()['draft'])['undo']) == 40
                    fill_native(app_snapshot(), solution)
                    until(lambda: app_snapshot()['phase'] == 'finished', 20, 'Native race did not complete')
                    assert app_snapshot()['result']['winner'] == app_snapshot()['own']['seat']
                else:
                    select(blank, size)
                    if size == 6:
                        proxy.drop_next_move = True
                        start = len(proxy.calls)
                        wrong = solution[blank] % size + 1
                        shell('input','keyevent','KEYCODE_'+str(wrong))
                        until(lambda: app_snapshot()['own']['wrong'] == 1, 15, 'Lost wrong response not retried')
                        moves = [c for c in proxy.calls[start:] if c['type'] == 'move']
                        assert len(moves) >= 2 and moves[0]['hash'] == moves[1]['hash'], 'Retry changed the command body'
                        assert app_snapshot()['own']['score'] == 0 and app_snapshot()['own']['wrong'] == 1
                        until(lambda: time.monotonic() and app_snapshot()['own']['wrong'] == 1)
                        time.sleep(2.1)
                        shell('input','keyevent','KEYCODE_'+str(solution[blank]))
                        until(lambda: app_snapshot()['own']['score'] == 10, 10, 'Native duel correct move not acknowledged')
                        partner = bot.call()['snapshot']
                        partner_solution = solve(partner['own']['givens'], size)
                        partner_blank = next(i for i,v in enumerate(partner['own']['givens']) if v == 0)
                        bot.call({'type':'command','command':{'type':'move','board_id':partner['own']['board_id'],'index':partner_blank,'value':partner_solution[partner_blank]}})
                        bot.call({'type':'command','command':{'type':'resign'}})
                        until(lambda: app_snapshot()['phase'] == 'finished', 10)
                        assert app_snapshot()['result']['winner'] == app_snapshot()['own']['seat']
                    else:
                        # Native rendering must await server time/result; no local winner.
                        shell('input','keyevent','KEYCODE_N')
                        assert saved_client()['draft'] is None, 'Duel exposed a notes/undo draft'
                        shell('input','keyevent','KEYCODE_HOME')
                        time.sleep(1)
                        launch()
                        until(lambda: app_snapshot()['generation'] >= 2, 15)
                        until(lambda: app_snapshot()['phase'] == 'finished', 100, '90-second duel did not finish authoritatively')
                        assert app_snapshot()['result']['reason'] == 'time' and app_snapshot()['result']['winner'] is None
                node('result-heading', True)
                if mode == 'duel' and size == 6:
                    assert 'Rakip: 10' in node('result-details', True).attrib.get('text', '')
                    bot.call({'type':'leave'})
                    until(lambda: app_snapshot()['opponent'] is None)
                    assert 'Rakip: 10' in node('result-details', True).attrib.get('text', ''), 'Verified final score lost on departure'
                    stop(); launch()
                    until(lambda: app_snapshot()['opponent'] is None and app_snapshot()['phase'] == 'finished')
                    assert 'Rakip: 10' in node('result-details', True).attrib.get('text', ''), 'Final score lost on cold restore'
                    assert node('rematch', True).attrib.get('enabled') == 'false'
                    leave_native()
                    bot.close()
                    print('Android duel 6x6: verified final score survives departure and cold restore', flush=True)
                    continue
                if mode == 'duel':
                    bot.call({'type':'command','command':{'type':'disconnect'}}, pause=True)
                    until(lambda: app_snapshot()['opponent']['connected'] is False)
                    assert node('rematch', True).attrib.get('enabled') == 'true', 'Cannot queue rematch during temporary opponent disconnect'
                    assert 'bağlantısı kesildi' in node('rematch-status', True).attrib.get('text', '')
                old_match = app_snapshot()['match_id']
                tap('rematch', True)
                if mode == 'duel':
                    until(lambda: app_snapshot()['own']['rematch'])
                    assert app_snapshot()['match_id'] == old_match, 'Rematch started with disconnected opponent'
                    bot.call({'type':'resume'})
                bot.call({'type':'command','command':{'type':'rematch'}})
                until(lambda: app_snapshot()['match_id'] != old_match, 12, 'Mutual rematch did not create a new match')
                ready_match(bot)
                # Leaving removes the seat, including its score. Never invent zero or offer rematch.
                bot.call({'type':'leave'})
                until(lambda: app_snapshot()['phase'] == 'finished' and app_snapshot()['opponent'] is None)
                assert app_snapshot()['result']['winner'] == app_snapshot()['own']['seat']
                assert 'kazandın' in node('result-heading', True).attrib.get('text', '')
                assert node('rematch', True).attrib.get('enabled') == 'false', 'Rematch offered without an opponent'
                if mode == 'duel':
                    assert 'Skor mevcut değil' in node('result-details', True).attrib.get('text', ''), 'Missing opponent score invented'
                # A later room loss must not hide an already verified result.
                if size == 9:
                    server_stop(server)
                    server = server_start()
                    until(lambda: 'dönülemiyor' in node('connection-status', True).attrib.get('text', ''), 20)
                    assert 'kazandın' in node('result-heading', True).attrib.get('text', '')
                    assert 'kazanan ilan edilmedi' not in node('live-message', True).attrib.get('text', ''), 'Room-loss banner contradicts verified result'
                    assert node('rematch', True).attrib.get('enabled') == 'false'
                    tap('home', True)
                    node('friend-start', True)
                else:
                    leave_native()
                bot.close()
                print(f'Android ↔ WASM HTTP {mode} {size}x{size}: full match/rematch/leave passed', flush=True)
        # Loss without a verified result must describe only data that this mode preserves.
        for mode in ['race', 'duel']:
            bot = Bot(script); bots.append(bot)
            tap('friend-start', True)
            # A lost old session requires an explicit reset before a new room.
            if app_snapshot() and not saved_client().get('voluntarily_left'):
                tap('live-recover', True); tap('live-reset-confirm', True)
            tap('mode-'+mode, True); tap('board-size-6', True); tap('room-create', True)
            until(lambda: app_snapshot() is not None and app_snapshot()['mode'] == mode and not saved_client().get('voluntarily_left'))
            bot.call({'type':'join','invite':app_snapshot()['room_code'],'name':'WebBot'})
            ready_match(bot)
            proxy.offline = True
            until(lambda: 'Sunucuya ulaşılamıyor' in node('live-message', True).attrib.get('text', ''), 20)
            message = node('live-message', True).attrib.get('text', '')
            assert ('Yerel yarış' in message) == (mode == 'race'), 'Disconnect copy claims a race draft in duel'
            server_stop(server)
            server = server_start()
            proxy.offline = False
            until(lambda: 'sürdürülemiyor' in node('result-heading', True).attrib.get('text',''), 20, 'Restart was not presented as lost session')
            calls_after_loss = len(proxy.calls)
            time.sleep(3)
            assert 'dönülemiyor' in node('connection-status', True).attrib.get('text', ''), 'Lost session resumed itself'
            assert len(proxy.calls) == calls_after_loss, 'Terminal session loss continued retrying HTTP'
            assert app_snapshot()['result'] is None, 'Restart invented a result'
            detail = node('result-details', True).attrib.get('text', '')
            assert ('Yerel yarış' in detail) == (mode == 'race'), 'Lost-session copy claims a race draft in duel'
            tap('home', True)
            print(f'Android {mode}: disconnect and unrecoverable-session copy passed', flush=True)
        stop()
        write('live-session.json',b'{broken-live-record')
        launch(False)
        tap('friend-start',True)
        node('live-recover',True)
        assert read('live-session.json') == b'{broken-live-record', 'Corrupt session silently replaced'
        tap('live-recover',True); tap('live-reset-confirm',True)
        assert any(read(name) == b'{broken-live-record' for name in private_files() if name.startswith('live-session-recovery-'))
        print('Android multiplayer passed: two modes/two sizes, deep links, exact lost-response retry, 40 offline edits/single sync, cold restart, explicit reconciliation, failed-save retry, corrupt-record preservation, 90-second timer, mutual rematch, missing-opponent score/rematch, verified-result preservation, mode-specific failure copy, explicit leave and server restart', flush=True)
    finally:
        for bot in bots: bot.close()
        if proxy: proxy.shutdown(); proxy.server_close()
        server_stop(server)
        stop()
        shell('run-as',PKG,'rm','-rf','files/live-session.json.new')
        for name in private_files(): shell('run-as', PKG, 'rm', '-f', 'files/'+name)
        for name, data in original.items(): write(name, data)
        shell('cmd', 'connectivity', 'airplane-mode', 'enable' if airplane == '1' else 'disable')
        shell('rm','-f','/sdcard/sudoku-live-check.xml')
