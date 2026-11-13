use crate::{CommandResponse, Envelope, Error, Mode, Session, SharedService, Snapshot, PROTOCOL};
use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Path, State},
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateRequest {
    protocol: u8,
    guest: Option<String>,
    name: String,
    mode: Mode,
    size: u8,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JoinRequest {
    protocol: u8,
    guest: Option<String>,
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResumeRequest {
    protocol: u8,
    token: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StateRequest {
    protocol: u8,
    token: String,
    generation: u64,
}
#[derive(Deserialize)]
struct Board {
    cells: Vec<u8>,
    size: u8,
}
#[derive(Serialize)]
struct ResultBody {
    valid: bool,
}
#[derive(Serialize)]
struct ErrorBody {
    protocol: u8,
    error: Error,
}

struct ApiError(Error);
impl From<Error> for ApiError {
    fn from(error: Error) -> Self {
        Self(error)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::RoomMissing => StatusCode::NOT_FOUND,
            Error::RoomClosed => StatusCode::GONE,
            Error::RateLimited | Error::Capacity => StatusCode::TOO_MANY_REQUESTS,
            Error::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::CONFLICT,
        };
        (
            status,
            Json(ErrorBody {
                protocol: PROTOCOL,
                error: self.0,
            }),
        )
            .into_response()
    }
}
fn version(protocol: u8) -> Result<(), Error> {
    if protocol == PROTOCOL {
        Ok(())
    } else {
        Err(Error::Protocol)
    }
}

async fn validate(Json(board): Json<Board>) -> Json<ResultBody> {
    Json(ResultBody {
        valid: sudoku_core::validate_board(&board.cells, board.size),
    })
}
async fn create(
    State(shared): State<SharedService>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(body): Json<CreateRequest>,
) -> Result<Json<Session>, ApiError> {
    version(body.protocol)?;
    let peer = peer.ip().to_string();
    Ok(Json(
        shared
            .call(move |state, at| {
                state.create(&peer, body.guest, body.name, body.mode, body.size, at)
            })
            .await?,
    ))
}
async fn join(
    State(shared): State<SharedService>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(code): Path<String>,
    Json(body): Json<JoinRequest>,
) -> Result<Json<Session>, ApiError> {
    version(body.protocol)?;
    let peer = peer.ip().to_string();
    Ok(Json(
        shared
            .call(move |state, at| state.join(&peer, &code, body.guest, body.name, at))
            .await?,
    ))
}
async fn resume(
    State(shared): State<SharedService>,
    Path(code): Path<String>,
    Json(body): Json<ResumeRequest>,
) -> Result<Json<Snapshot>, ApiError> {
    version(body.protocol)?;
    Ok(Json(
        shared
            .call(move |state, at| state.resume(&code, &body.token, at))
            .await?,
    ))
}
#[derive(Serialize)]
struct LeaveResponse {
    protocol: u8,
    left: bool,
}
async fn leave(
    State(shared): State<SharedService>,
    Path(code): Path<String>,
    Json(body): Json<ResumeRequest>,
) -> Result<Json<LeaveResponse>, ApiError> {
    version(body.protocol)?;
    shared
        .call(move |state, at| state.leave(&code, &body.token, at))
        .await?;
    Ok(Json(LeaveResponse {
        protocol: PROTOCOL,
        left: true,
    }))
}

async fn state(
    State(shared): State<SharedService>,
    Path(code): Path<String>,
    Json(body): Json<StateRequest>,
) -> Result<Json<Snapshot>, ApiError> {
    version(body.protocol)?;
    Ok(Json(
        shared
            .call(move |state, at| state.state(&code, &body.token, body.generation, at))
            .await?,
    ))
}
async fn command(
    State(shared): State<SharedService>,
    Path(code): Path<String>,
    Json(body): Json<Envelope>,
) -> Result<Json<CommandResponse>, ApiError> {
    Ok(Json(
        shared
            .call(move |state, at| state.command(&code, &body, at))
            .await?,
    ))
}

// Only known public build assets can be read; no request path is joined onto
// the filesystem. Credentials, saves, sources and arbitrary files stay private.
async fn web_asset(uri: Uri) -> Response {
    let path = uri.path();
    let invite = path
        .strip_prefix("/r/")
        .is_some_and(|code| code.len() == 6 && code.bytes().all(|c| c.is_ascii_alphanumeric()));
    let (file, mime) = match path {
        "/" | "/index.html" => ("index.html", "text/html; charset=utf-8"),
        _ if invite => ("index.html", "text/html; charset=utf-8"),
        "/main.js" => ("main.js", "text/javascript; charset=utf-8"),
        "/session.js" => ("session.js", "text/javascript; charset=utf-8"),
        "/live.js" => ("live.js", "text/javascript; charset=utf-8"),
        "/styles.css" => ("styles.css", "text/css; charset=utf-8"),
        "/sw.js" => ("sw.js", "text/javascript; charset=utf-8"),
        "/manifest.webmanifest" => ("manifest.webmanifest", "application/manifest+json"),
        "/icon.svg" => ("icon.svg", "image/svg+xml"),
        "/pkg/sudoku_wasm.js" => ("pkg/sudoku_wasm.js", "text/javascript; charset=utf-8"),
        "/pkg/sudoku_wasm_bg.wasm" => ("pkg/sudoku_wasm_bg.wasm", "application/wasm"),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../apps/web");
    match tokio::task::spawn_blocking(move || std::fs::read(root.join(file))).await {
        Ok(Ok(bytes)) => (
            [
                ("content-type", mime),
                ("cache-control", "no-cache"),
                ("x-content-type-options", "nosniff"),
            ],
            bytes,
        )
            .into_response(),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Web uygulaması şu anda yüklenemiyor.",
        )
            .into_response(),
    }
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    protocol: u8,
}
async fn health(State(shared): State<SharedService>) -> Result<Json<Health>, ApiError> {
    shared.call(|_, _| Ok(())).await?;
    Ok(Json(Health {
        status: "ok",
        protocol: PROTOCOL,
    }))
}

pub fn router(shared: SharedService) -> Router {
    Router::new()
        .route("/health", get(health))
        .fallback(get(web_asset))
        .route("/validate", post(validate))
        .route("/v1/rooms", post(create))
        .route("/v1/rooms/{code}/join", post(join))
        .route("/v1/rooms/{code}/resume", post(resume))
        .route("/v1/rooms/{code}/leave", post(leave))
        .route("/v1/rooms/{code}/state", post(state))
        .route("/v1/rooms/{code}/command", post(command))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(shared)
}
