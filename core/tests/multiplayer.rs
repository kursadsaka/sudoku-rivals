use serde_json::{json, Value};
use sudoku_core::multiplayer::{
    parse_invite, validate_snapshot, ClientError, MultiplayerClient, RequestDescriptor,
};
use sudoku_core::protocol::*;

fn act(client: &mut MultiplayerClient, action: Value, now: u64) -> Value {
    serde_json::from_str(&client.dispatch_json(&action.to_string(), now).unwrap()).unwrap()
}
fn request(client: &mut MultiplayerClient, action: Value, now: u64) -> RequestDescriptor {
    serde_json::from_value(act(client, action, now)["request"].clone()).unwrap()
}
fn view(client: &MultiplayerClient, now: u64) -> Value {
    serde_json::from_str(&client.view_json(now).unwrap()).unwrap()
}
fn response(
    client: &mut MultiplayerClient,
    wire: &RequestDescriptor,
    body: Value,
    now: u64,
) -> Value {
    act(
        client,
        json!({"type":"receive","request_id":wire.id,"status":200,"response":body.to_string()}),
        now,
    )
}
fn lobby(mode: Mode, size: u8) -> Snapshot {
    Snapshot {
        protocol: 1,
        room_code: "ABC234".into(),
        match_id: "a".repeat(64),
        generation: 1,
        mode,
        size,
        phase: Phase::Lobby,
        server_ms: 0,
        event_order: 0,
        start_ms: None,
        end_ms: None,
        own: OwnState {
            seat: 0,
            name: "A".into(),
            ready: false,
            connected: true,
            next_sequence: 1,
            board_id: 0,
            board_version: 0,
            givens: vec![],
            cells: vec![],
            score: 0,
            combo: 0,
            wrong: 0,
            locked_until_ms: 0,
            rematch: false,
        },
        opponent: None,
        result: None,
    }
}
fn playing(mode: Mode, size: u8) -> (MultiplayerClient, Snapshot) {
    let mut client = MultiplayerClient::new(42);
    let wire = request(
        &mut client,
        json!({"type":"create","name":"A","mode":mode,"size":size}),
        0,
    );
    let mut snapshot = lobby(mode, size);
    let session = Session {
        protocol: 1,
        guest: "b".repeat(64),
        token: "c".repeat(64),
        invite_path: "/r/ABC234".into(),
        snapshot: snapshot.clone(),
    };
    response(
        &mut client,
        &wire,
        serde_json::to_value(session).unwrap(),
        0,
    );
    snapshot.phase = Phase::Playing;
    snapshot.start_ms = Some(3000);
    snapshot.server_ms = 3000;
    snapshot.event_order = 1;
    snapshot.end_ms = if mode == Mode::Duel {
        Some(93000)
    } else {
        None
    };
    let givens = sudoku_core::generate_puzzle(size, 42, if size == 6 { 18 } else { 43 }).unwrap();
    snapshot.own.givens = givens.clone();
    snapshot.own.cells = givens;
    snapshot.opponent = Some(OpponentState {
        name: "B".into(),
        ready: true,
        connected: true,
        progress_percent: if mode == Mode::Race { Some(0) } else { None },
        score: if mode == Mode::Duel { Some(0) } else { None },
        rematch: false,
    });
    let wire = request(&mut client, json!({"type":"poll"}), 3000);
    response(
        &mut client,
        &wire,
        serde_json::to_value(&snapshot).unwrap(),
        3000,
    );
    (client, snapshot)
}

#[test]
fn public_view_and_debug_never_expose_auth_and_invites_are_canonical() {
    let (client, _) = playing(Mode::Race, 6);
    let visible = client.view_json(3000).unwrap();
    for secret in ["b".repeat(64), "c".repeat(64)] {
        assert!(!visible.contains(&secret));
        assert!(!format!("{client:?}").contains(&secret));
    }
    assert!(view(&client, 3000)["race"]["solo"].is_object());
    for invite in [
        "abc234",
        "/r/ABC234",
        "https://example.test/r/abc234",
        "http://127.0.0.1:3000/r/ABC234",
    ] {
        assert_eq!(parse_invite(invite), Ok("ABC234".into()));
    }
    for invite in [
        "/r/ABC234?token=x",
        "https://user:secret@host/r/ABC234",
        "/x/ABC234",
        "/r/ABC234/",
        "/r/ABC230",
        "javascript:ABC234",
    ] {
        assert!(parse_invite(invite).is_err(), "{invite}");
    }
}

#[test]
fn untrusted_snapshots_are_checked_before_active_state_changes() {
    for mode in [Mode::Race, Mode::Duel] {
        let (mut client, snapshot) = playing(mode, 9);
        let wire = request(&mut client, json!({"type":"poll"}), 3001);
        let original = serde_json::to_value(snapshot).unwrap();
        let before = client.save().unwrap();
        for (path, bad) in [
            ("/protocol", json!(2)),
            ("/size", json!(8)),
            ("/own/seat", json!(2)),
            ("/generation", json!(0)),
            ("/own/next_sequence", json!(0)),
            ("/server_ms", json!(9_007_199_254_740_992u64)),
            ("/own/board_version", json!(9_007_199_254_740_992u64)),
            ("/own/cells", json!([])),
            ("/own/cells/0", json!(10)),
            ("/own/givens/0", json!(10)),
            ("/start_ms", json!(4000)),
            ("/opponent/progress_percent", json!(101)),
        ] {
            let mut bad_snapshot = original.clone();
            *bad_snapshot.pointer_mut(path).unwrap() = bad;
            let action = json!({"type":"receive","request_id":wire.id,"status":200,"response":bad_snapshot.to_string()});
            assert!(
                client.dispatch_json(&action.to_string(), 3001).is_err(),
                "{mode:?} {path}"
            );
            assert_eq!(client.save().unwrap(), before);
        }
        assert!(client
            .dispatch_json(
                &json!({"type":"receive","request_id":wire.id,"status":200,"response":"{"})
                    .to_string(),
                3001
            )
            .is_err());
        assert_eq!(client.save().unwrap(), before);
        response(&mut client, &wire, original, 3001);
    }
}

#[test]
fn future_bad_and_oversized_saves_are_rejected_and_load_resets_clock_epoch() {
    let (mut client, _) = playing(Mode::Race, 6);
    act(
        &mut client,
        json!({"type":"edit","command":{"type":"toggle_note","index":0,"value":1}}),
        3000,
    );
    let saved = client.save().unwrap();
    let original: Value = serde_json::from_str(&saved).unwrap();
    for (path, value) in [
        ("/version", json!(2)),
        ("/auth/token", json!("bad")),
        ("/next_id", json!(0)),
        ("/ack_cells", json!([])),
        ("/auth/snapshot/generation", json!(0)),
        (
            "/auth/snapshot/own/next_sequence",
            json!(9_007_199_254_740_992u64),
        ),
    ] {
        let mut bad = original.clone();
        *bad.pointer_mut(path).unwrap() = value;
        assert!(MultiplayerClient::load(&bad.to_string()).is_err(), "{path}");
    }
    for bad in ["null", "{}", "{", &" ".repeat(1_048_577)] {
        assert!(MultiplayerClient::load(bad).is_err());
    }
    let mut loaded = MultiplayerClient::load(&saved).unwrap();
    assert!(view(&loaded, 0)["estimated_server_ms"].is_null());
    assert_eq!(view(&loaded, 0)["race"]["notes"][0], json!([1]));
    assert_eq!(
        loaded.dispatch_json(r#"{"type":"poll"}"#, 0),
        Err(ClientError::ResumeRequired)
    );
    assert!(act(&mut loaded, json!({"type":"resume"}), 0)["request"].is_object());
}

#[test]
fn stale_transport_and_generation_responses_do_not_overwrite_current_state() {
    let (mut client, mut snapshot) = playing(Mode::Duel, 6);
    let abandoned = request(&mut client, json!({"type":"poll"}), 3001);
    let resume = request(&mut client, json!({"type":"resume"}), 3002);
    snapshot.generation = 2;
    snapshot.server_ms = 3002;
    snapshot.event_order = 2;
    response(
        &mut client,
        &resume,
        serde_json::to_value(&snapshot).unwrap(),
        3002,
    );
    let saved = client.save().unwrap();
    assert_eq!(
        response(
            &mut client,
            &abandoned,
            serde_json::to_value(&snapshot).unwrap(),
            3003
        )["changed"],
        false
    );
    assert_eq!(client.save().unwrap(), saved);
    let wire = request(&mut client, json!({"type":"poll"}), 3004);
    snapshot.generation = 1;
    assert_eq!(
        response(
            &mut client,
            &wire,
            serde_json::to_value(&snapshot).unwrap(),
            3004
        )["changed"],
        false
    );
    snapshot.generation = 2;
    snapshot.server_ms = 3004;
    snapshot.event_order = 3;
    response(
        &mut client,
        &wire,
        serde_json::to_value(snapshot).unwrap(),
        3004,
    );
    assert_eq!(view(&client, 3004)["snapshot"]["generation"], 2);
}

#[test]
fn deterministic_retry_jitter_and_exact_descriptor_are_bounded() {
    let mut client = MultiplayerClient::new(12);
    let first = request(
        &mut client,
        json!({"type":"create","name":"A","mode":"race","size":6}),
        0,
    );
    let mut now = 0;
    for base in [500u64, 1000, 2000, 4000, 5000, 5000] {
        act(&mut client, json!({"type":"network_failed"}), now);
        let due = view(&client, now)["retry_at_ms"].as_u64().unwrap();
        assert!((base * 80 / 100..=base * 120 / 100).contains(&(due - now)));
        assert_eq!(
            client.dispatch_json(r#"{"type":"retry"}"#, due - 1),
            Err(ClientError::RetryNotDue)
        );
        assert_eq!(request(&mut client, json!({"type":"retry"}), due), first);
        now = due;
    }
}

#[test]
fn race_conflicts_are_legal_but_server_givens_and_final_result_are_validated() {
    let (_, snapshot) = playing(Mode::Race, 6);
    let mut conflict = snapshot.clone();
    let index = conflict.own.givens.iter().position(|&v| v == 0).unwrap();
    let row = index / 6;
    let duplicate = conflict.own.cells[row * 6..row * 6 + 6]
        .iter()
        .copied()
        .find(|&v| v != 0)
        .unwrap();
    conflict.own.cells[index] = duplicate;
    assert!(validate_snapshot(&conflict).is_ok());
    let given = conflict.own.givens.iter().position(|&v| v != 0).unwrap();
    conflict.own.cells[given] = 0;
    assert!(validate_snapshot(&conflict).is_err());
    let mut invalid_result = snapshot;
    invalid_result.phase = Phase::Finished;
    invalid_result.result = Some(MatchResult {
        winner: Some(0),
        reason: EndReason::Completed,
        at_ms: 3000,
    });
    assert!(validate_snapshot(&invalid_result).is_err());
}

#[test]
fn action_and_saved_unit_commands_reject_unknown_fields() {
    let (mut client, _) = playing(Mode::Duel, 6);
    let before = client.save().unwrap();
    for action in [
        json!({"type":"resume","extra":true}),
        json!({"type":"poll","extra":true}),
        json!({"type":"sync","extra":true}),
        json!({"type":"leave","extra":true}),
        json!({"type":"network_failed","extra":true}),
        json!({"type":"retry","extra":true}),
        json!({"type":"command","command":{"type":"resign","extra":true}}),
    ] {
        assert_eq!(
            client.dispatch_json(&action.to_string(), 3000),
            Err(ClientError::InvalidCommand)
        );
        assert_eq!(client.save().unwrap(), before);
    }
    request(
        &mut client,
        json!({"type":"command","command":{"type":"resign"}}),
        3000,
    );
    let mut saved: Value = serde_json::from_str(&client.save().unwrap()).unwrap();
    saved["pending"]["envelope"]["command"]["extra"] = json!(true);
    assert!(MultiplayerClient::load(&saved.to_string()).is_err());
}

#[test]
fn forged_receipt_identity_is_atomic_and_pending_saves_restore_for_resume() {
    let (mut client, mut snapshot) = playing(Mode::Duel, 6);
    let wire = request(
        &mut client,
        json!({"type":"command","command":{"type":"resign"}}),
        3000,
    );
    let saved = client.save().unwrap();
    assert!(MultiplayerClient::load(&saved).is_ok());
    let mut corrupt: Value = serde_json::from_str(&saved).unwrap();
    corrupt["pending"]["envelope"]["generation"] = json!(2);
    assert!(MultiplayerClient::load(&corrupt.to_string()).is_err());
    snapshot.phase = Phase::Finished;
    snapshot.event_order = 2;
    snapshot.own.next_sequence = 2;
    snapshot.result = Some(MatchResult {
        winner: Some(1),
        reason: EndReason::Resigned,
        at_ms: 3000,
    });
    let receipt = Receipt {
        match_id: "d".repeat(64),
        at_ms: 3000,
        event_order: 2,
        sequence: 1,
        code: Code::Ok,
        board_version: 0,
        board_id: 0,
        score: 0,
        combo: 0,
        wrong: 0,
        locked_until_ms: 0,
    };
    let result = CommandResponse { receipt, snapshot };
    let action = json!({"type":"receive","request_id":wire.id,"status":200,"response":serde_json::to_string(&result).unwrap()});
    assert_eq!(
        client.dispatch_json(&action.to_string(), 3000),
        Err(ClientError::InvalidSnapshot)
    );
    assert_eq!(client.save().unwrap(), saved);
    assert!(!format!("{wire:?}").contains(&"c".repeat(64)));
}

#[test]
fn clock_uses_minimum_rtt_in_recent_samples_and_is_only_a_display_estimate() {
    let (mut client, mut snapshot) = playing(Mode::Duel, 6);
    for (index, rtt) in [10, 20, 30, 40, 50].into_iter().enumerate() {
        let sent = 4000 + index as u64 * 1000;
        let wire = request(&mut client, json!({"type":"poll"}), sent);
        snapshot.server_ms = sent + 100 + rtt / 2;
        snapshot.event_order += 1;
        response(
            &mut client,
            &wire,
            serde_json::to_value(&snapshot).unwrap(),
            sent + rtt,
        );
    }
    let display = view(&client, 9000);
    assert_eq!(display["estimated_server_ms"], 9100);
    assert_eq!(display["remaining_ms"], 83900);
    assert!(display["snapshot"]["result"].is_null());
}

#[test]
fn unsafe_numeric_inputs_and_impossible_duel_end_are_rejected() {
    let (mut client, mut snapshot) = playing(Mode::Duel, 6);
    let before = client.save().unwrap();
    for (id, status) in [(9_007_199_254_740_992u64, 200), (1, 0), (1, 600)] {
        let action = json!({"type":"receive","request_id":id,"status":status,"response":"{}"});
        assert!(client.dispatch_json(&action.to_string(), 3000).is_err());
        assert_eq!(client.save().unwrap(), before);
    }
    snapshot.server_ms = 9_007_199_254_740_991;
    snapshot.start_ms = Some(snapshot.server_ms);
    snapshot.end_ms = None;
    assert!(validate_snapshot(&snapshot).is_err());
}

#[test]
fn a_stale_network_failure_callback_cannot_disconnect_a_new_generation() {
    let (mut client, mut snapshot) = playing(Mode::Duel, 6);
    let old = request(&mut client, json!({"type":"poll"}), 3001);
    let resume = request(&mut client, json!({"type":"resume"}), 3002);
    snapshot.generation = 2;
    snapshot.server_ms = 3002;
    snapshot.event_order = 2;
    response(
        &mut client,
        &resume,
        serde_json::to_value(&snapshot).unwrap(),
        3002,
    );
    let before = client.save().unwrap();
    assert_eq!(
        act(
            &mut client,
            json!({"type":"network_failed","request_id":old.id}),
            3003
        )["changed"],
        false
    );
    assert_eq!(client.save().unwrap(), before);
    assert_eq!(view(&client, 3003)["connection"], "connected");
}

#[test]
fn lobby_and_countdown_poll_fast_enough_to_observe_the_three_second_start() {
    let mut client = MultiplayerClient::new(1);
    let wire = request(
        &mut client,
        json!({"type":"create","name":"A","mode":"race","size":6}),
        0,
    );
    let session = Session {
        protocol: 1,
        guest: "b".repeat(64),
        token: "c".repeat(64),
        invite_path: "/r/ABC234".into(),
        snapshot: lobby(Mode::Race, 6),
    };
    response(
        &mut client,
        &wire,
        serde_json::to_value(session).unwrap(),
        0,
    );
    assert_eq!(view(&client, 499)["poll_due"], false);
    assert_eq!(view(&client, 500)["poll_due"], true);
    let (mut client, mut snapshot) = playing(Mode::Duel, 6);
    assert_eq!(view(&client, 3999)["poll_due"], false);
    assert_eq!(view(&client, 4000)["poll_due"], true);
    let wire = request(&mut client, json!({"type":"poll"}), 4000);
    snapshot.match_id = "d".repeat(64);
    snapshot.phase = Phase::Countdown;
    snapshot.own.givens.clear();
    snapshot.own.cells.clear();
    snapshot.start_ms = Some(7000);
    snapshot.end_ms = Some(97000);
    snapshot.server_ms = 4000;
    snapshot.event_order = 2;
    response(
        &mut client,
        &wire,
        serde_json::to_value(&snapshot).unwrap(),
        4000,
    );
    assert_eq!(view(&client, 4499)["poll_due"], false);
    assert_eq!(view(&client, 4500)["poll_due"], true);
    assert!(view(&client, 7000)["snapshot"]["result"].is_null());
}

#[test]
fn missing_duel_opponent_score_is_rejected_before_presentation() {
    let (mut client, snapshot) = playing(Mode::Duel, 6);
    let wire = request(&mut client, json!({"type":"poll"}), 3001);
    let before = client.save().unwrap();
    for missing in [true, false] {
        let mut body = serde_json::to_value(&snapshot).unwrap();
        if missing {
            body["opponent"].as_object_mut().unwrap().remove("score");
        } else {
            body["opponent"]["score"] = Value::Null;
        }
        let action =
            json!({"type":"receive","request_id":wire.id,"status":200,"response":body.to_string()});
        assert!(matches!(
            client.dispatch_json(&action.to_string(), 3001),
            Err(ClientError::InvalidSnapshot)
        ));
        assert_eq!(client.save().unwrap(), before);
    }
}
