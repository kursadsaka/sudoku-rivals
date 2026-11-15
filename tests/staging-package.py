"""Verify a relocated staging package using only an owned loopback server."""
import json
import os
from pathlib import Path
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request


repo = Path(__file__).resolve().parents[1]
public = "--public" in sys.argv
if public:
    assert {name for _, name in socket.if_nameindex()} == {"lo"}, "Public bind test requires an isolated network namespace"
with tempfile.TemporaryDirectory(prefix="sudoku-staging-") as temporary:
    root = Path(temporary)
    shutil.copytree(repo / "dist", root / "package")
    package = root / "package"
    # A distinct entry proves the binary uses the relocated root, not its checkout.
    entry = package / "web/index.html"
    entry.write_bytes(entry.read_bytes() + b"\n<!-- relocated-package -->\n")
    env = {**os.environ, "SUDOKU_BIND": "0.0.0.0" if public else "127.0.0.1", "SUDOKU_WEB_ROOT": str(package / "web")}
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    env["PORT"] = str(port)
    env["SUDOKU_PORT"] = "0"  # Render PORT must win over the legacy local setting.
    origin = f"http://127.0.0.1:{port}"
    def get(path, headers=None):
        try:
            with urllib.request.urlopen(urllib.request.Request(origin + path, headers=headers or {}), timeout=3) as response:
                return response.status, response.headers, response.read()
        except urllib.error.HTTPError as error:
            return error.code, error.headers, error.read()

    process = subprocess.Popen([str(package / "sudoku-server")], cwd=root, env=env)
    try:
        for _ in range(100):
            assert process.poll() is None, "Packaged server exited during startup"
            try:
                if get("/health")[0] == 200:
                    break
            except OSError:
                pass
            time.sleep(0.05)
        else:
            raise AssertionError("Packaged server did not become ready")
        _, headers, health = get("/health")
        assert json.loads(health) == {"status": "ok", "protocol": 1}
        assert headers["Cache-Control"] == "no-store"
        for path in ["/", "/r/ABC234", "/r/abc234"]:
            status, headers, body = get(path)
            assert status == 200 and b"relocated-package" in body
            assert headers["Referrer-Policy"] == "no-referrer"
        status, headers, body = get("/pkg/sudoku_wasm_bg.wasm")
        assert status == 200 and body[:4] == b"\0asm"
        assert headers["Content-Type"] == "application/wasm"
        tag = headers["ETag"]
        status, headers, body = get("/pkg/sudoku_wasm_bg.wasm", {"If-None-Match": tag})
        assert status == 304 and not body and headers["ETag"] == tag
        assert get("/pkg/sudoku_wasm_bg.wasm", {"If-None-Match": '"different-deploy"'})[0] == 200
        # Install icons: every manifest icon and the apple-touch icon are served with the declared type and size.
        status, headers, body = get("/manifest.webmanifest")
        assert status == 200 and headers["Content-Type"] == "application/manifest+json"
        icons = json.loads(body)["icons"]
        assert {icon["purpose"] for icon in icons} == {"any", "maskable"}
        for src, mime, sizes in [(i["src"], i["type"], i["sizes"]) for i in icons] + [("/apple-touch-icon.png", "image/png", "180x180")]:
            status, headers, body = get(src)
            assert status == 200 and headers["Content-Type"] == mime, src
            if mime == "image/png":
                assert body[:8] == b"\x89PNG\r\n\x1a\n" and "%dx%d" % struct.unpack(">II", body[16:24]) == sizes, src
        assert b'rel="apple-touch-icon"' in get("/")[2]
        for path in ["/.env", "/Cargo.toml", "/server/src/main.rs", "/r/ABC012", "/r/ABC234?token=x"]:
            assert get(path)[0] == 404, path
        if public:
            transferred = 0
            for _ in range(256):
                status, _, payload = get("/pkg/sudoku_wasm_bg.wasm")
                if status == 503:
                    break
                assert status == 200
                transferred += len(payload)
            else:
                raise AssertionError("Public static budget did not stop downloads")
            assert transferred < 128 * 1024 * 1024
            assert get("/health")[0] == 200
            print("PASS public static budget exhausted; health and subsequent real-client matches remain available", flush=True)
        for test in ["wasm.mjs", "web-solo.mjs", "ui-contract.mjs"]:
            subprocess.run(["node", "tests/" + test], cwd=repo, check=True)
        subprocess.run(["node", "tests/web-multiplayer.mjs", "--http"], cwd=repo,
                       env={**os.environ, "SUDOKU_SERVER_URL": origin}, check=True)
        if public:
            # The suite created five distinct rooms. Lost-response replays must be free.
            request = urllib.request.Request(origin + "/v1/rooms", method="POST",
                data=json.dumps({"protocol": 1, "name": "Quota", "mode": "race", "size": 6}).encode(),
                headers={"Content-Type": "application/json", "X-Forwarded-For": "127.0.0.1"})
            try:
                urllib.request.urlopen(request, timeout=3).close()
                raise AssertionError("Public bind accidentally used local quota")
            except urllib.error.HTTPError as error:
                assert error.code == 429 and json.loads(error.read())["error"] == "rate_limited"
            print("PASS actual public bind: five distinct rooms + cold retries, then 429 despite forged IP", flush=True)
        print("PASS relocated release package, Render PORT precedence, static privacy, full HTTP match matrix", flush=True)
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
    (package / "web/pkg/sudoku_wasm_bg.wasm").unlink()
    missing = subprocess.run([str(package / "sudoku-server")], cwd=root, env=env,
                             capture_output=True, timeout=5)
    assert missing.returncode != 0, "Incomplete package must refuse startup"
    print("PASS missing packaged WASM refuses startup; owned server stopped", flush=True)
