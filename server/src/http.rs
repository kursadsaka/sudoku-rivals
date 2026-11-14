use crate::{CommandResponse, Envelope, Error, Mode, Session, SharedService, Snapshot, PROTOCOL};
use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

const PUBLIC_STATIC_BYTES: u64 = 128 * 1024 * 1024;

struct Asset {
    bytes: axum::body::Bytes,
    etag: String,
}
struct StaticAssets {
    files: HashMap<&'static str, Asset>,
    remaining: AtomicU64,
}
impl StaticAssets {
    fn load(config: &Config) -> Self {
        let files = WEB_ASSETS
            .iter()
            .filter_map(|&(file, _)| {
                let bytes = std::fs::read(config.web_root.join(file)).ok()?;
                let mut hash = std::collections::hash_map::DefaultHasher::new();
                bytes.hash(&mut hash);
                Some((
                    file,
                    Asset {
                        etag: format!("W/\"{:x}-{}\"", hash.finish(), bytes.len()),
                        bytes: bytes.into(),
                    },
                ))
            })
            .collect();
        Self {
            files,
            remaining: AtomicU64::new(if config.public_ingress {
                PUBLIC_STATIC_BYTES
            } else {
                u64::MAX
            }),
        }
    }

    fn reserve(&self, bytes: u64) -> bool {
        self.remaining
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |remaining| {
                remaining.checked_sub(bytes)
            })
            .is_ok()
    }
}

const WEB_ASSETS: &[(&str, &str)] = &[
    ("index.html", "text/html; charset=utf-8"),
    ("main.js", "text/javascript; charset=utf-8"),
    ("session.js", "text/javascript; charset=utf-8"),
    ("live.js", "text/javascript; charset=utf-8"),
    ("styles.css", "text/css; charset=utf-8"),
    ("sw.js", "text/javascript; charset=utf-8"),
    ("manifest.webmanifest", "application/manifest+json"),
    ("icon.svg", "image/svg+xml"),
    ("pkg/sudoku_wasm.js", "text/javascript; charset=utf-8"),
    ("pkg/sudoku_wasm_bg.wasm", "application/wasm"),
];

#[derive(Clone)]
pub struct Config {
    pub web_root: std::path::PathBuf,
    pub public_ingress: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            web_root: std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../apps/web"),
            public_ingress: false,
        }
    }
}
impl Config {
    pub fn validate_assets(&self) -> std::io::Result<()> {
        for (file, _) in WEB_ASSETS {
            let path = self.web_root.join(file);
            if !std::fs::metadata(&path)?.is_file() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Not a web asset: {}", path.display()),
                ));
            }
            std::fs::File::open(path)?;
        }
        Ok(())
    }

    fn quota_peer(&self, peer: SocketAddr) -> String {
        // shortcut: shared staging quota until the ingress IP-header contract is verified.
        // Never trust forwarded headers or grant a proxy the local-test quota.
        if self.public_ingress {
            crate::PUBLIC_INGRESS.into()
        } else {
            peer.ip().to_string()
        }
    }
}

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
    Extension(config): Extension<Config>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(body): Json<CreateRequest>,
) -> Result<Json<Session>, ApiError> {
    version(body.protocol)?;
    let peer = config.quota_peer(peer);
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
    Extension(config): Extension<Config>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(code): Path<String>,
    Json(body): Json<JoinRequest>,
) -> Result<Json<Session>, ApiError> {
    version(body.protocol)?;
    let peer = config.quota_peer(peer);
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
async fn web_asset(
    Extension(assets): Extension<Arc<StaticAssets>>,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let path = uri.path();
    let invite = path.starts_with("/r/")
        && uri.query().is_none()
        && sudoku_core::multiplayer::parse_invite(path).is_ok();
    let file = if path == "/" || invite {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };
    let Some(&(file, mime)) = WEB_ASSETS.iter().find(|(asset, _)| *asset == file) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(asset) = assets.files.get(file) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Web uygulaması şu anda yüklenemiyor.",
        )
            .into_response();
    };
    let unchanged = headers
        .get_all("if-none-match")
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|h| h.split(','))
        .any(|tag| {
            let tag = tag.trim();
            tag == "*" || tag.trim_start_matches("W/") == asset.etag.trim_start_matches("W/")
        });
    // Reserve conservatively for headers too; concurrent downloads cannot overspend.
    let cost = 1024
        + if unchanged {
            0
        } else {
            asset.bytes.len() as u64
        };
    if !assets.reserve(cost) {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            [("retry-after", "3600")],
            "Staging static transfer budget exhausted.",
        )
            .into_response();
    }
    let mut response = if unchanged {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        asset.bytes.clone().into_response()
    };
    for (name, value) in [
        ("content-type", mime),
        ("cache-control", "no-cache"),
        ("x-content-type-options", "nosniff"),
        ("etag", asset.etag.as_str()),
    ] {
        response.headers_mut().insert(name, value.parse().unwrap());
    }
    response
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
    router_with_config(shared, Config::default())
}

pub fn router_with_config(shared: SharedService, config: Config) -> Router {
    let assets = Arc::new(StaticAssets::load(&config));
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
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response
                    .headers_mut()
                    .entry("cache-control")
                    .or_insert(axum::http::HeaderValue::from_static("no-store"));
                response.headers_mut().insert(
                    "referrer-policy",
                    axum::http::HeaderValue::from_static("no-referrer"),
                );
                response
            },
        ))
        .layer(Extension(config))
        .layer(Extension(assets))
        .with_state(shared)
}

#[cfg(test)]
mod static_tests {
    use super::*;

    #[tokio::test]
    async fn conditional_static_requests_are_empty_and_budget_exhaustion_preserves_api() {
        let assets = Arc::new(StaticAssets {
            files: HashMap::from([(
                "index.html",
                Asset {
                    bytes: axum::body::Bytes::from_static(b"app"),
                    etag: "W/\"app-v1\"".into(),
                },
            )]),
            remaining: AtomicU64::new(2051),
        });
        let first = web_asset(
            Extension(assets.clone()),
            "/".parse().unwrap(),
            HeaderMap::new(),
        )
        .await;
        assert_eq!(first.status(), StatusCode::OK);
        let tag = first.headers()["etag"].clone();
        let mut headers = HeaderMap::new();
        headers.insert(
            "if-none-match",
            format!(
                "\"old\", {}",
                tag.to_str().unwrap().trim_start_matches("W/")
            )
            .parse()
            .unwrap(),
        );
        let cached = web_asset(
            Extension(assets.clone()),
            "/r/ABC234".parse().unwrap(),
            headers,
        )
        .await;
        assert_eq!(cached.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(cached.headers()["etag"], tag);
        assert!(axum::body::to_bytes(cached.into_body(), 1024)
            .await
            .unwrap()
            .is_empty());
        let exhausted = web_asset(
            Extension(assets.clone()),
            "/".parse().unwrap(),
            HeaderMap::new(),
        )
        .await;
        assert_eq!(exhausted.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(assets.remaining.load(Ordering::Relaxed), 0);
        assert!(health(State(crate::shared_service())).await.is_ok());
    }

    #[test]
    fn parallel_downloads_cannot_overspend_static_budget() {
        let assets = Arc::new(StaticAssets {
            files: HashMap::new(),
            remaining: AtomicU64::new(100),
        });
        let threads: Vec<_> = (0..20)
            .map(|_| {
                let assets = assets.clone();
                std::thread::spawn(move || assets.reserve(10))
            })
            .collect();
        let successes = threads
            .into_iter()
            .filter_map(|t| t.join().ok())
            .filter(|ok| *ok)
            .count();
        assert_eq!(successes, 10);
        assert_eq!(assets.remaining.load(Ordering::Relaxed), 0);
    }
}
