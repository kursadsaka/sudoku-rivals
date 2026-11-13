//! Local, in-memory authoritative 1v1 matches. Public snapshots never contain a
//! solution, the opponent's cells, seat credentials, or puzzle generation seeds.
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const COUNTDOWN_MS: u64 = 3_000;
const DUEL_MS: u64 = 90_000;
const HEARTBEAT_MS: u64 = 15_000;
const RECONNECT_MS: u64 = 120_000;
const LOBBY_MS: u64 = 1_800_000;
const RESULT_MS: u64 = 600_000;
const CLOSED_MS: u64 = 86_400_000;
const MAX_ROOMS: usize = 128;
const MAX_RECEIPTS: usize = 16_384;

pub use sudoku_core::protocol::*;

struct Puzzle {
    givens: Vec<u8>,
    solution: Vec<u8>,
}
#[derive(Clone)]
struct StoredReceipt {
    command: Command,
    receipt: Receipt,
}
struct Seat {
    guest: String,
    token: String,
    name: String,
    generation: u64,
    connected: bool,
    last_seen: u64,
    disconnected_at: Option<u64>,
    ready: bool,
    rematch: bool,
    cells: Vec<u8>,
    board_id: u32,
    version: u64,
    score: u32,
    combo: u32,
    wrong: u32,
    locked_until: u64,
    receipts: Vec<StoredReceipt>,
    rate: VecDeque<u64>,
    rate_rejections: VecDeque<u64>,
    // Opponent progress is sampled at most twice per second.
    published_progress: u8,
    progress_at: u64,
}
impl Seat {
    fn new(guest: String, token: String, name: String, now: u64) -> Self {
        Self {
            guest,
            token,
            name,
            generation: 1,
            connected: true,
            last_seen: now,
            disconnected_at: None,
            ready: false,
            rematch: false,
            cells: vec![],
            board_id: 0,
            version: 0,
            score: 0,
            combo: 0,
            wrong: 0,
            locked_until: 0,
            receipts: vec![],
            rate: VecDeque::new(),
            rate_rejections: VecDeque::new(),
            published_progress: 0,
            progress_at: now,
        }
    }
    fn reset(&mut self, givens: &[u8]) {
        self.ready = false;
        self.rematch = false;
        self.cells = givens.to_vec();
        self.board_id = 0;
        self.version = 0;
        self.score = 0;
        self.combo = 0;
        self.wrong = 0;
        self.locked_until = 0;
        self.receipts.clear();
        self.rate.clear();
        self.rate_rejections.clear();
        self.published_progress = 0;
    }
    fn disconnect(&mut self, now: u64) {
        if self.connected {
            self.connected = false;
            self.disconnected_at = Some(now);
            self.ready = false;
        }
    }
}

struct Room {
    code: String,
    match_id: String,
    mode: Mode,
    size: u8,
    seats: [Option<Seat>; 2],
    puzzles: Vec<Puzzle>,
    seed: u32,
    phase: Phase,
    start: Option<u64>,
    result: Option<MatchResult>,
    last_activity: u64,
    closed_at: Option<u64>,
    event_order: u64,
    last_event_ms: u64,
    archived: Vec<(String, [(String, Vec<StoredReceipt>); 2])>,
}
impl Room {
    fn new(code: String, mode: Mode, size: u8, seat: Seat, now: u64) -> Result<Self, Error> {
        if ![6, 9].contains(&size) {
            return Err(Error::InvalidInput);
        }
        let seed = u32::from_le_bytes(random_bytes::<4>()?);
        Ok(Self {
            code,
            match_id: random_hex()?,
            mode,
            size,
            seats: [Some(seat), None],
            puzzles: vec![],
            seed,
            phase: Phase::Lobby,
            start: None,
            result: None,
            last_activity: now,
            closed_at: None,
            event_order: 0,
            last_event_ms: now,
            archived: vec![],
        })
    }
    fn puzzle(&mut self, board_id: u32) -> Result<(), Error> {
        if board_id as usize == self.puzzles.len() {
            self.puzzles
                .push(make_puzzle(self.size, self.seed, board_id)?);
        }
        Ok(())
    }
    fn event(&mut self, now: u64) -> Result<(), Error> {
        if now < self.last_event_ms {
            return Err(Error::ClockOrder);
        }
        self.last_event_ms = now;
        self.event_order += 1;
        self.advance(now);
        Ok(())
    }
    fn finish(&mut self, winner: Option<usize>, reason: EndReason, now: u64) {
        if self.result.is_none() {
            if self.mode == Mode::Race {
                if let Some(puzzle) = self.puzzles.first() {
                    let blanks = puzzle.givens.iter().filter(|&&v| v == 0).count().max(1);
                    for seat in self.seats.iter_mut().flatten() {
                        let filled = seat
                            .cells
                            .iter()
                            .zip(&puzzle.givens)
                            .filter(|(v, g)| **g == 0 && **v != 0)
                            .count();
                        seat.published_progress = (filled * 100 / blanks) as u8;
                        seat.progress_at = now;
                    }
                }
            }
            self.result = Some(MatchResult {
                winner,
                reason,
                at_ms: now,
            });
            self.phase = Phase::Finished;
            self.last_activity = now;
        }
    }
    fn advance(&mut self, now: u64) {
        if self.phase == Phase::Closed {
            return;
        }
        for seat in self.seats.iter_mut().flatten() {
            if seat.connected && now >= seat.last_seen.saturating_add(HEARTBEAT_MS) {
                // The server declares disconnect at this event, not at a client timestamp.
                seat.disconnect(now);
            }
        }
        if self.phase == Phase::Countdown && self.seats.iter().flatten().any(|s| !s.connected) {
            self.phase = Phase::Lobby;
            self.start = None;
            for seat in self.seats.iter_mut().flatten() {
                seat.ready = false;
            }
        }
        if self.phase == Phase::Countdown && now >= self.start.unwrap_or(u64::MAX) {
            self.phase = Phase::Playing;
        }
        if self.phase == Phase::Playing {
            if self.mode == Mode::Duel && now >= self.start.unwrap() + DUEL_MS {
                let a = self.seats[0].as_ref().unwrap();
                let b = self.seats[1].as_ref().unwrap();
                let winner = match (a.score, std::cmp::Reverse(a.wrong))
                    .cmp(&(b.score, std::cmp::Reverse(b.wrong)))
                {
                    std::cmp::Ordering::Greater => Some(0),
                    std::cmp::Ordering::Less => Some(1),
                    _ => None,
                };
                self.finish(winner, EndReason::Time, self.start.unwrap() + DUEL_MS);
            } else if self.mode == Mode::Race {
                let deadlines: [Option<u64>; 2] = std::array::from_fn(|i| {
                    self.seats[i]
                        .as_ref()
                        .and_then(|s| s.disconnected_at)
                        .map(|t| t.saturating_add(RECONNECT_MS))
                });
                let first = deadlines.iter().flatten().copied().min();
                if let Some(first) = first.filter(|&t| now >= t) {
                    if deadlines[0] == Some(first) && deadlines[1] == Some(first) {
                        self.finish(None, EndReason::BothDisconnected, first);
                    } else {
                        self.finish(
                            Some(if deadlines[0] == Some(first) { 1 } else { 0 }),
                            EndReason::DisconnectTimeout,
                            first,
                        );
                    }
                }
            }
        }
        if self.phase == Phase::Lobby {
            // A disconnected second seat is reserved for the reconnect interval.
            for i in 0..2 {
                if self.seats[i]
                    .as_ref()
                    .and_then(|s| s.disconnected_at)
                    .is_some_and(|t| now >= t.saturating_add(RECONNECT_MS))
                    && self.seats[1 - i].is_some()
                {
                    self.seats[i] = None;
                }
            }
        }
        let ttl = match self.phase {
            Phase::Lobby => Some(LOBBY_MS),
            Phase::Finished => Some(RESULT_MS),
            _ => None,
        };
        let activity = if self.phase == Phase::Finished {
            self.result
                .as_ref()
                .map(|r| r.at_ms)
                .unwrap_or(self.last_activity)
        } else {
            self.last_activity
        };
        if ttl.is_some_and(|ttl| now >= activity.saturating_add(ttl)) {
            self.phase = Phase::Closed;
            self.closed_at = Some(now);
        }
        if self.phase == Phase::Playing && self.mode == Mode::Race {
            let givens = &self.puzzles[0].givens;
            let blanks = givens.iter().filter(|&&v| v == 0).count();
            for seat in self.seats.iter_mut().flatten() {
                if now >= seat.progress_at.saturating_add(500) {
                    let filled = seat
                        .cells
                        .iter()
                        .zip(givens)
                        .filter(|(v, g)| **g == 0 && **v != 0)
                        .count();
                    seat.published_progress = (filled * 100 / blanks.max(1)) as u8;
                    seat.progress_at = now;
                }
            }
        }
    }
    fn seat_index(&self, token: &str) -> Result<usize, Error> {
        self.seats
            .iter()
            .position(|s| s.as_ref().is_some_and(|s| s.token == token))
            .ok_or(Error::Unauthorized)
    }
    fn snapshot(&self, seat_index: usize, now: u64) -> Snapshot {
        let own = self.seats[seat_index].as_ref().unwrap();
        let shown = matches!(
            self.phase,
            Phase::Countdown | Phase::Playing | Phase::Finished
        );
        let givens = if shown {
            self.puzzles
                .get(own.board_id as usize)
                .map(|p| p.givens.clone())
                .unwrap_or_default()
        } else {
            vec![]
        };
        let opponent = self.seats[1 - seat_index].as_ref().map(|s| OpponentState {
            name: s.name.clone(),
            ready: s.ready,
            connected: s.connected,
            rematch: s.rematch,
            progress_percent: if self.mode == Mode::Race {
                Some(s.published_progress)
            } else {
                None
            },
            score: if self.mode == Mode::Duel {
                Some(s.score)
            } else {
                None
            },
        });
        Snapshot {
            protocol: PROTOCOL,
            room_code: self.code.clone(),
            match_id: self.match_id.clone(),
            generation: own.generation,
            mode: self.mode,
            size: self.size,
            phase: self.phase.clone(),
            server_ms: now,
            event_order: self.event_order,
            start_ms: self.start,
            end_ms: self
                .start
                .filter(|_| self.mode == Mode::Duel)
                .map(|start| start + DUEL_MS),
            own: OwnState {
                seat: seat_index,
                name: own.name.clone(),
                ready: own.ready,
                connected: own.connected,
                next_sequence: own.receipts.len() as u64 + 1,
                board_id: own.board_id,
                board_version: own.version,
                givens,
                cells: if shown { own.cells.clone() } else { vec![] },
                score: own.score,
                combo: own.combo,
                wrong: own.wrong,
                locked_until_ms: own.locked_until,
                rematch: own.rematch,
            },
            opponent,
            result: if self.phase == Phase::Finished {
                self.result.clone()
            } else {
                None
            },
        }
    }
    fn start_countdown(&mut self, now: u64) -> Result<(), Error> {
        self.puzzle(0)?;
        let givens = &self.puzzles[0].givens;
        for seat in self.seats.iter_mut().flatten() {
            seat.cells = givens.clone();
            seat.ready = true;
        }
        self.start = Some(now + COUNTDOWN_MS);
        self.phase = Phase::Countdown;
        Ok(())
    }
    fn apply(&mut self, index: usize, command: &Command, now: u64) -> Code {
        if self.phase == Phase::Closed {
            return Code::MatchOver;
        }
        if matches!(command, Command::Ready { .. }) {
            if !matches!(self.phase, Phase::Lobby | Phase::Countdown) {
                return Code::InvalidState;
            }
            if self.seats[1 - index].is_none() {
                return Code::InvalidState;
            }
            let Command::Ready { ready } = command else {
                unreachable!()
            };
            self.seats[index].as_mut().unwrap().ready = *ready;
            if !ready && self.phase == Phase::Countdown {
                self.phase = Phase::Lobby;
                self.start = None;
            }
            if self.phase == Phase::Lobby
                && self
                    .seats
                    .iter()
                    .all(|s| s.as_ref().is_some_and(|s| s.ready && s.connected))
            {
                let start = if self.result.is_some() {
                    Ok(())
                } else {
                    self.start_countdown(now)
                };
                if start.is_err() {
                    return Code::InvalidBoard;
                }
            }
            return Code::Ok;
        }
        match command {
            Command::Disconnect => {
                self.seats[index].as_mut().unwrap().disconnect(now);
                if self.phase == Phase::Countdown {
                    self.phase = Phase::Lobby;
                    self.start = None;
                }
                return Code::Ok;
            }
            Command::Rematch => {
                if self.phase != Phase::Finished {
                    return Code::InvalidState;
                }
                self.seats[index].as_mut().unwrap().rematch = true;
                if self
                    .seats
                    .iter()
                    .all(|s| s.as_ref().is_some_and(|s| s.rematch && s.connected))
                {
                    // The response to this command is kept in the old match's receipts
                    // by the caller; new match receipts are installed afterward.
                    return Code::Ok;
                }
                return Code::Ok;
            }
            Command::Lobby => {
                if self.phase != Phase::Finished {
                    return Code::InvalidState;
                }
                self.phase = Phase::Lobby;
                self.start = None;
                for seat in self.seats.iter_mut().flatten() {
                    seat.ready = false;
                    seat.rematch = false;
                }
                return Code::Ok;
            }
            _ => {}
        }
        if self.phase == Phase::Finished {
            return Code::MatchOver;
        }
        if self.phase != Phase::Playing {
            return Code::NotStarted;
        }
        if matches!(command, Command::Resign) {
            self.finish(Some(1 - index), EndReason::Resigned, now);
            return Code::Ok;
        }
        let seat = self.seats[index].as_mut().unwrap();
        // New attempts consume admission quota before correctness is examined.
        while seat
            .rate
            .front()
            .is_some_and(|&t| now.saturating_sub(t) >= 1000)
        {
            seat.rate.pop_front();
        }
        if seat.rate.len() >= 6 {
            while seat
                .rate_rejections
                .front()
                .is_some_and(|&t| now.saturating_sub(t) >= 10_000)
            {
                seat.rate_rejections.pop_front();
            }
            seat.rate_rejections.push_back(now);
            if seat.rate_rejections.len() >= 30 {
                seat.disconnect(now);
            }
            return Code::RateLimited;
        }
        seat.rate.push_back(now);
        match command {
            Command::Sync {
                base_version,
                cells,
            } => {
                if self.mode != Mode::Race {
                    return Code::WrongMode;
                }
                if *base_version != seat.version {
                    return Code::StaleVersion;
                }
                let givens = &self.puzzles[0].givens;
                if cells.len() != givens.len()
                    || cells.iter().any(|&v| v > self.size)
                    || givens.iter().zip(cells).any(|(&g, &v)| g != 0 && g != v)
                {
                    return Code::InvalidBoard;
                }
                if cells == &seat.cells {
                    return Code::NoChange;
                }
                seat.cells = cells.clone();
                seat.version += 1;
                if !cells.contains(&0) {
                    if sudoku_core::is_complete(cells, self.size) {
                        self.finish(Some(index), EndReason::Completed, now);
                        Code::Ok
                    } else {
                        Code::NotCorrect
                    }
                } else {
                    Code::Ok
                }
            }
            Command::Move {
                board_id,
                index: cell_index,
                value,
            } => {
                if self.mode != Mode::Duel {
                    return Code::WrongMode;
                }
                if now < seat.locked_until {
                    return Code::Locked;
                }
                if *board_id != seat.board_id {
                    return Code::StaleBoard;
                }
                let Some(cell) = seat.cells.get(*cell_index as usize) else {
                    return Code::InvalidCell;
                };
                if *value == 0 || *value > self.size {
                    return Code::InvalidValue;
                }
                let puzzle = &self.puzzles[*board_id as usize];
                if puzzle.givens[*cell_index as usize] != 0 {
                    return Code::GivenCell;
                }
                if *cell != 0 {
                    return Code::FilledCell;
                }
                if puzzle.solution[*cell_index as usize] != *value {
                    seat.score = seat.score.saturating_sub(5);
                    seat.combo = 0;
                    seat.wrong += 1;
                    seat.locked_until = now + 2_000;
                    return Code::Wrong;
                }
                seat.cells[*cell_index as usize] = *value;
                seat.version += 1;
                seat.combo += 1;
                seat.score += match seat.combo {
                    1..=3 => 10,
                    4..=6 => 15,
                    7..=9 => 20,
                    _ => 25,
                };
                if !seat.cells.contains(&0) {
                    seat.score += 50;
                    let next = seat.board_id + 1;
                    if self.puzzle(next).is_err() {
                        self.finish(None, EndReason::ResourceLimit, now);
                        return Code::InvalidBoard;
                    }
                    let seat = self.seats[index].as_mut().unwrap();
                    seat.board_id = next;
                    seat.cells = self.puzzles[next as usize].givens.clone();
                    seat.version += 1;
                }
                Code::Ok
            }
            _ => Code::InvalidState,
        }
    }
    fn command(&mut self, envelope: &Envelope, now: u64) -> Result<CommandResponse, Error> {
        self.event(now)?;
        let index = self.seat_index(&envelope.token)?;
        if envelope.generation != self.seats[index].as_ref().unwrap().generation {
            return Err(Error::StaleGeneration);
        }
        if envelope.sequence == 0 {
            return Err(Error::SequenceGap);
        }
        let receipt_index =
            usize::try_from(envelope.sequence - 1).map_err(|_| Error::SequenceGap)?;
        if envelope.match_id != self.match_id {
            if let Some((_, receipts)) = self
                .archived
                .iter()
                .find(|(id, _)| *id == envelope.match_id)
            {
                if receipts[index].0 != envelope.token {
                    return Err(Error::StaleMatch);
                }
                if let Some(stored) = receipts[index].1.get(receipt_index) {
                    if stored.command != envelope.command {
                        return Err(Error::SequenceConflict);
                    }
                    return Ok(CommandResponse {
                        receipt: stored.receipt.clone(),
                        snapshot: self.snapshot(index, now),
                    });
                }
            }
            return Err(Error::StaleMatch);
        }
        let total = self
            .seats
            .iter()
            .flatten()
            .map(|s| s.receipts.len())
            .sum::<usize>()
            + self
                .archived
                .iter()
                .map(|(_, rs)| rs[0].1.len() + rs[1].1.len())
                .sum::<usize>();
        let seat = self.seats[index].as_mut().unwrap();
        if let Some(stored) = seat.receipts.get(receipt_index) {
            if stored.command != envelope.command {
                return Err(Error::SequenceConflict);
            }
            return Ok(CommandResponse {
                receipt: stored.receipt.clone(),
                snapshot: self.snapshot(index, now),
            });
        }
        if envelope.sequence != seat.receipts.len() as u64 + 1 {
            return Err(Error::SequenceGap);
        }
        if total >= MAX_RECEIPTS {
            self.finish(None, EndReason::ResourceLimit, now);
            return Err(Error::Capacity);
        }
        let code = if !seat.connected {
            Code::Disconnected
        } else {
            seat.last_seen = now;
            self.apply(index, &envelope.command, now)
        };
        let seat = self.seats[index].as_mut().unwrap();
        let receipt = Receipt {
            match_id: self.match_id.clone(),
            at_ms: now,
            event_order: self.event_order,
            sequence: envelope.sequence,
            code,
            board_version: seat.version,
            board_id: seat.board_id,
            score: seat.score,
            combo: seat.combo,
            wrong: seat.wrong,
            locked_until_ms: seat.locked_until,
        };
        seat.receipts.push(StoredReceipt {
            command: envelope.command.clone(),
            receipt: receipt.clone(),
        });
        self.last_activity = now;
        if (matches!(envelope.command, Command::Rematch)
            && self.phase == Phase::Finished
            && self
                .seats
                .iter()
                .all(|s| s.as_ref().is_some_and(|s| s.rematch && s.connected)))
            || (matches!(envelope.command, Command::Ready { ready: true })
                && self.phase == Phase::Lobby
                && self.result.is_some()
                && self
                    .seats
                    .iter()
                    .all(|s| s.as_ref().is_some_and(|s| s.ready && s.connected)))
        {
            self.new_match(now)?;
        }
        Ok(CommandResponse {
            receipt,
            snapshot: self.snapshot(index, now),
        })
    }
    fn new_match(&mut self, now: u64) -> Result<(), Error> {
        let next_id = random_hex()?;
        let seed = u32::from_le_bytes(random_bytes::<4>()?);
        let puzzle = make_puzzle(self.size, seed, 0)?;
        let previous = std::array::from_fn(|i| {
            self.seats[i]
                .as_mut()
                .map(|s| (s.token.clone(), std::mem::take(&mut s.receipts)))
                .unwrap_or_default()
        });
        self.archived
            .push((std::mem::replace(&mut self.match_id, next_id), previous));
        self.seed = seed;
        self.puzzles = vec![puzzle];
        for seat in self.seats.iter_mut().flatten() {
            seat.reset(&self.puzzles[0].givens);
        }
        self.result = None;
        self.start_countdown(now)
    }
}

fn make_puzzle(size: u8, seed: u32, board_id: u32) -> Result<Puzzle, Error> {
    // Experimental blank counts. They do not establish human difficulty.
    let blanks = if size == 6 { 18 } else { 43 };
    let base = seed.wrapping_add(board_id.wrapping_mul(0x9e3779b9));
    for attempt in 0..16u32 {
        if let Ok(givens) = sudoku_core::generate_puzzle(size, base.wrapping_add(attempt), blanks) {
            let solution = sudoku_core::solve_board(&givens, size)
                .map_err(|_| Error::Internal)?
                .ok_or(Error::Internal)?;
            return Ok(Puzzle { givens, solution });
        }
    }
    Err(Error::Internal)
}

fn random_bytes<const N: usize>() -> Result<[u8; N], Error> {
    let mut bytes = [0; N];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|_| Error::Internal)?;
    Ok(bytes)
}
fn random_hex() -> Result<String, Error> {
    Ok(random_bytes::<32>()?
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
fn room_code() -> Result<String, Error> {
    const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
    let mut result = String::new();
    // Rejection sampling avoids modulo bias.
    while result.len() < 6 {
        for byte in random_bytes::<16>()? {
            let bound = 256 / ALPHABET.len() * ALPHABET.len();
            if (byte as usize) < bound {
                result.push(ALPHABET[byte as usize % ALPHABET.len()] as char);
            }
            if result.len() == 6 {
                break;
            }
        }
    }
    Ok(result)
}
fn normalized_code(code: &str) -> Result<String, Error> {
    if code.len() != 6 || !code.bytes().all(|c| c.is_ascii_alphanumeric()) {
        return Err(Error::InvalidInput);
    }
    Ok(code.to_ascii_uppercase())
}
fn valid_name(name: &str) -> bool {
    (1..=16).contains(&name.chars().count())
        && !name.chars().any(char::is_control)
        && !name.trim().is_empty()
}
fn valid_secret(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|c| c.is_ascii_hexdigit())
}

/// Timestamped events are admitted under this single lock. That admission point
/// defines arrival/order; work is then processed synchronously in the same order.
/// Timers never overtake an already admitted command.
pub struct Service {
    rooms: HashMap<String, Room>,
    create_attempts: HashMap<String, VecDeque<u64>>,
    join_attempts: HashMap<String, VecDeque<u64>>,
    last_ms: u64,
}
impl Default for Service {
    fn default() -> Self {
        Self {
            rooms: HashMap::new(),
            create_attempts: HashMap::new(),
            join_attempts: HashMap::new(),
            last_ms: 0,
        }
    }
}
impl Service {
    fn advance(&mut self, now: u64) -> Result<(), Error> {
        if now < self.last_ms {
            return Err(Error::ClockOrder);
        }
        self.last_ms = now;
        for room in self.rooms.values_mut() {
            room.advance(now);
        }
        self.rooms.retain(|_, room| {
            !room
                .closed_at
                .is_some_and(|t| now >= t.saturating_add(CLOSED_MS))
        });
        for attempts in [&mut self.create_attempts, &mut self.join_attempts] {
            attempts.retain(|_, times| {
                while times
                    .front()
                    .is_some_and(|&t| now.saturating_sub(t) >= 600_000)
                {
                    times.pop_front();
                }
                !times.is_empty()
            });
        }
        Ok(())
    }
    fn limit(
        attempts: &mut HashMap<String, VecDeque<u64>>,
        peer: &str,
        now: u64,
        max: usize,
    ) -> Result<(), Error> {
        let times = attempts.entry(peer.to_string()).or_default();
        if times.len() >= max {
            return Err(Error::RateLimited);
        }
        times.push_back(now);
        Ok(())
    }
    fn guest(&self, guest: Option<String>) -> Result<String, Error> {
        match guest {
            Some(guest) if valid_secret(&guest) => Ok(guest),
            Some(_) => Err(Error::InvalidInput),
            None => random_hex(),
        }
    }
    fn occupied(&self, guest: &str) -> bool {
        self.rooms.values().any(|room| {
            matches!(room.phase, Phase::Lobby | Phase::Countdown | Phase::Playing)
                && room.seats.iter().flatten().any(|s| s.guest == guest)
        })
    }
    pub fn create(
        &mut self,
        peer: &str,
        guest: Option<String>,
        name: String,
        mode: Mode,
        size: u8,
        now: u64,
    ) -> Result<Session, Error> {
        self.advance(now)?;
        Self::limit(&mut self.create_attempts, peer, now, 5)?;
        if !valid_name(&name) || ![6, 9].contains(&size) {
            return Err(Error::InvalidInput);
        }
        if self.rooms.len() >= MAX_ROOMS {
            return Err(Error::Capacity);
        }
        let guest = self.guest(guest)?;
        if self.occupied(&guest) {
            return Err(Error::InvalidInput);
        }
        let token = random_hex()?;
        let mut code = room_code()?;
        while self.rooms.contains_key(&code) {
            code = room_code()?;
        }
        let room = Room::new(
            code.clone(),
            mode,
            size,
            Seat::new(guest.clone(), token.clone(), name, now),
            now,
        )?;
        let snapshot = room.snapshot(0, now);
        self.rooms.insert(code.clone(), room);
        Ok(Session {
            protocol: PROTOCOL,
            guest,
            token,
            invite_path: format!("/r/{code}"),
            snapshot,
        })
    }
    pub fn join(
        &mut self,
        peer: &str,
        code: &str,
        guest: Option<String>,
        name: String,
        now: u64,
    ) -> Result<Session, Error> {
        self.advance(now)?;
        Self::limit(&mut self.join_attempts, peer, now, 10)?;
        if !valid_name(&name) {
            return Err(Error::InvalidInput);
        }
        let code = normalized_code(code)?;
        let guest = self.guest(guest)?;
        if self.occupied(&guest) {
            return Err(Error::InvalidInput);
        }
        let room = self.rooms.get_mut(&code).ok_or(Error::RoomMissing)?;
        room.event(now)?;
        if room.phase == Phase::Closed {
            return Err(Error::RoomClosed);
        }
        if room.phase != Phase::Lobby {
            return Err(Error::RoomFull);
        }
        let index = room
            .seats
            .iter()
            .position(Option::is_none)
            .ok_or(Error::RoomFull)?;
        let token = random_hex()?;
        room.seats[index] = Some(Seat::new(guest.clone(), token.clone(), name, now));
        room.last_activity = now;
        Ok(Session {
            protocol: PROTOCOL,
            guest,
            token,
            invite_path: format!("/r/{code}"),
            snapshot: room.snapshot(index, now),
        })
    }
    /// Explicit exit releases this seat. Repeats acknowledge without exposing
    /// whether a credential/room exists; they never replay a forfeit.
    pub fn leave(&mut self, code: &str, token: &str, now: u64) -> Result<(), Error> {
        self.advance(now)?;
        let code = normalized_code(code)?;
        if !valid_secret(token) {
            return Err(Error::Unauthorized);
        }
        let Some(room) = self.rooms.get_mut(&code) else {
            return Ok(());
        };
        let Ok(index) = room.seat_index(token) else {
            return Ok(());
        };
        room.event_order += 1;
        if room.phase == Phase::Playing {
            room.finish(Some(1 - index), EndReason::Resigned, now);
        } else if room.phase == Phase::Countdown {
            room.phase = Phase::Lobby;
            room.start = None;
        }
        room.seats[index] = None;
        for seat in room.seats.iter_mut().flatten() {
            seat.ready = false;
            seat.rematch = false;
        }
        room.last_activity = now;
        if room.seats.iter().all(Option::is_none) {
            room.phase = Phase::Closed;
            room.closed_at = Some(now);
        }
        Ok(())
    }

    pub fn resume(&mut self, code: &str, token: &str, now: u64) -> Result<Snapshot, Error> {
        self.advance(now)?;
        let code = normalized_code(code)?;
        let room = self.rooms.get_mut(&code).ok_or(Error::RoomMissing)?;
        room.event(now)?;
        if room.phase == Phase::Closed {
            return Err(Error::RoomClosed);
        }
        let index = room.seat_index(token)?;
        let seat = room.seats[index].as_mut().unwrap();
        seat.generation += 1;
        seat.connected = true;
        seat.last_seen = now;
        seat.disconnected_at = None;
        Ok(room.snapshot(index, now))
    }
    pub fn state(
        &mut self,
        code: &str,
        token: &str,
        generation: u64,
        now: u64,
    ) -> Result<Snapshot, Error> {
        self.advance(now)?;
        let code = normalized_code(code)?;
        let room = self.rooms.get_mut(&code).ok_or(Error::RoomMissing)?;
        room.event(now)?;
        let index = room.seat_index(token)?;
        let seat = room.seats[index].as_mut().unwrap();
        if seat.generation != generation {
            return Err(Error::StaleGeneration);
        }
        if seat.connected {
            seat.last_seen = now;
        }
        Ok(room.snapshot(index, now))
    }
    pub fn command(
        &mut self,
        code: &str,
        envelope: &Envelope,
        now: u64,
    ) -> Result<CommandResponse, Error> {
        if envelope.protocol != PROTOCOL {
            return Err(Error::Protocol);
        }
        self.advance(now)?;
        let code = normalized_code(code)?;
        self.rooms
            .get_mut(&code)
            .ok_or(Error::RoomMissing)?
            .command(envelope, now)
    }
}

pub type SharedService = Arc<Mutex<(Service, Instant)>>;
pub fn shared_service() -> SharedService {
    Arc::new(Mutex::new((Service::default(), Instant::now())))
}

pub mod http;
