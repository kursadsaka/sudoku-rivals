//! Shared v1 wire types. Transport I/O stays in each platform adapter.
use serde::{Deserialize, Serialize};

pub const PROTOCOL: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Race,
    Duel,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Ready {
        ready: bool,
    },
    Sync {
        base_version: u64,
        cells: Vec<u8>,
    },
    Move {
        board_id: u32,
        index: u32,
        value: u8,
    },
    Disconnect,
    Resign,
    Rematch,
    Lobby,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub protocol: u8,
    pub token: String,
    pub generation: u64,
    pub match_id: String,
    pub sequence: u64,
    pub command: Command,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    Ok,
    Wrong,
    NotCorrect,
    NoChange,
    NotStarted,
    MatchOver,
    WrongMode,
    InvalidBoard,
    InvalidCell,
    InvalidValue,
    GivenCell,
    FilledCell,
    StaleBoard,
    StaleVersion,
    Locked,
    RateLimited,
    Disconnected,
    InvalidState,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub match_id: String,
    pub at_ms: u64,
    pub event_order: u64,
    pub sequence: u64,
    pub code: Code,
    pub board_version: u64,
    pub board_id: u32,
    pub score: u32,
    pub combo: u32,
    pub wrong: u32,
    pub locked_until_ms: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Error {
    Protocol,
    InvalidInput,
    RoomMissing,
    RoomFull,
    RoomClosed,
    Unauthorized,
    StaleGeneration,
    StaleMatch,
    SequenceGap,
    SequenceConflict,
    Capacity,
    RateLimited,
    ClockOrder,
    Internal,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Lobby,
    Countdown,
    Playing,
    Finished,
    Closed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    Completed,
    Time,
    Resigned,
    DisconnectTimeout,
    BothDisconnected,
    ResourceLimit,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchResult {
    pub winner: Option<usize>,
    pub reason: EndReason,
    pub at_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OwnState {
    pub seat: usize,
    pub name: String,
    pub ready: bool,
    pub connected: bool,
    pub next_sequence: u64,
    pub board_id: u32,
    pub board_version: u64,
    pub givens: Vec<u8>,
    pub cells: Vec<u8>,
    pub score: u32,
    pub combo: u32,
    pub wrong: u32,
    pub locked_until_ms: u64,
    pub rematch: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpponentState {
    pub name: String,
    pub ready: bool,
    pub connected: bool,
    pub progress_percent: Option<u8>,
    pub score: Option<u32>,
    pub rematch: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub protocol: u8,
    pub room_code: String,
    pub match_id: String,
    pub generation: u64,
    pub mode: Mode,
    pub size: u8,
    /// Room setting fixed at creation, like mode and size: every board of every
    /// match in the room is generated at this level. Absent for rooms created
    /// without one (older clients); those keep the unrated blank-count puzzles.
    /// A level this build does not know (a newer server) reads as absent, so the
    /// room stays playable and simply shows no label.
    #[serde(
        default,
        deserialize_with = "known_difficulty",
        skip_serializing_if = "Option::is_none"
    )]
    pub difficulty: Option<crate::Difficulty>,
    pub phase: Phase,
    pub server_ms: u64,
    pub event_order: u64,
    pub start_ms: Option<u64>,
    pub end_ms: Option<u64>,
    pub own: OwnState,
    pub opponent: Option<OpponentState>,
    pub result: Option<MatchResult>,
}
#[derive(Serialize, Clone, Debug, Deserialize)]
pub struct Session {
    pub protocol: u8,
    pub guest: String,
    pub token: String,
    pub invite_path: String,
    pub snapshot: Snapshot,
}
#[derive(Serialize, Clone, Debug, Deserialize)]
pub struct CommandResponse {
    pub receipt: Receipt,
    pub snapshot: Snapshot,
}

fn known_difficulty<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<crate::Difficulty>, D::Error> {
    let name = Option::<String>::deserialize(deserializer)?;
    Ok(name.as_deref().and_then(crate::Difficulty::parse))
}
