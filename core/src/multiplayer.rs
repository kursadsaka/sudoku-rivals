//! Pure multiplayer state. Platform adapters execute descriptors; they must not
//! log request bodies or credential-bearing saves. Public views contain no auth.
use crate::{protocol::*, SoloCommand, SoloGame};
use serde::{Deserialize, Serialize};
use std::fmt;

const JS_MAX: u64 = 9_007_199_254_740_991;
const MAX_JSON: usize = 131_072;
const MAX_SAVE: usize = 1_048_576;
const SAVE_VERSION: u8 = 1;
const ALPHABET: &str = "23456789ABCDEFGHJKMNPQRSTUVWXYZ";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientError {
    InvalidCommand,
    InvalidSnapshot,
    InvalidSave,
    UnsupportedVersion,
    InvalidTime,
    Busy,
    NoSession,
    ResumeRequired,
    ReconcileRequired,
    RetryNotDue,
}
impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ClientError {}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RequestDescriptor {
    pub id: u64,
    pub method: String,
    pub path: String,
    pub body: String,
}
impl fmt::Debug for RequestDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestDescriptor")
            .field("id", &self.id)
            .field("body", &"<redacted>")
            .finish()
    }
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Reconnecting,
    SessionLost,
    Reconcile,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Auth {
    guest: String,
    token: String,
    snapshot: Snapshot,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingCommand {
    envelope: Envelope,
    wire: RequestDescriptor,
}
#[derive(Clone)]
enum Operation {
    Create,
    Join,
    Resume,
    Poll,
    Leave,
    Command,
}
#[derive(Clone)]
struct PendingHttp {
    wire: RequestDescriptor,
    operation: Operation,
    sent_ms: u64,
    retried: bool,
    failed: bool,
}
#[derive(Clone, Default)]
struct Clock {
    samples: Vec<(u64, i64)>,
}
impl Clock {
    fn sample(&mut self, sent: u64, received: u64, server: u64) {
        let rtt = received - sent;
        let midpoint = sent + rtt / 2;
        self.samples.push((rtt, server as i64 - midpoint as i64));
        if self.samples.len() > 5 {
            self.samples.remove(0);
        }
    }
    fn estimate(&self, now: u64) -> Option<u64> {
        let offset = self.samples.iter().min_by_key(|s| s.0)?.1;
        Some((now as i128 + offset as i128).clamp(0, JS_MAX as i128) as u64)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    version: u8,
    seed: u32,
    next_id: u64,
    auth: Option<Auth>,
    draft: Option<String>,
    ack_cells: Vec<u8>,
    ack_version: u64,
    pending: Option<PendingCommand>,
    reconcile: bool,
    session_lost: bool,
    voluntarily_left: bool,
}

#[derive(Clone)]
pub struct MultiplayerClient {
    seed: u32,
    next_id: u64,
    auth: Option<Auth>,
    draft: Option<SoloGame>,
    ack_cells: Vec<u8>,
    ack_version: u64,
    pending_command: Option<PendingCommand>,
    pending_http: Option<PendingHttp>,
    last_receipt: Option<Receipt>,
    last_error: Option<Error>,
    reconcile_required: bool,
    connection: ConnectionState,
    needs_resume: bool,
    voluntarily_left: bool,
    retry_index: usize,
    retry_at: Option<u64>,
    last_poll: Option<u64>,
    last_local_ms: u64,
    clock: Clock,
}
impl fmt::Debug for MultiplayerClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MultiplayerClient")
            .field("connection", &self.connection)
            .field("credentials", &"<redacted>")
            .finish()
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Action {
    Create {
        name: String,
        mode: Mode,
        size: u8,
        #[serde(default)]
        guest: Option<String>,
    },
    Join {
        invite: String,
        name: String,
        #[serde(default)]
        guest: Option<String>,
    },
    Resume {},
    Poll {},
    Sync {},
    Command {
        command: Command,
    },
    Leave {},
    Receive {
        request_id: u64,
        status: u16,
        response: String,
    },
    NetworkFailed {
        request_id: Option<u64>,
    },
    Retry {},
    Edit {
        command: SoloCommand,
    },
    Reconcile {
        keep_local: bool,
    },
}
#[derive(Serialize)]
struct Effect {
    request: Option<RequestDescriptor>,
    changed: bool,
}

impl MultiplayerClient {
    pub fn new(seed: u32) -> Self {
        Self {
            seed,
            next_id: 1,
            auth: None,
            draft: None,
            ack_cells: vec![],
            ack_version: 0,
            pending_command: None,
            pending_http: None,
            last_receipt: None,
            last_error: None,
            reconcile_required: false,
            connection: ConnectionState::Disconnected,
            needs_resume: false,
            voluntarily_left: false,
            retry_index: 0,
            retry_at: None,
            last_poll: None,
            last_local_ms: 0,
            clock: Clock::default(),
        }
    }

    /// One binding-safe action boundary. Invalid actions and malformed responses
    /// are atomic: neither pending requests nor active local data are replaced.
    pub fn dispatch_json(&mut self, action: &str, now_ms: u64) -> Result<String, ClientError> {
        let raw: serde_json::Value = parse(action).map_err(|_| ClientError::InvalidCommand)?;
        if let Some(command) = raw.get("command") {
            strict_command(command).map_err(|_| ClientError::InvalidCommand)?;
        }
        let action: Action =
            serde_json::from_value(raw).map_err(|_| ClientError::InvalidCommand)?;
        let mut next = self.clone();
        if now_ms > JS_MAX || now_ms < next.last_local_ms {
            return Err(ClientError::InvalidTime);
        }
        next.last_local_ms = now_ms;
        let effect = next.act(action, now_ms)?;
        let json = serde_json::to_string(&effect).map_err(|_| ClientError::InvalidCommand)?;
        *self = next;
        Ok(json)
    }

    fn act(&mut self, action: Action, now: u64) -> Result<Effect, ClientError> {
        let mut request = None;
        let mut changed = true;
        match action {
            Action::Create {
                name,
                mode,
                size,
                guest,
            } => {
                self.new_session_allowed()?;
                if !valid_name(&name) || ![6, 9].contains(&size) {
                    return Err(ClientError::InvalidCommand);
                }
                let guest = self.session_guest(guest)?;
                self.pending_http = None;
                self.last_error = None;
                request = Some(self.start(Operation::Create, "/v1/rooms".into(),
                    serde_json::json!({"protocol":PROTOCOL,"guest":guest,"name":name,"mode":mode,"size":size}), now)?);
            }
            Action::Join {
                invite,
                name,
                guest,
            } => {
                self.new_session_allowed()?;
                if !valid_name(&name) {
                    return Err(ClientError::InvalidCommand);
                }
                let code = parse_invite(&invite)?;
                let guest = self.session_guest(guest)?;
                self.pending_http = None;
                self.last_error = None;
                request = Some(self.start(
                    Operation::Join,
                    format!("/v1/rooms/{code}/join"),
                    serde_json::json!({"protocol":PROTOCOL,"guest":guest,"name":name}),
                    now,
                )?);
            }
            Action::Resume {} => {
                if self.retry_at.is_some_and(|at| now < at) {
                    return Err(ClientError::RetryNotDue);
                }
                let auth = self.auth.as_ref().ok_or(ClientError::NoSession)?;
                let path = format!("/v1/rooms/{}/resume", auth.snapshot.room_code);
                let body = serde_json::json!({"protocol":PROTOCOL,"token":auth.token});
                // A resume supersedes transport callbacks; the uncertain command
                // is retained until the new generation's snapshot arrives.
                self.pending_http = None;
                self.connection = ConnectionState::Reconnecting;
                request = Some(self.start(Operation::Resume, path, body, now)?);
            }
            Action::Poll {} => {
                if self.connection == ConnectionState::SessionLost {
                    return Err(ClientError::NoSession);
                }
                if self.needs_resume {
                    return Err(ClientError::ResumeRequired);
                }
                let auth = self.auth.as_ref().ok_or(ClientError::NoSession)?;
                let path = format!("/v1/rooms/{}/state", auth.snapshot.room_code);
                let body = serde_json::json!({"protocol":PROTOCOL,"token":auth.token,"generation":auth.snapshot.generation});
                if self.pending_http.as_ref().is_some_and(|p| p.failed) {
                    self.pending_http = None;
                }
                request = Some(self.start(Operation::Poll, path, body, now)?);
            }
            Action::Sync {} => {
                if self.reconcile_required {
                    return Err(ClientError::ReconcileRequired);
                }
                let draft = self.draft.as_ref().ok_or(ClientError::InvalidCommand)?;
                let command = Command::Sync {
                    base_version: self.ack_version,
                    cells: draft.values(),
                };
                request = Some(self.command(command, now)?);
            }
            Action::Command { command } => {
                request = Some(self.command(command, now)?);
            }
            Action::Leave {} => {
                let auth = self.auth.as_ref().ok_or(ClientError::NoSession)?;
                let path = format!("/v1/rooms/{}/leave", auth.snapshot.room_code);
                let body = serde_json::json!({"protocol":PROTOCOL,"token":auth.token});
                self.pending_http = None;
                request = Some(self.start(Operation::Leave, path, body, now)?);
            }
            Action::Receive {
                request_id,
                status,
                response,
            } => {
                changed = self.receive(request_id, status, &response, now)?;
            }
            Action::NetworkFailed { request_id } => {
                if let Some(id) = request_id {
                    if id == 0 || id > JS_MAX {
                        return Err(ClientError::InvalidCommand);
                    }
                    if self
                        .pending_http
                        .as_ref()
                        .is_none_or(|pending| pending.wire.id != id)
                    {
                        return Ok(Effect {
                            request: None,
                            changed: false,
                        });
                    }
                }
                if let Some(pending) = &mut self.pending_http {
                    pending.failed = true;
                }
                if self.connection != ConnectionState::SessionLost || self.pending_http.is_some() {
                    self.disconnected(now);
                }
            }
            Action::Retry {} => {
                if self.needs_resume {
                    return Err(ClientError::ResumeRequired);
                }
                if self.retry_at.is_some_and(|at| now < at) {
                    return Err(ClientError::RetryNotDue);
                }
                if let Some(http) = &mut self.pending_http {
                    http.sent_ms = now;
                    http.retried = true;
                    http.failed = false;
                    request = Some(http.wire.clone());
                } else if let Some(pending) = &self.pending_command {
                    let wire = pending.wire.clone();
                    self.pending_http = Some(PendingHttp {
                        wire: wire.clone(),
                        operation: Operation::Command,
                        sent_ms: now,
                        retried: true,
                        failed: false,
                    });
                    request = Some(wire);
                } else {
                    return Err(ClientError::InvalidCommand);
                }
                self.connection = ConnectionState::Reconnecting;
            }
            Action::Edit { command } => {
                let snapshot = &self.auth.as_ref().ok_or(ClientError::NoSession)?.snapshot;
                if snapshot.mode != Mode::Race
                    || !matches!(snapshot.phase, Phase::Countdown | Phase::Playing)
                {
                    return Err(ClientError::InvalidCommand);
                }
                changed = self
                    .draft
                    .as_mut()
                    .ok_or(ClientError::InvalidCommand)?
                    .apply(command)
                    .map_err(|_| ClientError::InvalidCommand)?;
            }
            Action::Reconcile { keep_local } => {
                if !self.reconcile_required {
                    return Err(ClientError::InvalidCommand);
                }
                let snapshot = &self.auth.as_ref().ok_or(ClientError::NoSession)?.snapshot;
                if keep_local {
                    if self.draft.as_ref().is_none_or(|draft| {
                        draft.givens() != snapshot.own.givens || draft.size() != snapshot.size
                    }) {
                        return Err(ClientError::ReconcileRequired);
                    }
                } else {
                    self.draft = Some(draft_from(snapshot)?);
                }
                self.reconcile_required = false;
                if self.connection == ConnectionState::Reconcile {
                    self.connection = ConnectionState::Connected;
                }
            }
        }
        Ok(Effect { request, changed })
    }

    fn start(
        &mut self,
        operation: Operation,
        path: String,
        body: serde_json::Value,
        now: u64,
    ) -> Result<RequestDescriptor, ClientError> {
        if self.pending_http.is_some() {
            return Err(ClientError::Busy);
        }
        if self.next_id >= JS_MAX {
            return Err(ClientError::InvalidCommand);
        }
        let wire = RequestDescriptor {
            id: self.next_id,
            method: "POST".into(),
            path,
            body: body.to_string(),
        };
        self.next_id += 1;
        self.pending_http = Some(PendingHttp {
            wire: wire.clone(),
            operation,
            sent_ms: now,
            retried: false,
            failed: false,
        });
        Ok(wire)
    }
    /// A create/join whose response was lost keeps its guest secret so the
    /// replacement request is an idempotent replay on the server.
    fn session_guest(&self, provided: Option<String>) -> Result<Option<String>, ClientError> {
        if let Some(auth) = &self.auth {
            return Ok(Some(auth.guest.clone()));
        }
        if let Some(sent) = self.pending_http.as_ref().and_then(|p| {
            serde_json::from_str::<serde_json::Value>(&p.wire.body)
                .ok()
                .and_then(|b| b["guest"].as_str().map(str::to_owned))
        }) {
            return Ok(Some(sent));
        }
        match provided {
            Some(guest) if !secret(&guest) => Err(ClientError::InvalidCommand),
            other => Ok(other),
        }
    }
    fn new_session_allowed(&self) -> Result<(), ClientError> {
        let abandoned_start = self.pending_http.as_ref().is_some_and(|p| {
            p.failed && matches!(p.operation, Operation::Create | Operation::Join)
        });
        if (self.pending_http.is_some() && !abandoned_start) || self.pending_command.is_some() {
            return Err(ClientError::Busy);
        }
        let ended = self
            .auth
            .as_ref()
            .is_some_and(|a| matches!(a.snapshot.phase, Phase::Finished | Phase::Closed));
        if (self.dirty() || self.reconcile_required) && !self.voluntarily_left && !ended {
            return Err(ClientError::ReconcileRequired);
        }
        if self
            .auth
            .as_ref()
            .is_some_and(|a| !matches!(a.snapshot.phase, Phase::Finished | Phase::Closed))
            && self.connection != ConnectionState::SessionLost
            && !self.voluntarily_left
        {
            return Err(ClientError::Busy);
        }
        Ok(())
    }
    fn command(&mut self, command: Command, now: u64) -> Result<RequestDescriptor, ClientError> {
        if self.needs_resume {
            return Err(ClientError::ResumeRequired);
        }
        if self.pending_command.is_some() {
            return Err(ClientError::Busy);
        }
        if self.reconcile_required {
            return Err(ClientError::ReconcileRequired);
        }
        let auth = self.auth.as_ref().ok_or(ClientError::NoSession)?;
        if self.connection == ConnectionState::SessionLost {
            return Err(ClientError::NoSession);
        }
        if matches!(command, Command::Sync { .. }) {
            let Command::Sync {
                base_version,
                cells,
            } = &command
            else {
                unreachable!()
            };
            if auth.snapshot.mode != Mode::Race
                || *base_version != self.ack_version
                || self
                    .draft
                    .as_ref()
                    .is_none_or(|draft| draft.values() != *cells)
            {
                return Err(ClientError::InvalidCommand);
            }
        }
        if let Command::Move {
            board_id,
            index,
            value,
        } = &command
        {
            if auth.snapshot.mode != Mode::Duel
                || *board_id != auth.snapshot.own.board_id
                || *index as usize >= auth.snapshot.own.cells.len()
                || *value == 0
                || *value > auth.snapshot.size
            {
                return Err(ClientError::InvalidCommand);
            }
        }
        let envelope = Envelope {
            protocol: PROTOCOL,
            token: auth.token.clone(),
            generation: auth.snapshot.generation,
            match_id: auth.snapshot.match_id.clone(),
            sequence: auth.snapshot.own.next_sequence,
            command,
        };
        let path = format!("/v1/rooms/{}/command", auth.snapshot.room_code);
        let body = serde_json::to_value(&envelope).map_err(|_| ClientError::InvalidCommand)?;
        let wire = self.start(Operation::Command, path, body, now)?;
        self.pending_command = Some(PendingCommand {
            envelope,
            wire: wire.clone(),
        });
        Ok(wire)
    }
    fn disconnected(&mut self, now: u64) {
        self.connection = ConnectionState::Disconnected;
        let base = [500u64, 1000, 2000, 4000, 5000][self.retry_index.min(4)];
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let percent = 80 + self.seed as u64 % 41;
        self.retry_at = Some(now.saturating_add(base * percent / 100).min(JS_MAX));
        self.retry_index = (self.retry_index + 1).min(4);
    }
    fn receive(
        &mut self,
        id: u64,
        status: u16,
        response: &str,
        now: u64,
    ) -> Result<bool, ClientError> {
        if id == 0 || id > JS_MAX || response.len() > MAX_JSON || !(100..=599).contains(&status) {
            return Err(ClientError::InvalidSnapshot);
        }
        let Some(http) = self
            .pending_http
            .as_ref()
            .filter(|p| p.wire.id == id)
            .cloned()
        else {
            return Ok(false);
        };
        if !(200..300).contains(&status) {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Failure {
                protocol: u8,
                error: Error,
            }
            let failure: Failure = parse(response).map_err(|_| ClientError::InvalidSnapshot)?;
            if failure.protocol != PROTOCOL {
                return Err(ClientError::InvalidSnapshot);
            }
            if let Some(pending) = &mut self.pending_http {
                pending.failed = true;
            }
            if matches!(http.operation, Operation::Create | Operation::Join) {
                // A refused room start never touches the current session.
                self.pending_http = None;
                self.last_error = Some(failure.error);
                return Ok(true);
            }
            match failure.error {
                Error::RoomMissing | Error::RoomClosed | Error::Unauthorized => {
                    self.connection = ConnectionState::SessionLost;
                    self.pending_http = None;
                    self.pending_command = None;
                    self.needs_resume = false;
                }
                Error::StaleGeneration => {
                    self.needs_resume = true;
                    self.disconnected(now);
                }
                _ => {
                    self.disconnected(now);
                }
            }
            return Ok(true);
        }
        self.last_error = None;
        if matches!(http.operation, Operation::Leave) {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Left {
                protocol: u8,
                left: bool,
            }
            let left: Left = parse(response).map_err(|_| ClientError::InvalidSnapshot)?;
            if left.protocol != PROTOCOL || !left.left {
                return Err(ClientError::InvalidSnapshot);
            }
            self.pending_http = None;
            self.pending_command = None;
            self.connection = ConnectionState::SessionLost;
            self.needs_resume = false;
            self.voluntarily_left = true;
            return Ok(true);
        }
        let (snapshot, new_auth, receipt) = match http.operation {
            Operation::Create | Operation::Join => {
                let session: Session = parse(response).map_err(|_| ClientError::InvalidSnapshot)?;
                if session.protocol != PROTOCOL
                    || !secret(&session.guest)
                    || !secret(&session.token)
                    || session.invite_path != format!("/r/{}", session.snapshot.room_code)
                {
                    return Err(ClientError::InvalidSnapshot);
                }
                let sent: serde_json::Value =
                    parse(&http.wire.body).map_err(|_| ClientError::InvalidSnapshot)?;
                if session.snapshot.phase != Phase::Lobby
                    || sent["name"] != session.snapshot.own.name
                    || sent["guest"]
                        .as_str()
                        .is_some_and(|guest| guest != session.guest)
                    || (matches!(http.operation, Operation::Create)
                        && (sent["mode"]
                            != serde_json::to_value(session.snapshot.mode)
                                .map_err(|_| ClientError::InvalidSnapshot)?
                            || sent["size"] != session.snapshot.size))
                    || (matches!(http.operation, Operation::Join)
                        && http.wire.path
                            != format!("/v1/rooms/{}/join", session.snapshot.room_code))
                {
                    return Err(ClientError::InvalidSnapshot);
                }
                let auth = Auth {
                    guest: session.guest,
                    token: session.token,
                    snapshot: session.snapshot.clone(),
                };
                (session.snapshot, Some(auth), None)
            }
            Operation::Command => {
                let result: CommandResponse =
                    parse(response).map_err(|_| ClientError::InvalidSnapshot)?;
                (result.snapshot, None, Some(result.receipt))
            }
            _ => (
                parse(response).map_err(|_| ClientError::InvalidSnapshot)?,
                None,
                None,
            ),
        };
        validate_snapshot(&snapshot)?;
        if now < http.sent_ms {
            return Err(ClientError::InvalidTime);
        }
        if new_auth.is_none() {
            let old = &self.auth.as_ref().ok_or(ClientError::NoSession)?.snapshot;
            if snapshot.room_code != old.room_code
                || snapshot.own.seat != old.own.seat
                || snapshot.mode != old.mode
                || snapshot.size != old.size
            {
                return Err(ClientError::InvalidSnapshot);
            }
            if snapshot.generation < old.generation
                || snapshot.event_order < old.event_order
                || snapshot.server_ms < old.server_ms
            {
                return Ok(false);
            }
            if matches!(http.operation, Operation::Resume) {
                if snapshot.generation <= old.generation {
                    return Ok(false);
                }
            } else if snapshot.generation != old.generation {
                return Ok(false);
            }
            if snapshot.match_id == old.match_id
                && snapshot.own.next_sequence < old.own.next_sequence
            {
                return Ok(false);
            }
        }
        if let Some(receipt) = &receipt {
            let pending = self
                .pending_command
                .as_ref()
                .ok_or(ClientError::InvalidSnapshot)?;
            let counters_match = snapshot.match_id != receipt.match_id
                || snapshot.own.next_sequence != receipt.sequence.saturating_add(1)
                || (receipt.board_version == snapshot.own.board_version
                    && receipt.board_id == snapshot.own.board_id
                    && receipt.score == snapshot.own.score
                    && receipt.combo == snapshot.own.combo
                    && receipt.wrong == snapshot.own.wrong
                    && receipt.locked_until_ms == snapshot.own.locked_until_ms);
            if !counters_match
                || receipt.match_id != pending.envelope.match_id
                || receipt.sequence != pending.envelope.sequence
                || receipt.at_ms > snapshot.server_ms
                || receipt.event_order > snapshot.event_order
                || receipt.at_ms > JS_MAX
                || receipt.sequence > JS_MAX
                || receipt.event_order > JS_MAX
                || receipt.board_version > JS_MAX
                || receipt.locked_until_ms > JS_MAX
                || (snapshot.match_id == receipt.match_id
                    && snapshot.own.next_sequence <= receipt.sequence)
            {
                return Err(ClientError::InvalidSnapshot);
            }
        }
        let fresh_session = new_auth.is_some();
        if fresh_session {
            self.clock = Clock::default();
        }
        if !http.retried {
            self.clock.sample(http.sent_ms, now, snapshot.server_ms);
        }
        let old_snapshot = self.auth.as_ref().map(|a| a.snapshot.clone());
        if let Some(auth) = new_auth {
            self.auth = Some(auth);
            self.voluntarily_left = false;
            self.draft = None;
            self.ack_cells.clear();
            self.ack_version = 0;
            self.reconcile_required = false;
        }
        self.accept_board(&snapshot, old_snapshot.as_ref(), fresh_session)?;
        let resume = matches!(http.operation, Operation::Resume);
        let processed = self.pending_command.as_ref().is_some_and(|pending| {
            snapshot.match_id != pending.envelope.match_id
                || snapshot.own.next_sequence > pending.envelope.sequence
        });
        if resume || processed || receipt.is_some() {
            self.pending_command = None;
        }
        if let Some(receipt) = receipt {
            self.last_receipt = Some(receipt);
        } else if old_snapshot
            .as_ref()
            .is_some_and(|old| old.match_id != snapshot.match_id)
        {
            self.last_receipt = None;
        }
        self.auth.as_mut().unwrap().snapshot = snapshot.clone();
        self.pending_http = None;
        self.needs_resume = false;
        self.last_poll = Some(now);
        self.retry_at = None;
        self.retry_index = 0;
        self.connection = if !snapshot.own.connected {
            ConnectionState::Disconnected
        } else if self.reconcile_required {
            ConnectionState::Reconcile
        } else {
            ConnectionState::Connected
        };
        if !snapshot.own.connected {
            self.needs_resume = true;
            self.disconnected(now);
        }
        Ok(true)
    }

    fn accept_board(
        &mut self,
        snapshot: &Snapshot,
        previous: Option<&Snapshot>,
        fresh: bool,
    ) -> Result<(), ClientError> {
        if snapshot.mode != Mode::Race {
            if fresh {
                self.draft = None;
                self.ack_cells.clear();
                self.ack_version = 0;
                self.reconcile_required = false;
            }
            return Ok(());
        }
        let ended =
            previous.is_some_and(|old| matches!(old.phase, Phase::Finished | Phase::Closed));
        if snapshot.own.givens.is_empty() {
            let new_match = previous.is_some_and(|old| old.match_id != snapshot.match_id);
            if fresh || (new_match && (!self.dirty() || ended)) {
                self.draft = None;
                self.ack_cells.clear();
                self.ack_version = 0;
                self.reconcile_required = false;
            } else if new_match && self.dirty() {
                self.reconcile_required = true;
            }
            return Ok(());
        }
        let same_match = previous.is_some_and(|p| p.match_id == snapshot.match_id);
        let dirty = self.dirty();
        if self.draft.is_none() || fresh || (!same_match && (!dirty || ended)) {
            self.draft = Some(draft_from(snapshot)?);
            self.reconcile_required = false;
        } else {
            let draft = self.draft.as_ref().unwrap();
            let expected = self.pending_command.as_ref().is_some_and(|pending| {
                pending.envelope.match_id == snapshot.match_id
                    && match &pending.envelope.command {
                        Command::Sync {
                            cells,
                            base_version,
                        } => {
                            *cells == snapshot.own.cells
                                && snapshot.own.board_version >= *base_version
                        }
                        _ => false,
                    }
            });
            if draft.givens() != snapshot.own.givens || draft.size() != snapshot.size {
                self.reconcile_required = true;
            } else if snapshot.own.cells != self.ack_cells
                && (dirty || draft.can_undo() || draft.can_redo())
                && draft.values() != snapshot.own.cells
                && !expected
            {
                self.reconcile_required = true;
            } else if !dirty && draft.values() != snapshot.own.cells {
                self.draft = Some(draft_from(snapshot)?);
            }
        }
        self.ack_cells = snapshot.own.cells.clone();
        self.ack_version = snapshot.own.board_version;
        Ok(())
    }
    fn dirty(&self) -> bool {
        self.draft
            .as_ref()
            .is_some_and(|draft| draft.values() != self.ack_cells)
    }

    pub fn view_json(&self, now_ms: u64) -> Result<String, ClientError> {
        if now_ms > JS_MAX || now_ms < self.last_local_ms {
            return Err(ClientError::InvalidTime);
        }
        #[derive(Serialize)]
        struct RaceView {
            cells: Vec<u8>,
            notes: Vec<Vec<u8>>,
            solo: serde_json::Value,
            conflicts: Vec<u32>,
            can_undo: bool,
            can_redo: bool,
            dirty: bool,
        }
        #[derive(Serialize)]
        struct View<'a> {
            connection: &'a ConnectionState,
            snapshot: Option<&'a Snapshot>,
            race: Option<RaceView>,
            last_receipt: Option<&'a Receipt>,
            last_error: Option<&'a Error>,
            pending_command: bool,
            pending_http: bool,
            reconcile_required: bool,
            retry_at_ms: Option<u64>,
            poll_due: bool,
            estimated_server_ms: Option<u64>,
            remaining_ms: Option<u64>,
            locked_remaining_ms: Option<u64>,
            clock_samples: usize,
            voluntarily_left: bool,
        }
        let snapshot = self.auth.as_ref().map(|a| &a.snapshot);
        let estimate = self.clock.estimate(now_ms);
        let remaining = snapshot
            .and_then(|s| s.end_ms)
            .zip(estimate)
            .map(|(end, now)| end.saturating_sub(now));
        let lock = snapshot
            .zip(estimate)
            .map(|(s, now)| s.own.locked_until_ms.saturating_sub(now));
        let race = self.draft.as_ref().map(|draft| RaceView {
            cells: draft.values(),
            notes: (0..draft.values().len())
                .map(|i| draft.notes(i as u32).unwrap_or_default())
                .collect(),
            solo: draft
                .save()
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::Value::Null),
            conflicts: draft.conflicts(),
            can_undo: draft.can_undo(),
            can_redo: draft.can_redo(),
            dirty: self.dirty(),
        });
        let countdown_due = snapshot.is_some_and(|s| {
            s.phase == Phase::Countdown
                && s.start_ms
                    .zip(estimate)
                    .is_some_and(|(start, now)| now >= start)
        });
        let poll_interval =
            if snapshot.is_some_and(|s| matches!(s.phase, Phase::Lobby | Phase::Countdown)) {
                500
            } else {
                1000
            };
        let view = View {
            connection: &self.connection,
            snapshot,
            race,
            last_receipt: self.last_receipt.as_ref(),
            last_error: self.last_error.as_ref(),
            pending_command: self.pending_command.is_some(),
            pending_http: self.pending_http.is_some(),
            reconcile_required: self.reconcile_required,
            retry_at_ms: self.retry_at,
            poll_due: self.auth.is_some()
                && matches!(
                    self.connection,
                    ConnectionState::Connected | ConnectionState::Reconcile
                )
                && !self.needs_resume
                && self.pending_http.is_none()
                && (self
                    .last_poll
                    .is_none_or(|last| now_ms.saturating_sub(last) >= poll_interval)
                    || countdown_due),
            estimated_server_ms: estimate,
            remaining_ms: remaining,
            locked_remaining_ms: lock,
            clock_samples: self.clock.samples.len(),
            voluntarily_left: self.voluntarily_left,
        };
        serde_json::to_string(&view).map_err(|_| ClientError::InvalidSnapshot)
    }
    pub fn save(&self) -> Result<String, ClientError> {
        let saved = Saved {
            version: SAVE_VERSION,
            seed: self.seed,
            next_id: self.next_id,
            auth: self.auth.clone(),
            draft: self
                .draft
                .as_ref()
                .map(SoloGame::save)
                .transpose()
                .map_err(|_| ClientError::InvalidSave)?,
            ack_cells: self.ack_cells.clone(),
            ack_version: self.ack_version,
            pending: self.pending_command.clone(),
            reconcile: self.reconcile_required,
            session_lost: self.connection == ConnectionState::SessionLost,
            voluntarily_left: self.voluntarily_left,
        };
        let json = serde_json::to_string(&saved).map_err(|_| ClientError::InvalidSave)?;
        if json.len() > MAX_SAVE {
            return Err(ClientError::InvalidSave);
        }
        Ok(json)
    }
    pub fn load(json: &str) -> Result<Self, ClientError> {
        if json.len() > MAX_SAVE {
            return Err(ClientError::InvalidSave);
        }
        let raw: serde_json::Value =
            serde_json::from_str(json).map_err(|_| ClientError::InvalidSave)?;
        if let Some(command) = raw.pointer("/pending/envelope/command") {
            strict_command(command).map_err(|_| ClientError::InvalidSave)?;
        }
        let saved: Saved = serde_json::from_value(raw).map_err(|_| ClientError::InvalidSave)?;
        if saved.version != SAVE_VERSION {
            return Err(ClientError::UnsupportedVersion);
        }
        if saved.next_id == 0 || saved.next_id > JS_MAX || saved.ack_version > JS_MAX {
            return Err(ClientError::InvalidSave);
        }
        let draft = saved
            .draft
            .as_deref()
            .map(SoloGame::load)
            .transpose()
            .map_err(|_| ClientError::InvalidSave)?;
        if let Some(auth) = &saved.auth {
            if !secret(&auth.guest) || !secret(&auth.token) {
                return Err(ClientError::InvalidSave);
            }
            validate_snapshot(&auth.snapshot).map_err(|_| ClientError::InvalidSave)?;
            if auth.snapshot.mode == Mode::Duel
                && (draft.is_some() || !saved.ack_cells.is_empty() || saved.ack_version != 0)
            {
                return Err(ClientError::InvalidSave);
            }
            if auth.snapshot.mode == Mode::Race {
                if let Some(draft) = &draft {
                    if draft.size() != auth.snapshot.size
                        || saved.ack_cells.len() != (auth.snapshot.size as usize).pow(2)
                        || saved.ack_cells.iter().any(|&v| v > auth.snapshot.size)
                        || (auth.snapshot.own.givens.is_empty()
                            && saved
                                .ack_cells
                                .iter()
                                .zip(draft.givens())
                                .any(|(&v, &g)| g != 0 && v != g))
                    {
                        return Err(ClientError::InvalidSave);
                    }
                } else if !saved.ack_cells.is_empty() || saved.ack_version != 0 || saved.reconcile {
                    return Err(ClientError::InvalidSave);
                }
            }
            if auth.snapshot.mode == Mode::Race && !auth.snapshot.own.givens.is_empty() {
                if saved.ack_cells != auth.snapshot.own.cells
                    || saved.ack_version != auth.snapshot.own.board_version
                    || draft.as_ref().is_none_or(|d| {
                        d.size() != auth.snapshot.size
                            || (!saved.reconcile && d.givens() != auth.snapshot.own.givens)
                    })
                {
                    return Err(ClientError::InvalidSave);
                }
            }
            if let Some(pending) = &saved.pending {
                match &pending.envelope.command {
                    Command::Sync {
                        base_version,
                        cells,
                    } => {
                        if auth.snapshot.mode != Mode::Race
                            || *base_version > JS_MAX
                            || cells.len() != (auth.snapshot.size as usize).pow(2)
                            || cells.iter().any(|&v| v > auth.snapshot.size)
                            || (!auth.snapshot.own.givens.is_empty()
                                && cells
                                    .iter()
                                    .zip(&auth.snapshot.own.givens)
                                    .any(|(&v, &g)| g != 0 && v != g))
                        {
                            return Err(ClientError::InvalidSave);
                        }
                    }
                    Command::Move {
                        board_id,
                        index,
                        value,
                    } => {
                        if auth.snapshot.mode != Mode::Duel
                            || *board_id != auth.snapshot.own.board_id
                            || *index as usize >= auth.snapshot.own.cells.len()
                            || *value == 0
                            || *value > auth.snapshot.size
                        {
                            return Err(ClientError::InvalidSave);
                        }
                    }
                    _ => {}
                }
                let expected = serde_json::to_value(&pending.envelope)
                    .map_err(|_| ClientError::InvalidSave)?
                    .to_string();
                if pending.envelope.protocol != PROTOCOL
                    || pending.envelope.token != auth.token
                    || pending.envelope.generation != auth.snapshot.generation
                    || pending.envelope.match_id != auth.snapshot.match_id
                    || pending.envelope.sequence != auth.snapshot.own.next_sequence
                    || pending.wire.id == 0
                    || pending.wire.id >= saved.next_id
                    || pending.wire.method != "POST"
                    || pending.wire.path != format!("/v1/rooms/{}/command", auth.snapshot.room_code)
                    || pending.wire.body != expected
                {
                    return Err(ClientError::InvalidSave);
                }
            }
        } else if draft.is_some()
            || !saved.ack_cells.is_empty()
            || saved.ack_version != 0
            || saved.pending.is_some()
            || saved.reconcile
            || saved.voluntarily_left
        {
            return Err(ClientError::InvalidSave);
        }
        let mut client = Self::new(saved.seed);
        client.voluntarily_left = saved.voluntarily_left;
        client.next_id = saved.next_id;
        client.needs_resume = saved.auth.is_some();
        client.auth = saved.auth;
        client.draft = draft;
        client.ack_cells = saved.ack_cells;
        client.ack_version = saved.ack_version;
        client.pending_command = saved.pending;
        client.reconcile_required = saved.reconcile;
        if saved.session_lost {
            client.connection = ConnectionState::SessionLost;
        }
        Ok(client)
    }
}

fn parse<T: serde::de::DeserializeOwned>(json: &str) -> Result<T, ()> {
    if json.len() > MAX_JSON {
        return Err(());
    }
    serde_json::from_str(json).map_err(|_| ())
}
fn strict_command(command: &serde_json::Value) -> Result<(), ()> {
    if command
        .get("type")
        .and_then(|v| v.as_str())
        .is_some_and(|kind| ["disconnect", "resign", "rematch", "lobby"].contains(&kind))
        && command.as_object().is_none_or(|object| object.len() != 1)
    {
        return Err(());
    }
    Ok(())
}

fn secret(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|c| c.is_ascii_hexdigit())
}
fn valid_name(name: &str) -> bool {
    (1..=16).contains(&name.chars().count())
        && !name.trim().is_empty()
        && !name.chars().any(char::is_control)
}
fn code(value: &str) -> bool {
    value.len() == 6 && value.bytes().all(|c| ALPHABET.as_bytes().contains(&c))
}

/// Accept a bare room code, canonical path, or HTTP(S) invitation URL. Query
/// strings, fragments and userinfo are rejected so links never carry seat auth.
pub fn parse_invite(invite: &str) -> Result<String, ClientError> {
    if invite.len() > 2048 || invite.contains(['?', '#']) {
        return Err(ClientError::InvalidCommand);
    }
    let invite = invite.trim();
    let path = if let Some(url) = invite
        .strip_prefix("https://")
        .or_else(|| invite.strip_prefix("http://"))
    {
        let (host, path) = url.split_once('/').ok_or(ClientError::InvalidCommand)?;
        if host.is_empty()
            || host.contains('@')
            || host.chars().any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(ClientError::InvalidCommand);
        }
        path.strip_prefix("r/").ok_or(ClientError::InvalidCommand)?
    } else {
        invite.strip_prefix("/r/").unwrap_or(invite)
    };
    let normalized = path.to_ascii_uppercase();
    if !code(&normalized) {
        return Err(ClientError::InvalidCommand);
    }
    Ok(normalized)
}
fn draft_from(snapshot: &Snapshot) -> Result<SoloGame, ClientError> {
    let mut game = SoloGame::new(snapshot.size, snapshot.own.givens.clone())
        .map_err(|_| ClientError::InvalidSnapshot)?;
    for (index, (&given, &value)) in snapshot
        .own
        .givens
        .iter()
        .zip(&snapshot.own.cells)
        .enumerate()
    {
        if given == 0 && value != 0 {
            game.apply(SoloCommand::Set {
                index: index as u32,
                value,
            })
            .map_err(|_| ClientError::InvalidSnapshot)?;
        }
    }
    Ok(game)
}

pub fn validate_snapshot(snapshot: &Snapshot) -> Result<(), ClientError> {
    let fail = ClientError::InvalidSnapshot;
    if snapshot.protocol != PROTOCOL
        || ![6, 9].contains(&snapshot.size)
        || !code(&snapshot.room_code)
        || !secret(&snapshot.match_id)
        || snapshot.generation == 0
        || snapshot.generation > JS_MAX
        || snapshot.server_ms > JS_MAX
        || snapshot.event_order > JS_MAX
        || snapshot.own.seat > 1
        || snapshot.own.next_sequence == 0
        || snapshot.own.next_sequence > JS_MAX
        || snapshot.own.board_version > JS_MAX
        || snapshot.own.locked_until_ms > JS_MAX
        || !valid_name(&snapshot.own.name)
        || snapshot.own.locked_until_ms > snapshot.server_ms.saturating_add(2000)
    {
        return Err(fail);
    }
    for time in [snapshot.start_ms, snapshot.end_ms].into_iter().flatten() {
        if time > JS_MAX {
            return Err(fail);
        }
    }
    if let Some(opponent) = &snapshot.opponent {
        if !valid_name(&opponent.name)
            || opponent.progress_percent.is_some_and(|v| v > 100)
            || (snapshot.mode == Mode::Race
                && (opponent.progress_percent.is_none() || opponent.score.is_some()))
            || (snapshot.mode == Mode::Duel
                && (opponent.score.is_none() || opponent.progress_percent.is_some()))
        {
            return Err(fail);
        }
    }
    if snapshot.mode == Mode::Race && snapshot.end_ms.is_some() {
        return Err(fail);
    }
    if snapshot.mode == Mode::Duel {
        let expected = match snapshot.start_ms {
            Some(start) => Some(
                start
                    .checked_add(90_000)
                    .filter(|&end| end <= JS_MAX)
                    .ok_or(ClientError::InvalidSnapshot)?,
            ),
            None => None,
        };
        if snapshot.end_ms != expected {
            return Err(fail);
        }
    }
    if snapshot.mode == Mode::Race
        && (snapshot.own.score != 0
            || snapshot.own.combo != 0
            || snapshot.own.wrong != 0
            || snapshot.own.locked_until_ms != 0)
    {
        return Err(fail);
    }
    if snapshot.phase == Phase::Finished
        && snapshot.start_ms.is_none()
        && snapshot
            .result
            .as_ref()
            .is_none_or(|result| result.reason != EndReason::ResourceLimit)
    {
        return Err(fail);
    }
    let visible = matches!(snapshot.phase, Phase::Playing | Phase::Finished);
    let empty_resource_result = snapshot.phase == Phase::Finished
        && snapshot.start_ms.is_none()
        && snapshot
            .result
            .as_ref()
            .is_some_and(|r| r.reason == EndReason::ResourceLimit);
    if visible && !empty_resource_result {
        if !crate::validate_board(&snapshot.own.givens, snapshot.size)
            || snapshot.own.cells.len() != snapshot.own.givens.len()
            || snapshot.own.cells.iter().any(|&v| v > snapshot.size)
            || snapshot
                .own
                .cells
                .iter()
                .zip(&snapshot.own.givens)
                .any(|(&v, &g)| g != 0 && g != v)
            || (snapshot.mode == Mode::Duel
                && !crate::validate_board(&snapshot.own.cells, snapshot.size))
        {
            return Err(fail);
        }
    } else if !snapshot.own.cells.is_empty() || !snapshot.own.givens.is_empty() {
        return Err(fail);
    }
    match snapshot.phase {
        Phase::Lobby => {
            if snapshot.start_ms.is_some() || snapshot.result.is_some() {
                return Err(fail);
            }
        }
        Phase::Countdown => {
            if snapshot
                .start_ms
                .is_none_or(|start| start <= snapshot.server_ms)
                || snapshot.opponent.is_none()
                || snapshot.result.is_some()
            {
                return Err(fail);
            }
        }
        Phase::Playing => {
            if snapshot
                .start_ms
                .is_none_or(|start| start > snapshot.server_ms)
                || snapshot.opponent.is_none()
                || snapshot.result.is_some()
                || snapshot.end_ms.is_some_and(|end| end <= snapshot.server_ms)
            {
                return Err(fail);
            }
        }
        Phase::Finished => {
            let result = snapshot
                .result
                .as_ref()
                .ok_or(ClientError::InvalidSnapshot)?;
            if result.winner.is_some_and(|w| w > 1)
                || result.at_ms > snapshot.server_ms
                || result.at_ms > JS_MAX
                || (result.reason != EndReason::ResourceLimit
                    && snapshot.start_ms.is_some_and(|start| result.at_ms < start))
            {
                return Err(fail);
            }
            match result.reason {
                EndReason::Completed => {
                    if snapshot.mode != Mode::Race
                        || result.winner.is_none()
                        || (result.winner == Some(snapshot.own.seat)
                            && !crate::is_complete(&snapshot.own.cells, snapshot.size))
                    {
                        return Err(fail);
                    }
                }
                EndReason::Time => {
                    if snapshot.mode != Mode::Duel || Some(result.at_ms) != snapshot.end_ms {
                        return Err(fail);
                    }
                }
                EndReason::DisconnectTimeout => {
                    if snapshot.mode != Mode::Race || result.winner.is_none() {
                        return Err(fail);
                    }
                }
                EndReason::BothDisconnected => {
                    if snapshot.mode != Mode::Race || result.winner.is_some() {
                        return Err(fail);
                    }
                }
                EndReason::ResourceLimit => {
                    if result.winner.is_some() {
                        return Err(fail);
                    }
                }
                EndReason::Resigned => {
                    if result.winner.is_none() {
                        return Err(fail);
                    }
                }
            }
        }
        Phase::Closed => {
            if snapshot.result.is_some() {
                return Err(fail);
            }
        }
    }
    Ok(())
}
