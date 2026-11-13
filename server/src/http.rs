use crate::{CommandResponse, Envelope, Error, Mode, Session, SharedService, Snapshot, PROTOCOL};
use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
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
fn now(clock: &std::time::Instant) -> u64 {
    clock.elapsed().as_millis().min(u64::MAX as u128) as u64
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
    let mut state = shared.lock().map_err(|_| Error::Internal)?;
    let at = now(&state.1);
    Ok(Json(state.0.create(
        &peer.ip().to_string(),
        body.guest,
        body.name,
        body.mode,
        body.size,
        at,
    )?))
}
async fn join(
    State(shared): State<SharedService>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(code): Path<String>,
    Json(body): Json<JoinRequest>,
) -> Result<Json<Session>, ApiError> {
    version(body.protocol)?;
    let mut state = shared.lock().map_err(|_| Error::Internal)?;
    let at = now(&state.1);
    Ok(Json(state.0.join(
        &peer.ip().to_string(),
        &code,
        body.guest,
        body.name,
        at,
    )?))
}
async fn resume(
    State(shared): State<SharedService>,
    Path(code): Path<String>,
    Json(body): Json<ResumeRequest>,
) -> Result<Json<Snapshot>, ApiError> {
    version(body.protocol)?;
    let mut state = shared.lock().map_err(|_| Error::Internal)?;
    let at = now(&state.1);
    Ok(Json(state.0.resume(&code, &body.token, at)?))
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
    let mut state = shared.lock().map_err(|_| Error::Internal)?;
    let at = now(&state.1);
    state.0.leave(&code, &body.token, at)?;
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
    let mut state = shared.lock().map_err(|_| Error::Internal)?;
    let at = now(&state.1);
    Ok(Json(state.0.state(
        &code,
        &body.token,
        body.generation,
        at,
    )?))
}
async fn command(
    State(shared): State<SharedService>,
    Path(code): Path<String>,
    Json(body): Json<Envelope>,
) -> Result<Json<CommandResponse>, ApiError> {
    let mut state = shared.lock().map_err(|_| Error::Internal)?;
    let at = now(&state.1);
    Ok(Json(state.0.command(&code, &body, at)?))
}

pub fn router(shared: SharedService) -> Router {
    Router::new()
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
