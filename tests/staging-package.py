"""Verify a relocated staging package using only an owned loopback server."""
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request


repo = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="sudoku-staging-") as temporary:
    root = Path(temporary)
    shutil.copytree(repo / "dist", root / "package")
    package = root / "package"
    # A distinct entry proves the binary uses the relocated root, not its checkout.
    entry = package / "web/index.html"
    entry.write_bytes(entry.read_bytes() + b"\n<!-- relocated-package -->\n")
    env = {**os.environ, "SUDOKU_BIND": "127.0.0.1", "SUDOKU_WEB_ROOT": str(package / "web")}
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    env["PORT"] = str(port)
    env["SUDOKU_PORT"] = "0"  # Render PORT must win over the legacy local setting.
    origin = f"http://127.0.0.1:{port}"
    def get(path):
        try:
            with urllib.request.urlopen(origin + path, timeout=3) as response:
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
        for path in ["/.env", "/Cargo.toml", "/server/src/main.rs", "/r/ABC012", "/r/ABC234?token=x"]:
            assert get(path)[0] == 404, path
        for test in ["wasm.mjs", "web-solo.mjs", "ui-contract.mjs"]:
            subprocess.run(["node", "tests/" + test], cwd=repo, check=True)
        subprocess.run(["node", "tests/web-multiplayer.mjs", "--http"], cwd=repo,
                       env={**os.environ, "SUDOKU_SERVER_URL": origin}, check=True)
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
