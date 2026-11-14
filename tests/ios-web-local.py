#!/usr/bin/env python3
"""Manual iOS ↔ browser acceptance harness; no simulator/browser automation.

Run from any directory after building sudoku-server and WASM. Web uses :3141;
the Debug simulator app uses :3000 through a fault proxy. Existing listeners
are never replaced. POST /__harness/offline {"offline":true|false} to :3000
disconnects only the iOS seat; GET /__harness/calls returns body hashes only.
Ctrl-C stops only the server this process started. All match data is local.
"""
import hashlib
import http.server
import json
import os
from pathlib import Path
import socket
import subprocess
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
UPSTREAM = 'http://127.0.0.1:3141'


class Proxy(http.server.ThreadingHTTPServer):
    daemon_threads = True
    offline = False
    calls = []


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def reply(self, status, data, mime='application/json'):
        self.send_response(status)
        self.send_header('Content-Type', mime)
        self.send_header('Content-Length', str(len(data)))
        self.send_header('Cache-Control', 'no-store')
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path == '/__harness/calls':
            return self.reply(200, json.dumps(self.server.calls).encode())
        self.forward()

    def do_POST(self):
        body = self.rfile.read(int(self.headers.get('Content-Length', '0')))
        if self.path == '/__harness/offline':
            self.server.offline = bool(json.loads(body)['offline'])
            return self.reply(200, b'{"ok":true}')
        if self.server.offline:
            self.close_connection = True
            return
        command = (json.loads(body).get('command') or {}).get('type')
        self.server.calls.append({'path': self.path, 'type': command,
                                  'hash': hashlib.sha256(body).hexdigest()})
        self.forward(body)

    def forward(self, body=None):
        request = urllib.request.Request(UPSTREAM + self.path, body,
                                         {'Content-Type': 'application/json'})
        try:
            try:
                response = urllib.request.urlopen(request, timeout=10)
            except urllib.error.HTTPError as error:
                response = error
            with response:
                self.reply(response.code, response.read(), response.headers.get_content_type())
        except OSError:
            self.close_connection = True


def main():
    # Reserve the proxy before starting anything; refuse both occupied ports.
    proxy = Proxy(('127.0.0.1', 3000), Handler)
    process = None
    try:
        with socket.socket() as probe:
            probe.bind(('127.0.0.1', 3141))
        process = subprocess.Popen([str(ROOT / 'target/debug/sudoku-server')],
                                   env=dict(os.environ, SUDOKU_PORT='3141'))
        end = time.monotonic() + 10
        while True:
            assert process.poll() is None, 'Owned server exited'
            try:
                with urllib.request.urlopen(UPSTREAM + '/health', timeout=1) as response:
                    assert json.load(response)['status'] == 'ok'
                break
            except OSError:
                assert time.monotonic() < end, 'Owned server did not start'
                time.sleep(.05)
        print('Web: http://127.0.0.1:3141 | iOS Debug/fault proxy: http://127.0.0.1:3000', flush=True)
        proxy.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        proxy.server_close()
        if process and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


if __name__ == '__main__':
    main()
