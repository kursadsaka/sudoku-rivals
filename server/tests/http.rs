use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

struct Server {
    address: SocketAddr,
    stop: Option<mpsc::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn start() -> Self {
        Self::start_with_config(sudoku_server::http::Config::default())
    }
    fn start_with_config(config: sudoku_server::http::Config) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, rx) = mpsc::channel();
        let thread = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                let app = sudoku_server::http::router_with_config(
                    sudoku_server::shared_service(),
                    config,
                );
                axum::serve(
                    listener,
                    app.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .with_graceful_shutdown(async move {
                    let _ = tokio::task::spawn_blocking(move || rx.recv()).await;
                })
                .await
                .unwrap();
            });
        });
        Self {
            address,
            stop: Some(stop),
            thread: Some(thread),
        }
    }
    fn get(&self, path: &str) -> (u16, String, String) {
        let mut socket = TcpStream::connect(self.address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(
            socket,
            "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.address
        )
        .unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).unwrap();
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        let status = headers
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        (status, headers.to_owned(), body.to_owned())
    }
    fn post(&self, path: &str, body: Value) -> (u16, Value) {
        self.post_with_headers(path, body, "")
    }
    fn post_with_headers(&self, path: &str, body: Value, headers: &str) -> (u16, Value) {
        let body = body.to_string();
        let mut socket = TcpStream::connect(self.address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let request = format!("POST {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{headers}Connection: close\r\n\r\n{}", self.address, body.len(), body);
        socket.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).unwrap();
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        let status = headers
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        (status, serde_json::from_str(body).unwrap())
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.take().unwrap().send(()).unwrap();
        self.thread.take().unwrap().join().unwrap();
    }
}
fn command(
    server: &Server,
    code: &str,
    token: &Value,
    snapshot: &Value,
    command: Value,
) -> (u16, Value) {
    server.post(&format!("/v1/rooms/{code}/command"), json!({"protocol":1,"token":token,"generation":snapshot["generation"],"match_id":snapshot["match_id"],"sequence":snapshot["own"]["next_sequence"],"command":command}))
}

#[test]
fn real_http_bot_race_auth_versioning_and_snapshot_privacy() {
    let server = Server::start();
    let (status, board) = server.post("/validate", json!({"size":6,"cells":vec![0;36]}));
    assert_eq!(status, 200);
    assert_eq!(board["valid"], true);
    let (status, _) = server.post(
        "/v1/rooms",
        json!({"protocol":2,"name":"A","mode":"race","size":6}),
    );
    assert_eq!(status, 409);
    let (status, a) = server.post(
        "/v1/rooms",
        json!({"protocol":1,"name":"A","mode":"race","size":6}),
    );
    assert_eq!(status, 200);
    let code = a["snapshot"]["room_code"].as_str().unwrap();
    let (status, b) = server.post(
        &format!("/v1/rooms/{code}/join"),
        json!({"protocol":1,"name":"B"}),
    );
    assert_eq!(status, 200);
    let (status, _) = server.post(
        &format!("/v1/rooms/{code}/state"),
        json!({"protocol":1,"token":"bad","generation":1}),
    );
    assert_eq!(status, 401);
    let (status, ready_a) = command(
        &server,
        code,
        &a["token"],
        &a["snapshot"],
        json!({"type":"ready","ready":true}),
    );
    assert_eq!(status, 200);
    let (status, ready_b) = command(
        &server,
        code,
        &b["token"],
        &b["snapshot"],
        json!({"type":"ready","ready":true}),
    );
    assert_eq!(status, 200);
    assert_eq!(ready_b["snapshot"]["phase"], "countdown");
    thread::sleep(Duration::from_millis(3010));
    let (_, state) = server.post(
        &format!("/v1/rooms/{code}/state"),
        json!({"protocol":1,"token":a["token"],"generation":1}),
    );
    assert_eq!(state["phase"], "playing");
    let cells: Vec<u8> = serde_json::from_value(state["own"]["givens"].clone()).unwrap();
    let solved = sudoku_core::solve_board(&cells, 6).unwrap().unwrap();
    let (status, finish) = command(
        &server,
        code,
        &a["token"],
        &state,
        json!({"type":"sync","base_version":0,"cells":solved}),
    );
    assert_eq!(status, 200);
    assert_eq!(finish["snapshot"]["result"]["winner"], 0);
    assert_eq!(finish["snapshot"]["result"]["reason"], "completed");
    let opponent = finish["snapshot"]["opponent"].as_object().unwrap();
    for field in [
        "solution", "cells", "givens", "token", "guest", "combo", "seed",
    ] {
        assert!(!opponent.contains_key(field));
    }
    assert!(!finish.to_string().contains(b["token"].as_str().unwrap()));
    let (_, resumed) = server.post(
        &format!("/v1/rooms/{code}/resume"),
        json!({"protocol":1,"token":a["token"]}),
    );
    assert_eq!(resumed["generation"], 2);
    let (status, _) = command(
        &server,
        code,
        &a["token"],
        &ready_a["snapshot"],
        json!({"type":"resign"}),
    );
    assert_eq!(status, 409);
    for _ in 0..2 {
        let (status, left) = server.post(
            &format!("/v1/rooms/{code}/leave"),
            json!({"protocol":1,"token":a["token"]}),
        );
        assert_eq!(status, 200);
        assert_eq!(left, json!({"protocol":1,"left":true}));
    }
    let (status, _) = server.post(
        &format!("/v1/rooms/{code}/state"),
        json!({"protocol":1,"token":a["token"],"generation":2}),
    );
    assert_eq!(status, 401);
}

#[test]
fn local_http_poll_load_is_bounded_and_recorded() {
    let server = Server::start();
    let (_, session) = server.post(
        "/v1/rooms",
        json!({"protocol":1,"name":"Load","mode":"duel","size":9}),
    );
    let code = session["snapshot"]["room_code"].as_str().unwrap();
    let started = Instant::now();
    for _ in 0..1000 {
        let (status, response) = server.post(
            &format!("/v1/rooms/{code}/state"),
            json!({"protocol":1,"token":session["token"],"generation":1}),
        );
        assert_eq!(status, 200);
        assert_eq!(response["phase"], "lobby");
    }
    eprintln!(
        "G06 local load: 1000 sequential authenticated HTTP snapshots in {:?}",
        started.elapsed()
    );
}

#[test]
fn web_and_invites_use_one_origin_without_exposing_repository_files() {
    let server = Server::start();
    let (status, _, health) = server.get("/health");
    assert_eq!(status, 200);
    assert_eq!(
        serde_json::from_str::<Value>(&health).unwrap(),
        json!({"protocol":1,"status":"ok"})
    );
    let (status, headers, html) = server.get("/");
    assert_eq!(status, 200);
    assert!(headers
        .to_ascii_lowercase()
        .contains("content-type: text/html"));
    assert!(headers
        .to_ascii_lowercase()
        .contains("x-content-type-options: nosniff"));
    assert!(html.contains("Sudoku Rivals"));
    let (status, _, invite) = server.get("/r/ABC234");
    assert_eq!(status, 200);
    assert_eq!(html, invite);
    assert_eq!(server.get("/r/abc234").0, 200);
    for path in [
        "/r/ABC012",
        "/r/ABC234/",
        "/r/ABC234?token=secret",
        "/r/%41BC234",
    ] {
        assert_eq!(server.get(path).0, 404, "{path}");
    }
    let (status, headers, script) = server.get("/main.js");
    assert_eq!(status, 200);
    assert!(headers.to_ascii_lowercase().contains("text/javascript"));
    assert!(script.contains("sudoku_wasm.js"));
    for path in [
        "/.env",
        "/Cargo.toml",
        "/../Cargo.toml",
        "/%2e%2e/Cargo.toml",
        "/pkg/../../core/src/lib.rs",
        "/core/src/lib.rs",
        "/v1/private",
    ] {
        assert_eq!(server.get(path).0, 404, "{path}");
    }
}

#[test]
fn public_ingress_cannot_bypass_shared_quotas_with_forged_ip_headers() {
    let server = Server::start_with_config(sudoku_server::http::Config {
        public_ingress: true,
        ..Default::default()
    });
    for i in 0..6 {
        let (status, body) = server.post_with_headers(
            "/v1/rooms",
            json!({"protocol":1,"name":"Staging","mode":"race","size":6}),
            &format!("X-Forwarded-For: 127.0.0.1, 198.51.100.{i}\r\nX-Real-IP: 203.0.113.{i}\r\nForwarded: for=203.0.113.{i}\r\n"),
        );
        assert_eq!(status, if i < 5 { 200 } else { 429 });
        if i == 5 {
            assert_eq!(body["error"], "rate_limited");
        }
    }
    for i in 0..11 {
        let (status, body) = server.post_with_headers(
            "/v1/rooms/ABC234/join",
            json!({"protocol":1,"name":"Staging"}),
            &format!("X-Forwarded-For: 127.0.0.{i}\r\n"),
        );
        assert_eq!(status, if i < 10 { 404 } else { 429 });
        if i == 10 {
            assert_eq!(body["error"], "rate_limited");
        }
    }
    let (_, headers, _) = server.get("/health");
    assert!(headers
        .to_ascii_lowercase()
        .contains("cache-control: no-store"));
}

#[test]
fn configured_web_root_is_used_without_exposing_other_files() {
    let root = std::env::temp_dir().join(format!(
        "sudoku-web-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("index.html"), "portable web fixture").unwrap();
    std::fs::write(root.join(".env"), "private fixture").unwrap();
    let config = sudoku_server::http::Config {
        web_root: root.clone(),
        public_ingress: false,
    };
    assert!(
        config.validate_assets().is_err(),
        "Incomplete package must fail readiness at startup"
    );
    let server = Server::start_with_config(config);
    assert_eq!(server.get("/r/ABC234").2, "portable web fixture");
    assert_eq!(server.get("/.env").0, 404);
    assert_eq!(
        server.get("/main.js").0,
        503,
        "Never fall back to the build checkout"
    );
    drop(server);
    std::fs::remove_dir_all(root).unwrap();
}
