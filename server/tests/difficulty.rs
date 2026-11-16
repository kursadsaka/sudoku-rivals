//! Room difficulty: an optional creation-time setting. Rooms created with it
//! serve both seats the same puzzle at that level in every board and match;
//! rooms created without it behave exactly as before.
use serde_json::{json, Value};
use sudoku_core::multiplayer::{MultiplayerClient, RequestDescriptor};
use sudoku_core::{rate_puzzle, Difficulty};
use sudoku_server::{
    Code, Command, Envelope, Error, Mode, Phase, Service, Session, Snapshot, PROTOCOL,
};

struct Bot {
    token: String,
    snapshot: Snapshot,
}
impl From<Session> for Bot {
    fn from(session: Session) -> Self {
        Self {
            token: session.token,
            snapshot: session.snapshot,
        }
    }
}
impl Bot {
    fn send(&mut self, service: &mut Service, command: Command, now: u64) -> Code {
        let envelope = Envelope {
            protocol: PROTOCOL,
            token: self.token.clone(),
            generation: self.snapshot.generation,
            match_id: self.snapshot.match_id.clone(),
            sequence: self.snapshot.own.next_sequence,
            command,
        };
        let result = service
            .command(&self.snapshot.room_code, &envelope, now)
            .unwrap();
        self.snapshot = result.snapshot;
        result.receipt.code
    }
    fn state(&mut self, service: &mut Service, now: u64) {
        self.snapshot = service
            .state(
                &self.snapshot.room_code,
                &self.token,
                self.snapshot.generation,
                now,
            )
            .unwrap();
    }
    fn level(&self) -> Option<Difficulty> {
        rate_puzzle(&self.snapshot.own.givens, self.snapshot.size)
            .unwrap()
            .level
    }
    /// Plays the current duel board to the end with correct moves, 200 ms apart.
    fn finish_duel_board(&mut self, service: &mut Service, mut now: u64) -> u64 {
        let board = self.snapshot.own.board_id;
        let solution = sudoku_core::solve_board(&self.snapshot.own.givens, self.snapshot.size)
            .unwrap()
            .unwrap();
        while self.snapshot.own.board_id == board {
            let index = self
                .snapshot
                .own
                .cells
                .iter()
                .position(|&v| v == 0)
                .unwrap();
            let command = Command::Move {
                board_id: board,
                index: index as u32,
                value: solution[index],
            };
            assert_eq!(self.send(service, command, now), Code::Ok);
            now += 200;
        }
        now
    }
}

fn started(mode: Mode, size: u8, level: Option<Difficulty>) -> (Service, Bot, Bot) {
    let mut service = Service::default();
    let mut a: Bot = service
        .create_rated("a", None, "A".into(), mode, size, level, 0)
        .unwrap()
        .into();
    // The creator's choice is visible to both players from the lobby on.
    assert_eq!(a.snapshot.difficulty, level);
    let mut b: Bot = service
        .join("b", &a.snapshot.room_code, None, "B".into(), 0)
        .unwrap()
        .into();
    assert_eq!(b.snapshot.phase, Phase::Lobby);
    assert_eq!(b.snapshot.difficulty, level);
    assert!(
        b.snapshot.own.givens.is_empty(),
        "no board before the start"
    );
    assert_eq!(
        a.send(&mut service, Command::Ready { ready: true }, 0),
        Code::Ok
    );
    assert_eq!(
        b.send(&mut service, Command::Ready { ready: true }, 0),
        Code::Ok
    );
    a.state(&mut service, 3000);
    b.state(&mut service, 3000);
    assert_eq!(a.snapshot.phase, Phase::Playing);
    (service, a, b)
}

#[test]
fn both_players_get_the_same_puzzle_at_the_room_level_in_every_mode_and_size() {
    for mode in [Mode::Race, Mode::Duel] {
        for size in [6, 9] {
            for level in Difficulty::ALL {
                let (_, a, b) = started(mode, size, Some(level));
                assert_eq!(a.snapshot.own.givens, b.snapshot.own.givens);
                assert_eq!(a.level(), Some(level), "{mode:?} {size} {level:?}");
                assert_eq!(a.snapshot.difficulty, Some(level));
                assert_eq!(b.snapshot.difficulty, Some(level));
                assert_eq!(
                    sudoku_core::count_solutions(&a.snapshot.own.givens, size, 2),
                    Ok(1)
                );
            }
        }
    }
}

#[test]
fn duel_follow_up_boards_keep_the_level_and_are_the_same_sequence_for_both() {
    for (size, level) in [(6, Difficulty::Medium), (9, Difficulty::Easy)] {
        let (mut service, mut a, mut b) = started(Mode::Duel, size, Some(level));
        let first = a.snapshot.own.givens.clone();
        let now = a.finish_duel_board(&mut service, 3000);
        assert_eq!(a.snapshot.own.board_id, 1);
        assert_ne!(a.snapshot.own.givens, first);
        assert_eq!(a.level(), Some(level), "board 1 of a {level:?} duel");
        // The slower player reaches the same second board later.
        b.state(&mut service, now);
        assert_eq!(
            (b.snapshot.own.board_id, &b.snapshot.own.givens),
            (0, &first)
        );
        b.finish_duel_board(&mut service, now);
        assert_eq!(b.snapshot.own.board_id, 1);
        assert_eq!(b.snapshot.own.givens, a.snapshot.own.givens);
        assert_eq!(b.snapshot.difficulty, Some(level));
    }
}

#[test]
fn rematch_and_return_to_lobby_start_a_new_puzzle_at_the_same_level() {
    let level = Difficulty::Hard;
    let (mut service, mut a, mut b) = started(Mode::Race, 9, Some(level));
    let (first_match, first_board) = (a.snapshot.match_id.clone(), a.snapshot.own.givens.clone());
    let cells = sudoku_core::solve_board(&first_board, 9).unwrap().unwrap();
    assert_eq!(
        a.send(
            &mut service,
            Command::Sync {
                base_version: 0,
                cells
            },
            4000
        ),
        Code::Ok
    );
    assert_eq!(a.snapshot.phase, Phase::Finished);
    b.state(&mut service, 4000);
    // Rematch: both ask, a new match starts in the same room with the same settings.
    assert_eq!(a.send(&mut service, Command::Rematch, 5000), Code::Ok);
    assert_eq!(b.send(&mut service, Command::Rematch, 5000), Code::Ok);
    a.state(&mut service, 8000);
    b.state(&mut service, 8000);
    assert_eq!(a.snapshot.phase, Phase::Playing);
    assert_ne!(a.snapshot.match_id, first_match);
    assert_ne!(a.snapshot.own.givens, first_board);
    assert_eq!(a.snapshot.own.givens, b.snapshot.own.givens);
    assert_eq!(a.level(), Some(level));
    assert_eq!(
        (a.snapshot.difficulty, b.snapshot.difficulty),
        (Some(level), Some(level))
    );
    // Back to the lobby after the next result: the level stays with the room.
    let second_board = a.snapshot.own.givens.clone();
    let cells = sudoku_core::solve_board(&second_board, 9).unwrap().unwrap();
    assert_eq!(
        b.send(
            &mut service,
            Command::Sync {
                base_version: 0,
                cells
            },
            9000
        ),
        Code::Ok
    );
    a.state(&mut service, 9000);
    assert_eq!(a.send(&mut service, Command::Lobby, 9500), Code::Ok);
    b.state(&mut service, 9500);
    assert_eq!(b.snapshot.phase, Phase::Lobby);
    assert_eq!(b.snapshot.difficulty, Some(level));
    assert_eq!(
        a.send(&mut service, Command::Ready { ready: true }, 9600),
        Code::Ok
    );
    assert_eq!(
        b.send(&mut service, Command::Ready { ready: true }, 9600),
        Code::Ok
    );
    a.state(&mut service, 12_600);
    assert_eq!(a.snapshot.phase, Phase::Playing);
    assert_ne!(a.snapshot.own.givens, second_board);
    assert_eq!(a.level(), Some(level));
}

#[test]
fn rooms_without_a_level_are_unchanged() {
    for (size, blanks) in [(6, 18), (9, 43)] {
        let (_, a, b) = started(Mode::Race, size, None);
        assert_eq!(a.snapshot.own.givens, b.snapshot.own.givens);
        assert_eq!(
            a.snapshot.own.givens.iter().filter(|&&v| v == 0).count(),
            blanks
        );
        // The v1 snapshot shape: no difficulty key at all.
        let wire = serde_json::to_value(&a.snapshot).unwrap();
        assert!(wire.get("difficulty").is_none());
        // And a v1 snapshot without the key still parses.
        let parsed: Snapshot = serde_json::from_value(wire).unwrap();
        assert_eq!(parsed.difficulty, None);
    }
    let mut service = Service::default();
    let plain = service
        .create("a", None, "A".into(), Mode::Duel, 6, 0)
        .unwrap();
    assert_eq!(plain.snapshot.difficulty, None);
}

#[test]
fn a_lost_create_is_replayed_only_for_the_same_level() {
    let mut service = Service::default();
    let first = service
        .create_rated(
            "a",
            None,
            "A".into(),
            Mode::Race,
            9,
            Some(Difficulty::Medium),
            0,
        )
        .unwrap();
    let again = service
        .create_rated(
            "a",
            Some(first.guest.clone()),
            "A".into(),
            Mode::Race,
            9,
            Some(Difficulty::Medium),
            1,
        )
        .unwrap();
    assert_eq!(
        (again.token, again.snapshot.room_code),
        (first.token, first.snapshot.room_code.clone())
    );
    // A different level is a different request, not a retry: the guest is still seated.
    for other in [Some(Difficulty::Hard), None] {
        let refused = service.create_rated(
            "a",
            Some(first.guest.clone()),
            "A".into(),
            Mode::Race,
            9,
            other,
            2,
        );
        assert!(matches!(refused, Err(Error::InvalidInput)));
    }
}

fn request(client: &mut MultiplayerClient, action: Value, now: u64) -> RequestDescriptor {
    let effect: Value =
        serde_json::from_str(&client.dispatch_json(&action.to_string(), now).unwrap()).unwrap();
    serde_json::from_value(effect["request"].clone()).unwrap()
}

#[test]
fn shared_client_sends_the_level_only_when_chosen_and_checks_the_answer() {
    let mut service = Service::default();
    // Chosen: the request carries it, the room has it, the view exposes it, a save keeps it.
    let mut client = MultiplayerClient::new(1);
    let wire = request(
        &mut client,
        json!({"type":"create","name":"A","mode":"duel","size":6,"difficulty":"hard"}),
        0,
    );
    let body: Value = serde_json::from_str(&wire.body).unwrap();
    assert_eq!(body["difficulty"], "hard");
    let session = service
        .create_rated(
            "a",
            body["guest"].as_str().map(str::to_owned),
            "A".into(),
            Mode::Duel,
            6,
            serde_json::from_value(body["difficulty"].clone()).unwrap(),
            0,
        )
        .unwrap();
    let answer = serde_json::to_string(&session).unwrap();
    client
        .dispatch_json(
            &json!({"type":"receive","request_id":wire.id,"status":200,"response":answer})
                .to_string(),
            1,
        )
        .unwrap();
    let view: Value = serde_json::from_str(&client.view_json(1).unwrap()).unwrap();
    assert_eq!(view["snapshot"]["difficulty"], "hard");
    let restored = MultiplayerClient::load(&client.save().unwrap()).unwrap();
    let view: Value = serde_json::from_str(&restored.view_json(1).unwrap()).unwrap();
    assert_eq!(view["snapshot"]["difficulty"], "hard");

    // A room that does not have the requested level is not accepted as the answer.
    let mut strict = MultiplayerClient::new(2);
    let wire = request(
        &mut strict,
        json!({"type":"create","name":"B","mode":"race","size":6,"difficulty":"medium"}),
        0,
    );
    let body: Value = serde_json::from_str(&wire.body).unwrap();
    let unrated = service
        .create(
            "b",
            body["guest"].as_str().map(str::to_owned),
            "B".into(),
            Mode::Race,
            6,
            0,
        )
        .unwrap();
    let answer = serde_json::to_string(&unrated).unwrap();
    assert!(strict
        .dispatch_json(
            &json!({"type":"receive","request_id":wire.id,"status":200,"response":answer})
                .to_string(),
            1,
        )
        .is_err());

    // Not chosen (every existing caller): the request is the v1 body, key for key.
    let mut legacy = MultiplayerClient::new(3);
    let wire = request(
        &mut legacy,
        json!({"type":"create","name":"C","mode":"race","size":9}),
        0,
    );
    let body: Value = serde_json::from_str(&wire.body).unwrap();
    let mut keys: Vec<&str> = body
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["guest", "mode", "name", "protocol", "size"]);
    // An unknown level is refused before anything is sent.
    assert!(legacy
        .dispatch_json(
            &json!({"type":"create","name":"C","mode":"race","size":9,"difficulty":"expert"})
                .to_string(),
            1,
        )
        .is_err());
}
