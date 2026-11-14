#!/usr/bin/env python3
"""iOS simulator ↔ HTTP ↔ local sudoku-server end-to-end tests (G13).

Owns only a fresh server on --server-port and a fault proxy on --proxy-port;
never touches :3000. The proxy forwards /v1 traffic for the app under test
and exposes /__harness/* controls the XCUITests use (offline, drop one
response, restart server, call log). Request bodies are hashed, never printed.

Usage: python3 tests/ios-multiplayer.py [--skip-build] [-- extra xcodebuild args]
"""
import argparse
import hashlib
import http.server
import json
import os
import pathlib
import socket
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser()
parser.add_argument('--server-bin', default=str(ROOT / 'target/debug/sudoku-server'))
parser.add_argument('--server-port', type=int, default=3101)
parser.add_argument('--proxy-port', type=int, default=3102)
parser.add_argument('--destination', default=os.environ.get('IOS_SIM_DESTINATION', 'platform=iOS Simulator,name=iPhone 18 Pro'))
parser.add_argument('--skip-build', action='store_true')
parser.add_argument('xcodebuild_args', nargs='*')
args = parser.parse_args()
assert 3000 not in (args.server_port, args.proxy_port), 'Never replace the shared dev server.'
UPSTREAM = f'http://127.0.0.1:{args.server_port}'
lock = threading.Lock()
server = None


def port_open(port):
    with socket.socket() as probe:
        return probe.connect_ex(('127.0.0.1', port)) == 0


def server_start():
    assert not port_open(args.server_port), 'Test server port already in use; not replacing it.'
    process = subprocess.Popen([args.server_bin], env=dict(os.environ, SUDOKU_PORT=str(args.server_port)),
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    end = time.monotonic() + 10
    while not port_open(args.server_port):
        assert process.poll() is None and time.monotonic() < end, 'Owned test server did not start'
        time.sleep(.05)
    return process


def server_stop(process):
    if process and process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


class Proxy(http.server.ThreadingHTTPServer):
    daemon_threads = True
    offline = False
    drop_next = None
    calls = []


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def reply(self, status, payload):
        data = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path == '/__harness/calls':
            return self.reply(200, self.server.calls)
        self.reply(404, {})

    def do_POST(self):
        global server
        body = self.rfile.read(int(self.headers.get('Content-Length', '0')))
        if self.path.startswith('/__harness/'):
            control = json.loads(body or b'{}')
            if self.path == '/__harness/offline':
                self.server.offline = bool(control['offline'])
            elif self.path == '/__harness/drop':
                self.server.drop_next = control['type']
            elif self.path == '/__harness/restart':
                with lock:
                    server_stop(server)
                    server = server_start()
            else:
                return self.reply(404, {})
            return self.reply(200, {'ok': True})
        if self.server.offline:
            self.close_connection = True
            return
        try:
            command = (json.loads(body).get('command') or {}).get('type')
        except ValueError:
            command = None
        self.server.calls.append({'path': self.path, 'type': command, 'hash': hashlib.sha256(body).hexdigest()})
        try:
            request = urllib.request.Request(UPSTREAM + self.path, body, {'Content-Type': 'application/json'}, method='POST')
            try:
                with urllib.request.urlopen(request, timeout=10) as response:
                    status, content = response.status, response.read(131073)
            except urllib.error.HTTPError as error:
                status, content = error.code, error.read(131073)
            if self.server.drop_next and command == self.server.drop_next:
                self.server.drop_next = None
                self.close_connection = True
                return
            self.send_response(status)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(content)))
            self.end_headers()
            self.wfile.write(content)
        except (OSError, ValueError):
            self.close_connection = True


def main():
    global server
    if not args.skip_build:
        subprocess.run(['sh', 'scripts/ios-build.sh'], cwd=ROOT, check=True)
        subprocess.run(['cargo', 'build', '--locked', '-p', 'sudoku-server'], cwd=ROOT, check=True)
    proxy = None
    try:
        server = server_start()
        proxy = Proxy(('127.0.0.1', args.proxy_port), Handler)
        threading.Thread(target=proxy.serve_forever, daemon=True).start()
        result = ROOT / 'target/ios-multiplayer.xcresult'
        subprocess.run(['rm', '-rf', str(result)], check=True)
        env = dict(os.environ, TEST_RUNNER_LIVE_TEST_SERVER=UPSTREAM,
                   TEST_RUNNER_LIVE_UI_PROXY=f'http://127.0.0.1:{args.proxy_port}')
        code = subprocess.run(['xcodebuild', '-project', 'SudokuRivals.xcodeproj', '-scheme', 'SudokuRivals',
                               '-destination', args.destination, '-resultBundlePath', str(result),
                               # A stuck UI test fails instead of holding the server/proxy forever.
                               '-test-timeouts-enabled', 'YES', '-default-test-execution-time-allowance', '900',
                               'CODE_SIGNING_ALLOWED=NO', 'test', *args.xcodebuild_args],
                              cwd=ROOT / 'apps/ios', env=env).returncode
        return code
    finally:
        if proxy:
            proxy.shutdown()
            proxy.server_close()
        with lock:
            server_stop(server)


if __name__ == '__main__':
    sys.exit(main())
