#!/usr/bin/env python3
"""Regression test on a disposable emulator: real Android UI <-> hold/drop proxy <-> real local server.

Scenario A: create is in flight, user navigates Home (system back), returns to the friend screen.
Scenario C: leave reaches the server, its response is lost, the app is backgrounded and reopened.
No credentials are printed: only paths, statuses, body hashes and view flags.
"""
import argparse, hashlib, http.server, json, os, re, socket, subprocess, sys, threading, time
import urllib.error, urllib.request
import xml.etree.ElementTree as ET

p = argparse.ArgumentParser()
p.add_argument('--adb', required=True)
p.add_argument('--serial', required=True)
p.add_argument('--server-bin', required=True)
p.add_argument('--server-port', type=int, required=True)
p.add_argument('--proxy-port', type=int, required=True)
args = p.parse_args()
PKG = 'com.sudokurivals'
UPSTREAM = f'http://127.0.0.1:{args.server_port}'
ORIGIN = f'http://10.0.2.2:{args.proxy_port}'
T0 = time.monotonic()
def say(*parts): print(f'[{time.monotonic()-T0:7.2f}]', *parts, flush=True)

def adb(*parts, data=None, check=True):
    return subprocess.run([args.adb, '-s', args.serial, *parts], input=data, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, timeout=40, check=check).stdout
def shell(*parts): return adb('shell', *parts).decode().strip()
def launch(): shell('am', 'start', '-W', '-n', PKG + '/.MainActivity', '--es', 'dev_server_url', ORIGIN)
def saved():
    raw = adb('exec-out', 'run-as', PKG, 'cat', 'files/live-session.json', check=False)
    return json.loads(json.loads(raw)['client'])
def flags():
    c = saved(); a = c.get('auth') or {}; s = a.get('snapshot') or {}
    return {'has_auth': bool(a), 'phase': s.get('phase'), 'room': s.get('room_code'), 'setup_pending': c.get('setup') is not None,
            'session_lost': c.get('session_lost'), 'voluntarily_left': c.get('voluntarily_left')}
def until(test, timeout=15, message='condition'):
    end = time.monotonic() + timeout
    while True:
        try:
            v = test()
            if v: return v
        except (json.JSONDecodeError, KeyError, subprocess.CalledProcessError, ValueError):
            pass
        if time.monotonic() >= end: raise AssertionError('timeout: ' + message)
        time.sleep(.2)
def hierarchy():
    shell('uiautomator', 'dump', '/sdcard/review-probe.xml')
    return ET.fromstring(adb('exec-out', 'cat', '/sdcard/review-probe.xml'))
def find(tag, tree=None):
    for item in (tree if tree is not None else hierarchy()).iter('node'):
        rid = item.attrib.get('resource-id', '')
        if rid == tag or rid.endswith(':id/' + tag): return item
    return None
def node(tag, scroll=False):
    for attempt in range(10):
        item = find(tag)
        if item is not None: return item
        if scroll and attempt % 2: shell('input', 'swipe', '160', '520', '160', '220', '200')
        else: time.sleep(.3)
    raise AssertionError('UI node missing: ' + tag)
def center(item):
    x1, y1, x2, y2 = map(int, re.findall(r'\d+', item.attrib['bounds']))
    return (x1 + x2) // 2, (y1 + y2) // 2
def tap(tag, scroll=False):
    item = node(tag, scroll)
    for _ in range(10):
        if item.attrib.get('enabled') == 'true': break
        time.sleep(.3); item = node(tag, scroll)
    assert item.attrib.get('enabled') == 'true', 'UI node disabled: ' + tag
    x, y = center(item); shell('input', 'tap', str(x), str(y)); time.sleep(.2)
def texts(tree): return [n.attrib.get('text') for n in tree.iter('node') if n.attrib.get('text')]

class Proxy(http.server.ThreadingHTTPServer):
    daemon_threads = True
    offline = False
    hold_setup_s = 0.0      # forward a create/join, then keep the response back this long
    expire_room = False
    drop_leave = False      # forward leave, lose its response, then go offline
    calls = []
class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_): pass
    def do_POST(self):
        body = self.rfile.read(int(self.headers.get('Content-Length', '0')))
        path = re.sub(r'/rooms/[A-Z2-9]{6}/', '/rooms/<ROOM>/', self.path)
        entry = {'path': path, 'hash': hashlib.sha256(body).hexdigest()[:12], 't_in': round(time.monotonic() - T0, 2)}
        self.server.calls.append(entry)
        if self.server.offline:
            entry['status'] = 'offline(dropped)'; self.close_connection = True; return
        try:
            request = urllib.request.Request(UPSTREAM + self.path, body, {'Content-Type': 'application/json'}, method='POST')
            try:
                with urllib.request.urlopen(request, timeout=10) as response: status, content = response.status, response.read(131073)
            except urllib.error.HTTPError as error: status, content = error.code, error.read(131073)
            entry['status'] = status
            if self.server.expire_room and status == 200 and self.path.endswith('/state'):
                snap = json.loads(content)
                snap.update(phase='closed', result=None, server_ms=snap['server_ms']+600_000, event_order=snap['event_order']+1)
                snap['own'].update(cells=[], givens=[])
                content = json.dumps(snap).encode()
            if status >= 400: entry['error'] = json.loads(content).get('error')
            if status == 200 and (self.path == '/v1/rooms' or self.path.endswith('/join')):
                entry['room'] = json.loads(content)['snapshot']['room_code']
                if self.server.hold_setup_s:
                    hold, self.server.hold_setup_s = self.server.hold_setup_s, 0.0
                    time.sleep(hold); entry['held_s'] = hold
            if self.server.drop_leave and self.path.endswith('/leave') and status == 200:
                self.server.drop_leave = False; self.server.offline = True
                entry['response'] = 'dropped'; self.close_connection = True; return
            self.send_response(status); self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(content))); self.end_headers(); self.wfile.write(content)
            entry['t_out'] = round(time.monotonic() - T0, 2)
        except OSError:
            entry['t_out'] = round(time.monotonic() - T0, 2); entry['response'] = 'client had closed the connection'
            self.close_connection = True

def creates(proxy): return [c for c in proxy.calls if c['path'] == '/v1/rooms']

def scenario_a(proxy, label):
    say(f'--- A ({label}): create in flight -> system BACK to Home -> reopen friend screen')
    tap('friend-start', True)
    tap('mode-race', True); tap('board-size-6', True)
    x, y = center(node('room-create', True))
    before = len(creates(proxy))
    proxy.hold_setup_s = 3.5        # below Android's 4 s read timeout: the request is simply slow
    shell(f'input tap {x} {y}; input keyevent KEYCODE_BACK')
    t_back = round(time.monotonic() - T0, 2)
    until(lambda: len(creates(proxy)) > before, 5, 'create never reached the proxy')
    first = creates(proxy)[before]
    until(lambda: 't_out' in first, 8, 'held response never finished')
    say('create reached server:', {k: first[k] for k in first if k != 'room'}, '| BACK injected by', t_back)
    assert t_back < first['t_out'], 'BACK landed after the response; scenario not exercised'
    assert find('friend-start') is not None, 'not on the home screen after BACK'
    say('on home screen; saved client flags:', flags())
    assert flags()['setup_pending'], 'no pending setup was saved'
    tap('friend-start', True)
    until(lambda: flags()['phase'] == 'lobby' and not flags()['voluntarily_left'], 20, 'Home/open must recover setup without backgrounding')
    replay = creates(proxy)[before:]
    assert len(replay) >= 2 and all(c['hash'] == first['hash'] for c in replay), 'setup retry body changed'
    assert flags()['room'] == first['room'], 'retry created a ghost seat'
    say('PASS setup Home/open recovered identical request and room:', label)
    return True

def leave(proxy):
    tap('back'); tap('leave-confirm')
    until(lambda: flags()['voluntarily_left'], 15, 'leave not acknowledged')
    node('friend-start', True)

def scenario_c(proxy, cold=False):
    say('--- C: leave reaches server, response lost, app backgrounded and reopened')
    assert flags()['phase'] == 'lobby'
    mark = len(proxy.calls)
    proxy.drop_leave = True
    tap('back'); tap('leave-confirm')
    until(lambda: any(c.get('response') == 'dropped' for c in proxy.calls[mark:]), 10, 'leave never reached the proxy')
    time.sleep(1.0)
    if cold:
        assert saved().get('leave'), 'pending leave must be durable before process death'
        shell('am', 'force-stop', PKG)
    else:
        shell('input', 'keyevent', 'KEYCODE_HOME'); time.sleep(1.5)
    proxy.offline = False
    launch()
    until(lambda: flags()['voluntarily_left'], 20, 'leave must retry after foregrounding')
    node('friend-start', True)
    calls = proxy.calls[mark:]
    leaves = [c for c in calls if c['path'].endswith('/leave')]
    assert len(leaves) >= 2 and all(c['hash'] == leaves[0]['hash'] for c in leaves)
    assert not any(c['path'].endswith('/resume') for c in calls), 'departed seat was resumed'
    say('PASS lost leave recovered to Home without resume:', 'process death' if cold else 'background')
    return True

def post(path, body):
    req = urllib.request.Request(UPSTREAM + path, json.dumps(body).encode(), {'Content-Type': 'application/json'})
    with urllib.request.urlopen(req, timeout=10) as reply: return json.load(reply)

def expiry_result(proxy, mode):
    tap('friend-start', True)
    tap('mode-'+mode, True); tap('board-size-6', True); tap('room-create', True)
    until(lambda: not flags()['voluntarily_left'] and flags()['phase'] == 'lobby')
    code = flags()['room']
    guest = post('/v1/rooms/'+code+'/join', {'protocol':1, 'name':'Guest'})
    def command(kind, **extra):
        snap = guest['snapshot']
        result = post('/v1/rooms/'+code+'/command', {'protocol':1, 'token':guest['token'],
            'generation':snap['generation'], 'match_id':snap['match_id'], 'sequence':snap['own']['next_sequence'],
            'command':dict(type=kind, **extra)})
        guest['snapshot'] = result['snapshot']
    tap('ready-toggle', True)
    command('ready', ready=True)
    until(lambda: flags()['phase'] == 'playing', 15)
    command('resign')
    until(lambda: flags()['phase'] == 'finished')
    known = saved()['auth']['snapshot']
    heading = node('result-heading').attrib.get('text')
    proxy.expire_room = True
    until(lambda: flags()['session_lost'], 15, 'controlled 10-minute expiry not received')
    retained = saved()['auth']['snapshot']
    assert retained['phase'] == 'finished' and retained['result'] == known['result'], 'verified result was replaced by closure'
    assert retained['match_id'] == known['match_id'] and retained['own']['cells'] == known['own']['cells']
    assert node('result-heading').attrib.get('text') == heading
    assert node('rematch').attrib.get('enabled') == 'false'
    shell('am', 'force-stop', PKG); launch()
    assert node('result-heading').attrib.get('text') == heading, 'cold restore lost verified result'
    proxy.expire_room = False
    def backups():
        return {name for name in adb('exec-out', 'run-as', PKG, 'ls', 'files').decode().splitlines()
                if name.startswith('live-session-recovery-') and name.endswith('.json')}
    before = backups()
    tap('live-recover', True)
    original = adb('exec-out', 'run-as', PKG, 'cat', 'files/live-session.json')
    tap('live-reset-confirm', True)
    node('room-create', True)  # Confirmation completes only after atomic preservation and reset.
    copies = backups() - before
    assert len(copies) == 1, 'expected one committed result backup; got '+str(len(copies))
    copied = adb('exec-out', 'run-as', PKG, 'cat', 'files/'+copies.pop())
    assert copied == original, 'reset backup differs from the record present at confirmation'
    tap('room-create', True)
    until(lambda: flags()['phase'] == 'lobby' and flags()['room'] != code and not flags()['voluntarily_left'], 15)
    leave(proxy)
    say('PASS verified result survives expiry/cold restore; UI preserves record and creates a new room:', mode)

assert shell('getprop', 'ro.kernel.qemu') == '1', 'Only run on a disposable emulator.'
with socket.socket() as probe: assert probe.connect_ex(('127.0.0.1', args.server_port)) != 0, 'server port in use'
server = subprocess.Popen([args.server_bin], env=dict(os.environ, SUDOKU_PORT=str(args.server_port)),
                          stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
proxy = None
results = {}
try:
    until(lambda: socket.socket().connect_ex(('127.0.0.1', args.server_port)) == 0, 10, 'server start')
    proxy = Proxy(('127.0.0.1', args.proxy_port), Handler)
    threading.Thread(target=proxy.serve_forever, daemon=True).start()
    shell('am', 'force-stop', PKG)
    shell('run-as', PKG, 'rm', '-f', 'files/live-session.json')
    launch()
    # Control: a held (slow) create alone is fine.
    say('--- control: slow create (3.5 s) without leaving the screen')
    tap('friend-start', True); tap('mode-race', True); tap('board-size-6', True)
    proxy.hold_setup_s = 3.5
    tap('room-create', True)
    until(lambda: flags()['phase'] == 'lobby', 15, 'control create failed')
    say('control OK: lobby reached, create requests:', len(creates(proxy)))
    leave(proxy)
    results['A_after_leave'] = scenario_a(proxy, 'after leaving a room')
    results['C'] = scenario_c(proxy)
    # Fresh state (no previous session at all): reset through the UI, then repeat A.
    shell('am', 'force-stop', PKG)
    shell('run-as', PKG, 'rm', '-f', 'files/live-session.json')
    launch()
    results['A_fresh'] = scenario_a(proxy, 'fresh client')
    results['C_cold'] = scenario_c(proxy, cold=True)
    for mode in ['race', 'duel']: expiry_result(proxy, mode)
    assert all(results.values())
finally:
    say('SUMMARY', json.dumps(results))
    if proxy: proxy.shutdown(); proxy.server_close()
    server.terminate()
    try: server.wait(timeout=10)
    except subprocess.TimeoutExpired: server.kill()
