//! Serialized core-client <-> authoritative Service tests without HTTP or UI.
use serde_json::{json, Value};
use sudoku_core::multiplayer::{ClientError, MultiplayerClient, RequestDescriptor};
use sudoku_core::protocol::*;
use sudoku_server::Service;

fn act(client: &mut MultiplayerClient, action: Value, now: u64) -> Value {
    serde_json::from_str(&client.dispatch_json(&action.to_string(), now).unwrap()).unwrap()
}
fn req(client: &mut MultiplayerClient, action: Value, now: u64) -> RequestDescriptor {
    serde_json::from_value(act(client, action, now)["request"].clone()).unwrap()
}
fn view(client: &MultiplayerClient, now: u64) -> Value {
    serde_json::from_str(&client.view_json(now).unwrap()).unwrap()
}
fn snapshot(client: &MultiplayerClient, now: u64) -> Snapshot {
    serde_json::from_value(view(client, now)["snapshot"].clone()).unwrap()
}
fn exchange(service: &mut Service, wire: &RequestDescriptor, now: u64) -> (u16, String) {
    assert_eq!(wire.method, "POST");
    let body: Value = serde_json::from_str(&wire.body).unwrap();
    let code = wire.path.split('/').nth(3).unwrap_or("");
    let result: Result<Value, Error> = if wire.path == "/v1/rooms" {
        service
            .create(
                "creator",
                body["guest"].as_str().map(str::to_owned),
                body["name"].as_str().unwrap().into(),
                serde_json::from_value(body["mode"].clone()).unwrap(),
                body["size"].as_u64().unwrap() as u8,
                now,
            )
            .map(|s| serde_json::to_value(s).unwrap())
    } else if wire.path.ends_with("/join") {
        service
            .join(
                "joiner",
                code,
                body["guest"].as_str().map(str::to_owned),
                body["name"].as_str().unwrap().into(),
                now,
            )
            .map(|s| serde_json::to_value(s).unwrap())
    } else if wire.path.ends_with("/resume") {
        service
            .resume(code, body["token"].as_str().unwrap(), now)
            .map(|s| serde_json::to_value(s).unwrap())
    } else if wire.path.ends_with("/state") {
        service
            .state(
                code,
                body["token"].as_str().unwrap(),
                body["generation"].as_u64().unwrap(),
                now,
            )
            .map(|s| serde_json::to_value(s).unwrap())
    } else if wire.path.ends_with("/leave") {
        service
            .leave(code, body["token"].as_str().unwrap(), now)
            .map(|_| json!({"protocol":1,"left":true}))
    } else {
        let envelope: Envelope = serde_json::from_str(&wire.body).unwrap();
        service
            .command(code, &envelope, now)
            .map(|s| serde_json::to_value(s).unwrap())
    };
    match result {
        Ok(body) => (200, body.to_string()),
        Err(error) => {
            let status = match error {
                Error::Unauthorized => 401,
                Error::RoomMissing => 404,
                Error::RoomClosed => 410,
                Error::RateLimited | Error::Capacity => 429,
                Error::Internal => 500,
                _ => 409,
            };
            (status, json!({"protocol":1,"error":error}).to_string())
        }
    }
}
fn receive(
    client: &mut MultiplayerClient,
    wire: &RequestDescriptor,
    response: (u16, String),
    now: u64,
) -> Value {
    act(
        client,
        json!({"type":"receive","request_id":wire.id,"status":response.0,"response":response.1}),
        now,
    )
}
fn drive(service: &mut Service, client: &mut MultiplayerClient, action: Value, now: u64) -> Value {
    let wire = req(client, action, now);
    let response = exchange(service, &wire, now);
    receive(client, &wire, response, now)
}
fn started(mode: Mode, size: u8) -> (Service, MultiplayerClient, MultiplayerClient) {
    let mut service = Service::default();
    let mut a = MultiplayerClient::new(1);
    let mut b = MultiplayerClient::new(2);
    drive(
        &mut service,
        &mut a,
        json!({"type":"create","name":"A","mode":mode,"size":size}),
        0,
    );
    let code = snapshot(&a, 0).room_code;
    drive(
        &mut service,
        &mut b,
        json!({"type":"join","invite":format!("/r/{code}"),"name":"B"}),
        0,
    );
    for client in [&mut a, &mut b] {
        drive(
            &mut service,
            client,
            json!({"type":"command","command":{"type":"ready","ready":true}}),
            0,
        );
    }
    for client in [&mut a, &mut b] {
        drive(&mut service, client, json!({"type":"poll"}), 3000);
    }
    (service, a, b)
}
fn set(client: &mut MultiplayerClient, index: usize, value: u8, now: u64) {
    act(
        client,
        json!({"type":"edit","command":{"type":"set","index":index,"value":value}}),
        now,
    );
}
fn next_move(client: &MultiplayerClient, now: u64, wrong: bool) -> Value {
    let snapshot = snapshot(client, now);
    let index = snapshot.own.cells.iter().position(|&v| v == 0).unwrap();
    let correct = sudoku_core::solve_board(&snapshot.own.givens, snapshot.size)
        .unwrap()
        .unwrap()[index];
    json!({"type":"command","command":{"type":"move","board_id":snapshot.own.board_id,"index":index,"value":if wrong {correct%snapshot.size+1}else{correct}}})
}

#[test]
fn shared_clients_finish_both_modes_in_both_sizes() {
    for size in [6, 9] {
        for mode in [Mode::Race, Mode::Duel] {
            let (mut service, mut a, mut b) = started(mode, size);
            let initial = snapshot(&a, 3000);
            assert_eq!(initial.own.givens, snapshot(&b, 3000).own.givens);
            let finish_at = if mode == Mode::Race {
                let solved = sudoku_core::solve_board(&initial.own.givens, size)
                    .unwrap()
                    .unwrap();
                for (index, (&given, &value)) in initial.own.givens.iter().zip(&solved).enumerate()
                {
                    if given == 0 {
                        set(&mut a, index, value, 4000);
                    }
                }
                drive(&mut service, &mut a, json!({"type":"sync"}), 4000);
                4000
            } else {
                let blanks = initial.own.cells.iter().filter(|&&v| v == 0).count();
                for i in 0..blanks {
                    let now = 4000 + i as u64 * 200;
                    let action = next_move(&a, now, false);
                    drive(&mut service, &mut a, action, now);
                }
                assert_eq!(snapshot(&a, 13000).own.board_id, 1);
                drive(&mut service, &mut a, json!({"type":"poll"}), 93000);
                93000
            };
            drive(&mut service, &mut b, json!({"type":"poll"}), finish_at);
            let a_view = view(&a, finish_at);
            let b_view = view(&b, finish_at);
            assert_eq!(a_view["snapshot"]["result"], b_view["snapshot"]["result"]);
            assert_eq!(a_view["snapshot"]["result"]["winner"], 0);
            assert_eq!(a_view["snapshot"]["phase"], "finished");
        }
    }
}

#[test]
fn lost_wrong_response_retries_exact_body_once_then_resume_discards_unprocessed_duel() {
    let (mut service, mut a, _) = started(Mode::Duel, 6);
    let action = next_move(&a, 4000, true);
    let original = req(&mut a, action, 4000);
    let first = exchange(&mut service, &original, 4000); // The response is lost.
    act(&mut a, json!({"type":"network_failed"}), 4000);
    let due = view(&a, 4000)["retry_at_ms"].as_u64().unwrap();
    let retry = req(&mut a, json!({"type":"retry"}), due);
    assert_eq!(original, retry);
    let again = exchange(&mut service, &retry, due);
    assert_eq!(
        serde_json::from_str::<Value>(&first.1).unwrap()["receipt"],
        serde_json::from_str::<Value>(&again.1).unwrap()["receipt"]
    );
    receive(&mut a, &retry, again, due);
    assert_eq!(view(&a, due)["snapshot"]["own"]["wrong"], 1);
    let action = next_move(&a, due, true);
    let unprocessed = req(&mut a, action, due);
    let resume = req(&mut a, json!({"type":"resume"}), 6000);
    let response = exchange(&mut service, &resume, 6000);
    receive(&mut a, &resume, response, 6000);
    assert_eq!(view(&a, 6000)["pending_command"], false);
    assert_eq!(view(&a, 6000)["snapshot"]["generation"], 2);
    let stale = exchange(&mut service, &unprocessed, 6000);
    assert_eq!(stale.0, 409);
    assert_eq!(receive(&mut a, &unprocessed, stale, 6000)["changed"], false);
    assert_eq!(view(&a, 6000)["snapshot"]["own"]["wrong"], 1);
}

#[test]
fn forty_offline_edits_become_one_sync_and_inflight_edits_survive_ack() {
    for size in [6, 9] {
        let (mut service, mut a, _) = started(Mode::Race, size);
        let initial = snapshot(&a, 3000);
        let blanks: Vec<usize> = initial
            .own
            .givens
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| (v == 0).then_some(i))
            .collect();
        act(&mut a, json!({"type":"network_failed"}), 4000);
        for i in 0..40 {
            set(&mut a, blanks[0], (i % size as usize + 1) as u8, 4000);
        }
        drive(&mut service, &mut a, json!({"type":"resume"}), 5000);
        let wire = req(&mut a, json!({"type":"sync"}), 5000);
        let body: Value = serde_json::from_str(&wire.body).unwrap();
        assert_eq!(body["command"]["base_version"], 0);
        assert_eq!(body["sequence"], initial.own.next_sequence);
        set(&mut a, blanks[1], size, 5000); // More local work while sync is in flight.
        let response = exchange(&mut service, &wire, 5000);
        receive(&mut a, &wire, response, 5000);
        let current = view(&a, 5000);
        assert_eq!(current["race"]["cells"][blanks[1]], size);
        assert_eq!(current["snapshot"]["own"]["cells"][blanks[1]], 0);
        assert_eq!(current["race"]["dirty"], true);
        assert_eq!(current["reconcile_required"], false);
        assert_eq!(current["race"]["can_undo"], true);
        let next = req(&mut a, json!({"type":"sync"}), 5200);
        let nextbody: Value = serde_json::from_str(&next.body).unwrap();
        assert_eq!(nextbody["command"]["base_version"], 1);
        let response = exchange(&mut service, &next, 5200);
        receive(&mut a, &next, response, 5200);
        assert_eq!(view(&a, 5200)["race"]["dirty"], false);
        assert_eq!(view(&a, 5200)["race"]["can_undo"], true);
    }
}

#[test]
fn processed_pending_is_reconciled_from_poll_without_resending_wrong_penalty() {
    let (mut service, mut a, _) = started(Mode::Duel, 9);
    let action = next_move(&a, 4000, true);
    let wire = req(&mut a, action, 4000);
    exchange(&mut service, &wire, 4000);
    act(&mut a, json!({"type":"network_failed"}), 4000);
    drive(&mut service, &mut a, json!({"type":"poll"}), 4500);
    let state = view(&a, 4500);
    assert_eq!(state["pending_command"], false);
    assert_eq!(state["snapshot"]["own"]["wrong"], 1);
    assert!(a.dispatch_json(r#"{"type":"retry"}"#, 5000).is_err());
}

#[test]
fn divergent_server_board_requires_explicit_reconcile_and_restart_keeps_draft() {
    let (mut service, mut a, _) = started(Mode::Race, 6);
    let initial = snapshot(&a, 3000);
    let blanks: Vec<usize> = initial
        .own
        .givens
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (v == 0).then_some(i))
        .collect();
    set(&mut a, blanks[0], 1, 4000);
    let saved: Value = serde_json::from_str(&a.save().unwrap()).unwrap();
    let mut remote = initial.own.cells.clone();
    remote[blanks[1]] = 2;
    let envelope = Envelope {
        protocol: 1,
        token: saved["auth"]["token"].as_str().unwrap().into(),
        generation: initial.generation,
        match_id: initial.match_id,
        sequence: initial.own.next_sequence,
        command: Command::Sync {
            base_version: 0,
            cells: remote,
        },
    };
    service
        .command(&initial.room_code, &envelope, 4000)
        .unwrap();
    drive(&mut service, &mut a, json!({"type":"poll"}), 4000);
    let state = view(&a, 4000);
    assert_eq!(state["connection"], "reconcile");
    assert_eq!(state["race"]["cells"][blanks[0]], 1);
    assert_eq!(state["race"]["cells"][blanks[1]], 0);
    assert_eq!(state["snapshot"]["own"]["cells"][blanks[1]], 2);
    assert_eq!(
        a.dispatch_json(r#"{"type":"sync"}"#, 4000),
        Err(ClientError::ReconcileRequired)
    );
    act(&mut a, json!({"type":"reconcile","keep_local":true}), 4000);
    let wire = req(&mut a, json!({"type":"sync"}), 5000);
    let body: Value = serde_json::from_str(&wire.body).unwrap();
    assert_eq!(body["command"]["base_version"], 1);
    let response = exchange(&mut service, &wire, 5000);
    receive(&mut a, &wire, response, 5000);
    set(&mut a, blanks[1], 3, 5000);
    let local = view(&a, 5000)["race"].clone();
    let mut restarted = Service::default();
    drive(&mut restarted, &mut a, json!({"type":"poll"}), 6000);
    assert_eq!(view(&a, 6000)["connection"], "session_lost");
    assert_eq!(view(&a, 6000)["race"], local);
    assert!(view(&a, 6000)["snapshot"]["result"].is_null());
}

#[test]
fn estimated_deadline_never_invents_result_and_server_boundary_wins() {
    let (mut service, mut a, _) = started(Mode::Duel, 6);
    for now in (13000..93000).step_by(10000) {
        drive(&mut service, &mut a, json!({"type":"poll"}), now);
    }
    let action = next_move(&a, 92999, false);
    let wire = req(&mut a, action, 92999);
    let response = exchange(&mut service, &wire, 92999);
    receive(&mut a, &wire, response, 93010);
    assert_eq!(view(&a, 93010)["remaining_ms"], 0);
    assert!(view(&a, 93010)["snapshot"]["result"].is_null());
    let action = next_move(&a, 93010, false);
    let wire = req(&mut a, action, 93010);
    let response = exchange(&mut service, &wire, 93000);
    receive(&mut a, &wire, response, 93010);
    assert_eq!(view(&a, 93010)["last_receipt"]["code"], "match_over");
    assert_eq!(view(&a, 93010)["snapshot"]["own"]["score"], 10);
}

#[test]
fn explicit_leave_preserves_old_work_until_a_new_session_is_successful() {
    let (mut service, mut a, _) = started(Mode::Race, 6);
    let initial = snapshot(&a, 3000);
    let index = initial.own.givens.iter().position(|&v| v == 0).unwrap();
    set(&mut a, index, 2, 4000);
    let local = view(&a, 4000)["race"].clone();
    drive(&mut service, &mut a, json!({"type":"leave"}), 4000);
    assert_eq!(view(&a, 4000)["race"], local);
    assert_eq!(view(&a, 4000)["voluntarily_left"], true);
    let wire = req(
        &mut a,
        json!({"type":"create","name":"A","mode":"duel","size":9}),
        5000,
    );
    assert_eq!(view(&a, 5000)["race"], local);
    let response = exchange(&mut service, &wire, 5000);
    receive(&mut a, &wire, response, 5000);
    assert_eq!(view(&a, 5000)["snapshot"]["mode"], "duel");
    assert!(view(&a, 5000)["race"].is_null());
}

#[test]
fn rematch_is_new_identity_and_delayed_old_response_cannot_undo_it() {
    let (mut service, mut a, mut b) = started(Mode::Duel, 6);
    drive(
        &mut service,
        &mut a,
        json!({"type":"command","command":{"type":"resign"}}),
        4000,
    );
    drive(&mut service, &mut b, json!({"type":"poll"}), 4000);
    let old_match = snapshot(&a, 4000).match_id;
    let request_a = req(
        &mut a,
        json!({"type":"command","command":{"type":"rematch"}}),
        4000,
    );
    let old_response = exchange(&mut service, &request_a, 4000);
    receive(&mut a, &request_a, old_response.clone(), 4000);
    drive(
        &mut service,
        &mut b,
        json!({"type":"command","command":{"type":"rematch"}}),
        4000,
    );
    drive(&mut service, &mut a, json!({"type":"poll"}), 4000);
    let current = snapshot(&a, 4000);
    assert_ne!(current.match_id, old_match);
    assert_eq!(current.own.next_sequence, 1);
    assert_eq!(
        receive(&mut a, &request_a, old_response, 4000)["changed"],
        false
    );
    assert_eq!(snapshot(&a, 4000).match_id, current.match_id);
}

#[test]
fn a_remote_change_never_discards_manual_notes_or_local_undo_history() {
    let (mut service, mut a, _) = started(Mode::Race, 6);
    let initial = snapshot(&a, 3000);
    let blanks: Vec<usize> = initial
        .own
        .givens
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (v == 0).then_some(i))
        .collect();
    act(
        &mut a,
        json!({"type":"edit","command":{"type":"toggle_note","index":blanks[0],"value":2}}),
        4000,
    );
    let saved: Value = serde_json::from_str(&a.save().unwrap()).unwrap();
    let mut remote = initial.own.cells.clone();
    remote[blanks[1]] = 3;
    let envelope = Envelope {
        protocol: 1,
        token: saved["auth"]["token"].as_str().unwrap().into(),
        generation: initial.generation,
        match_id: initial.match_id,
        sequence: initial.own.next_sequence,
        command: Command::Sync {
            base_version: 0,
            cells: remote,
        },
    };
    service
        .command(&initial.room_code, &envelope, 4000)
        .unwrap();
    drive(&mut service, &mut a, json!({"type":"poll"}), 4000);
    let state = view(&a, 4000);
    assert_eq!(state["reconcile_required"], true);
    assert_eq!(state["race"]["notes"][blanks[0]], json!([2]));
    assert_eq!(state["race"]["can_undo"], true);
}

#[test]
fn saved_uncertain_race_resumes_in_a_new_monotonic_epoch_without_losing_notes() {
    let (mut service, mut a, _) = started(Mode::Race, 6);
    let initial = snapshot(&a, 3000);
    let blanks: Vec<usize> = initial
        .own
        .givens
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (v == 0).then_some(i))
        .collect();
    set(&mut a, blanks[0], 3, 4000);
    act(
        &mut a,
        json!({"type":"edit","command":{"type":"toggle_note","index":blanks[1],"value":2}}),
        4000,
    );
    let unprocessed = req(&mut a, json!({"type":"sync"}), 4000);
    let saved = a.save().unwrap();
    let mut loaded = MultiplayerClient::load(&saved).unwrap();
    assert!(view(&loaded, 0)["estimated_server_ms"].is_null());
    let resume = req(&mut loaded, json!({"type":"resume"}), 0);
    let response = exchange(&mut service, &resume, 5000);
    receive(&mut loaded, &resume, response, 0);
    assert_eq!(view(&loaded, 0)["snapshot"]["generation"], 2);
    assert_eq!(view(&loaded, 0)["race"]["cells"][blanks[0]], 3);
    assert_eq!(view(&loaded, 0)["race"]["notes"][blanks[1]], json!([2]));
    assert_eq!(view(&loaded, 0)["race"]["can_undo"], true);
    let sync = req(&mut loaded, json!({"type":"sync"}), 1);
    let response = exchange(&mut service, &sync, 5001);
    receive(&mut loaded, &sync, response, 1);
    assert_eq!(view(&loaded, 1)["race"]["dirty"], false);
    assert_eq!(view(&loaded, 1)["race"]["notes"][blanks[1]], json!([2]));
    let stale = exchange(&mut service, &unprocessed, 5002);
    assert_eq!(stale.0, 409);
    assert_eq!(
        receive(&mut loaded, &unprocessed, stale, 2)["changed"],
        false
    );
}

// Review regressions: refused/lost room starts, finished-match drafts, offline signal.
fn try_act(client: &mut MultiplayerClient, action: Value, now: u64) -> Result<String, ClientError> {
    client.dispatch_json(&action.to_string(), now)
}
const GUEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

#[test]
fn refused_join_reports_error_without_wedging_client() {
    let (mut service, a, _) = started(Mode::Race, 6);
    let code = snapshot(&a, 3000).room_code;
    let mut c = MultiplayerClient::new(9);
    let wire = req(
        &mut c,
        json!({"type":"join","invite":code,"name":"C","guest":GUEST_C}),
        3000,
    );
    let response = exchange(&mut service, &wire, 3000);
    receive(&mut c, &wire, response, 3000);
    let v = view(&c, 3000);
    assert_eq!(v["last_error"], "room_full");
    assert_eq!(v["pending_http"], false);
    assert_eq!(v["retry_at_ms"], Value::Null);
    assert!(matches!(
        try_act(&mut c, json!({"type":"retry"}), 60_000),
        Err(ClientError::InvalidCommand)
    ));
    let fresh = req(
        &mut c,
        json!({"type":"create","name":"C","mode":"race","size":6}),
        60_000,
    );
    assert_eq!(exchange(&mut service, &fresh, 60_000).0, 200);
}

#[test]
fn wrong_code_keeps_finished_room_for_rematch() {
    let (mut service, mut a, mut b) = started(Mode::Duel, 6);
    drive(
        &mut service,
        &mut a,
        json!({"type":"command","command":{"type":"resign"}}),
        4000,
    );
    drive(&mut service, &mut b, json!({"type":"poll"}), 4000);
    let wire = req(
        &mut a,
        json!({"type":"join","invite":"ZZZZZZ","name":"A"}),
        4100,
    );
    let response = exchange(&mut service, &wire, 4100);
    receive(&mut a, &wire, response, 4100);
    assert_eq!(view(&a, 4100)["last_error"], "room_missing");
    let saved: Value = serde_json::from_str(&a.save().unwrap()).unwrap();
    assert_eq!(saved["session_lost"], false);
    drive(
        &mut service,
        &mut a,
        json!({"type":"command","command":{"type":"rematch"}}),
        5000,
    );
    assert_eq!(view(&a, 5000)["last_error"], Value::Null);
}

#[test]
fn lost_create_response_replays_same_room() {
    let mut service = Service::default();
    let mut a = MultiplayerClient::new(1);
    let wire = req(
        &mut a,
        json!({"type":"create","name":"A","mode":"duel","size":6,"guest":GUEST_C}),
        0,
    );
    let lost = exchange(&mut service, &wire, 0);
    assert_eq!(lost.0, 200);
    act(
        &mut a,
        json!({"type":"network_failed","request_id":wire.id}),
        0,
    );
    let at = view(&a, 0)["retry_at_ms"].as_u64().unwrap();
    let retry = req(&mut a, json!({"type":"retry"}), at);
    assert_eq!(retry, wire);
    let replay = exchange(&mut service, &retry, at);
    assert_eq!(replay.0, 200);
    let first: Value = serde_json::from_str(&lost.1).unwrap();
    let second: Value = serde_json::from_str(&replay.1).unwrap();
    assert_eq!(first["token"], second["token"]);
    assert_eq!(
        first["snapshot"]["room_code"],
        second["snapshot"]["room_code"]
    );
    receive(&mut a, &retry, replay, at);
    assert_eq!(view(&a, at)["connection"], "connected");
    // An abandoned create can be replaced; the same guest keeps it idempotent.
    let mut b = MultiplayerClient::new(2);
    let w = req(
        &mut b,
        json!({"type":"create","name":"B","mode":"race","size":9,"guest":"dd".repeat(32)}),
        0,
    );
    act(
        &mut b,
        json!({"type":"network_failed","request_id":w.id}),
        0,
    );
    let again = req(
        &mut b,
        json!({"type":"create","name":"B","mode":"race","size":9,"guest":"ee".repeat(32)}),
        10,
    );
    assert!(again.body.contains(&"dd".repeat(32)));
}

#[test]
fn lost_join_response_replays_seat_without_phantom() {
    let mut service = Service::default();
    let mut a = MultiplayerClient::new(1);
    drive(
        &mut service,
        &mut a,
        json!({"type":"create","name":"A","mode":"race","size":6}),
        0,
    );
    let code = snapshot(&a, 0).room_code;
    let mut b = MultiplayerClient::new(2);
    let wire = req(
        &mut b,
        json!({"type":"join","invite":code,"name":"B","guest":GUEST_C}),
        1000,
    );
    assert_eq!(exchange(&mut service, &wire, 1000).0, 200);
    act(
        &mut b,
        json!({"type":"network_failed","request_id":wire.id}),
        1000,
    );
    let at = view(&b, 1000)["retry_at_ms"].as_u64().unwrap();
    let retry = req(&mut b, json!({"type":"retry"}), at);
    let response = exchange(&mut service, &retry, at);
    assert_eq!(response.0, 200);
    receive(&mut b, &retry, response, at);
    assert_eq!(view(&b, at)["connection"], "connected");
    drive(
        &mut service,
        &mut b,
        json!({"type":"command","command":{"type":"ready","ready":true}}),
        at,
    );
    drive(
        &mut service,
        &mut a,
        json!({"type":"command","command":{"type":"ready","ready":true}}),
        at,
    );
    assert_eq!(snapshot(&a, at).phase, Phase::Countdown);
}

#[test]
fn unsent_board_from_finished_race_does_not_block_rematch_or_new_room() {
    let (mut service, mut a, mut b) = started(Mode::Race, 6);
    let initial = snapshot(&a, 3000);
    let solved = sudoku_core::solve_board(&initial.own.givens, 6)
        .unwrap()
        .unwrap();
    let blank = initial.own.givens.iter().position(|&v| v == 0).unwrap();
    set(&mut b, blank, solved[blank], 3500);
    for (i, (&g, &v)) in initial.own.givens.iter().zip(&solved).enumerate() {
        if g == 0 {
            set(&mut a, i, v, 3600);
        }
    }
    drive(&mut service, &mut a, json!({"type":"sync"}), 3600);
    drive(&mut service, &mut b, json!({"type":"poll"}), 3700);
    assert_eq!(snapshot(&b, 3700).phase, Phase::Finished);
    assert!(try_act(
        &mut b.clone(),
        json!({"type":"create","name":"B","mode":"race","size":6}),
        3800
    )
    .is_ok());
    drive(
        &mut service,
        &mut a,
        json!({"type":"command","command":{"type":"rematch"}}),
        3800,
    );
    drive(
        &mut service,
        &mut b,
        json!({"type":"command","command":{"type":"rematch"}}),
        3900,
    );
    drive(&mut service, &mut a, json!({"type":"poll"}), 7000);
    drive(&mut service, &mut b, json!({"type":"poll"}), 7000);
    let v = view(&b, 7000);
    let s = snapshot(&b, 7000);
    assert_eq!(s.phase, Phase::Playing);
    assert_eq!(v["reconcile_required"], false);
    assert_eq!(v["race"]["cells"], json!(s.own.givens));
}

#[test]
fn generic_offline_signal_keeps_lost_session() {
    let (mut service, mut a, _) = started(Mode::Race, 6);
    drive(&mut service, &mut a, json!({"type":"leave"}), 4000);
    act(&mut a, json!({"type":"network_failed"}), 4001);
    assert_eq!(view(&a, 4001)["connection"], "session_lost");
    let saved: Value = serde_json::from_str(&a.save().unwrap()).unwrap();
    assert_eq!(saved["session_lost"], true);
}

#[test]
fn player_command_supersedes_in_flight_poll() {
    let (mut service, mut a, mut b) = started(Mode::Duel, 6);
    drive(
        &mut service,
        &mut a,
        json!({"type":"command","command":{"type":"resign"}}),
        4000,
    );
    drive(&mut service, &mut b, json!({"type":"poll"}), 4000);
    let poll = req(&mut a, json!({"type":"poll"}), 5000);
    let rematch = req(
        &mut a,
        json!({"type":"command","command":{"type":"rematch"}}),
        5000,
    );
    assert_ne!(poll.id, rematch.id);
    let late = exchange(&mut service, &poll, 5000);
    assert_eq!(receive(&mut a, &poll, late, 5000)["changed"], false);
    let response = exchange(&mut service, &rematch, 5000);
    receive(&mut a, &rematch, response, 5000);
    drive(
        &mut service,
        &mut b,
        json!({"type":"command","command":{"type":"rematch"}}),
        5100,
    );
    assert_eq!(snapshot(&b, 5100).phase, Phase::Countdown);
    // A command never replaces another command or a session request.
    assert!(matches!(
        try_act(
            &mut b,
            json!({"type":"command","command":{"type":"ready","ready":false}}),
            5100
        )
        .and_then(|_| try_act(
            &mut b,
            json!({"type":"command","command":{"type":"ready","ready":true}}),
            5100
        )),
        Err(ClientError::Busy)
    ));
}

#[test]
fn cold_restore_replays_lost_create_and_join() {
    let mut service = Service::default();
    let mut a = MultiplayerClient::new(1);
    let create = req(
        &mut a,
        json!({"type":"create","name":"A","mode":"race","size":6,"guest":GUEST_C}),
        0,
    );
    let lost = exchange(&mut service, &create, 0);
    assert_eq!(lost.0, 200);
    // The process dies before the response arrives; only the opaque save survives.
    let mut a = MultiplayerClient::load(&a.save().unwrap()).unwrap();
    assert_eq!(view(&a, 0)["retry_at_ms"], 0);
    let retry = req(&mut a, json!({"type":"retry"}), 0);
    assert_eq!(retry, create, "Cold replay must reuse the exact body");
    let replay = exchange(&mut service, &retry, 0);
    receive(&mut a, &retry, replay, 0);
    let code = snapshot(&a, 0).room_code;
    let first: Value = serde_json::from_str(&lost.1).unwrap();
    assert_eq!(first["snapshot"]["room_code"], code.as_str());
    assert!(!a.save().unwrap().contains("\"setup\""));

    let mut b = MultiplayerClient::new(2);
    let join = req(
        &mut b,
        json!({"type":"join","invite":code,"name":"B","guest":"dd".repeat(32)}),
        10,
    );
    assert_eq!(exchange(&mut service, &join, 10).0, 200);
    let mut b = MultiplayerClient::load(&b.save().unwrap()).unwrap();
    let retry = req(&mut b, json!({"type":"retry"}), 0);
    assert_eq!(retry, join);
    let replay = exchange(&mut service, &retry, 20);
    receive(&mut b, &retry, replay, 20);
    assert_eq!(snapshot(&b, 20).room_code, code);
    drive(&mut service, &mut a, json!({"type":"poll"}), 30);
    assert_eq!(
        snapshot(&a, 30).opponent.unwrap().name,
        "B",
        "No phantom second seat"
    );

    // A tampered setup is never sent to an arbitrary path.
    let mut c = MultiplayerClient::new(3);
    req(
        &mut c,
        json!({"type":"create","name":"C","mode":"duel","size":9,"guest":"ee".repeat(32)}),
        0,
    );
    let forged = c
        .save()
        .unwrap()
        .replace("/v1/rooms", "/v1/rooms/ABCDEF/leave");
    assert!(MultiplayerClient::load(&forged).is_err());
}
