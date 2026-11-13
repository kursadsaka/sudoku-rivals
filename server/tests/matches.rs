use sudoku_server::{
    Code, Command, EndReason, Envelope, Error, Mode, Phase, Service, Session, Snapshot, PROTOCOL,
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
    fn envelope(&self, command: Command) -> Envelope {
        Envelope {
            protocol: PROTOCOL,
            token: self.token.clone(),
            generation: self.snapshot.generation,
            match_id: self.snapshot.match_id.clone(),
            sequence: self.snapshot.own.next_sequence,
            command,
        }
    }
    fn send(
        &mut self,
        service: &mut Service,
        command: Command,
        now: u64,
    ) -> sudoku_server::Receipt {
        let result = service
            .command(&self.snapshot.room_code, &self.envelope(command), now)
            .unwrap();
        self.snapshot = result.snapshot;
        result.receipt
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
}
fn started(mode: Mode, size: u8) -> (Service, Bot, Bot) {
    let mut service = Service::default();
    let mut a: Bot = service
        .create("a", None, "A".into(), mode, size, 0)
        .unwrap()
        .into();
    let mut b: Bot = service
        .join("b", &a.snapshot.room_code, None, "B".into(), 0)
        .unwrap()
        .into();
    assert_eq!(
        a.send(&mut service, Command::Ready { ready: true }, 0).code,
        Code::Ok
    );
    assert_eq!(
        b.send(&mut service, Command::Ready { ready: true }, 0).code,
        Code::Ok
    );
    a.state(&mut service, 3000);
    b.state(&mut service, 3000);
    assert_eq!(a.snapshot.phase, Phase::Playing);
    assert_eq!(a.snapshot.own.givens, b.snapshot.own.givens);
    (service, a, b)
}
fn solution(bot: &Bot) -> Vec<u8> {
    sudoku_core::solve_board(&bot.snapshot.own.givens, bot.snapshot.size)
        .unwrap()
        .unwrap()
}
fn next_move(bot: &Bot) -> Command {
    let index = bot.snapshot.own.cells.iter().position(|&v| v == 0).unwrap();
    Command::Move {
        board_id: bot.snapshot.own.board_id,
        index: index as u32,
        value: solution(bot)[index],
    }
}

#[test]
fn bots_complete_both_modes_in_both_sizes() {
    for size in [6, 9] {
        for mode in [Mode::Race, Mode::Duel] {
            let (mut service, mut a, mut b) = started(mode, size);
            if mode == Mode::Race {
                let cells = solution(&a);
                assert_eq!(
                    a.send(
                        &mut service,
                        Command::Sync {
                            base_version: 0,
                            cells
                        },
                        3000
                    )
                    .code,
                    Code::Ok
                );
            } else {
                let blanks = a.snapshot.own.cells.iter().filter(|&&v| v == 0).count();
                for i in 0..blanks {
                    let cmd = next_move(&a);
                    assert_eq!(
                        a.send(&mut service, cmd, 3000 + i as u64 * 200).code,
                        Code::Ok
                    );
                }
                assert_eq!(a.snapshot.own.board_id, 1);
                assert_eq!(a.snapshot.own.score, if size == 6 { 410 } else { 1035 });
                a.state(&mut service, 93_000);
            }
            assert_eq!(a.snapshot.phase, Phase::Finished);
            assert_eq!(a.snapshot.result.as_ref().unwrap().winner, Some(0));
            b.state(&mut service, a.snapshot.server_ms);
            assert_eq!(a.snapshot.result, b.snapshot.result);
            if mode == Mode::Race {
                assert_eq!(
                    b.snapshot.opponent.as_ref().unwrap().progress_percent,
                    Some(100)
                );
            }
        }
    }
}

#[test]
fn race_has_no_tie_window_or_wrong_board_lock() {
    for size in [6, 9] {
        let (mut service, mut a, mut b) = started(Mode::Race, size);
        let correct_a = solution(&a);
        let correct_b = solution(&b);
        let mut wrong = correct_a.clone();
        let index = a.snapshot.own.givens.iter().position(|&v| v == 0).unwrap();
        wrong[index] = wrong[index] % size + 1;
        assert_eq!(
            a.send(
                &mut service,
                Command::Sync {
                    base_version: 0,
                    cells: wrong
                },
                4000
            )
            .code,
            Code::NotCorrect
        );
        assert_eq!(
            a.send(
                &mut service,
                Command::Sync {
                    base_version: 1,
                    cells: correct_a
                },
                8000
            )
            .code,
            Code::Ok
        );
        assert_eq!(
            b.send(
                &mut service,
                Command::Sync {
                    base_version: 0,
                    cells: correct_b
                },
                8080
            )
            .code,
            Code::MatchOver
        );
        assert_eq!(a.snapshot.result.unwrap().winner, Some(0));
    }
}

#[test]
fn duel_deadline_uses_explicit_admission_times() {
    for size in [6, 9] {
        let (mut service, mut a, _) = started(Mode::Duel, size);
        // State polls keep the seat connected. Admission happened at 92.999;
        // no later tick is allowed to overtake this event, regardless of CPU delay.
        for t in (13_000..93_000).step_by(10_000) {
            a.state(&mut service, t);
        }
        let cmd = next_move(&a);
        let receipt = a.send(&mut service, cmd, 92_999);
        assert_eq!(receipt.score, 10);
        let cmd = next_move(&a);
        assert_eq!(a.send(&mut service, cmd, 93_000).code, Code::MatchOver);
        assert_eq!(a.snapshot.own.score, 10);
        assert_eq!(a.snapshot.result.as_ref().unwrap().at_ms, 93_000);
    }
}

#[test]
fn retries_are_frozen_and_do_not_repeat_penalties_or_quota() {
    for size in [6, 9] {
        let (mut service, mut a, _) = started(Mode::Duel, size);
        let correct = next_move(&a);
        let Command::Move {
            board_id,
            index,
            value,
        } = correct
        else {
            unreachable!()
        };
        let command = Command::Move {
            board_id,
            index,
            value: value % size + 1,
        };
        let envelope = a.envelope(command.clone());
        let first = service
            .command(&a.snapshot.room_code, &envelope, 4000)
            .unwrap();
        assert_eq!(first.receipt.code, Code::Wrong);
        for _ in 0..50 {
            let again = service
                .command(&a.snapshot.room_code, &envelope, 4000)
                .unwrap();
            assert_eq!(first.receipt, again.receipt);
        }
        a.snapshot = first.snapshot;
        assert_eq!(a.snapshot.own.wrong, 1);
        assert_eq!(a.snapshot.own.score, 0);
        let mut changed = envelope.clone();
        changed.command = Command::Resign;
        assert_eq!(
            service.command(&a.snapshot.room_code, &changed, 4000).err(),
            Some(Error::SequenceConflict)
        );
        let mut gap = a.envelope(Command::Resign);
        gap.sequence += 1;
        assert_eq!(
            service.command(&a.snapshot.room_code, &gap, 4000).err(),
            Some(Error::SequenceGap)
        );
        assert_eq!(
            a.send(
                &mut service,
                Command::Move {
                    board_id,
                    index,
                    value
                },
                5000
            )
            .code,
            Code::Locked
        );
        assert_eq!(
            a.send(
                &mut service,
                Command::Move {
                    board_id,
                    index,
                    value
                },
                6000
            )
            .code,
            Code::Ok
        );
        assert_eq!(a.snapshot.own.score, 10);
    }
}

#[test]
fn stale_generation_rejected_and_offline_edits_sync_once() {
    for size in [6, 9] {
        let (mut service, mut a, mut b) = started(Mode::Race, size);
        let stale = a.envelope(Command::Resign);
        a.send(&mut service, Command::Disconnect, 4000);
        b.state(&mut service, 5000);
        a.snapshot = service
            .resume(&a.snapshot.room_code, &a.token, 5000)
            .unwrap();
        assert_eq!(
            service.command(&a.snapshot.room_code, &stale, 5000).err(),
            Some(Error::StaleGeneration)
        );
        let mut local = a.snapshot.own.cells.clone();
        let index = local.iter().position(|&v| v == 0).unwrap();
        for n in 0..40 {
            local[index] = (n % size as usize + 1) as u8;
        }
        let before = a.snapshot.own.next_sequence;
        assert_eq!(
            a.send(
                &mut service,
                Command::Sync {
                    base_version: 0,
                    cells: local.clone()
                },
                5000
            )
            .code,
            Code::Ok
        );
        assert_eq!(a.snapshot.own.cells, local);
        assert_eq!(a.snapshot.own.board_version, 1);
        assert_eq!(a.snapshot.own.next_sequence, before + 1);
        let snapshot = a.snapshot.own.cells.clone();
        assert_eq!(
            a.send(
                &mut service,
                Command::Sync {
                    base_version: 0,
                    cells: solution(&b)
                },
                5000
            )
            .code,
            Code::StaleVersion
        );
        assert_eq!(a.snapshot.own.cells, snapshot);
    }
}

#[test]
fn disconnect_deadline_boundary_and_simultaneous_expirations() {
    for size in [6, 9] {
        let (mut service, mut a, mut b) = started(Mode::Race, size);
        a.send(&mut service, Command::Disconnect, 4000);
        b.send(&mut service, Command::Disconnect, 4000);
        a.snapshot = service
            .resume(&a.snapshot.room_code, &a.token, 124_000)
            .unwrap();
        assert_eq!(a.snapshot.result.as_ref().unwrap().winner, None);
        assert_eq!(
            a.snapshot.result.as_ref().unwrap().reason,
            EndReason::BothDisconnected
        );
        let (mut service, mut a, mut b) = started(Mode::Race, size);
        a.send(&mut service, Command::Disconnect, 4000);
        for t in (14_000..124_000).step_by(10_000) {
            b.state(&mut service, t);
        }
        a.snapshot = service
            .resume(&a.snapshot.room_code, &a.token, 124_000)
            .unwrap();
        assert_eq!(a.snapshot.result.as_ref().unwrap().winner, Some(1));
        assert_eq!(a.snapshot.result.as_ref().unwrap().at_ms, 124_000);
    }
}

#[test]
fn spam_is_limited_before_correctness_and_replay_is_free() {
    let (mut service, mut a, _) = started(Mode::Duel, 6);
    let invalid = Command::Move {
        board_id: 0,
        index: u32::MAX,
        value: 1,
    };
    for _ in 0..6 {
        assert_eq!(
            a.send(&mut service, invalid.clone(), 4000).code,
            Code::InvalidCell
        );
    }
    let correct = next_move(&a);
    assert_eq!(a.send(&mut service, correct, 4000).code, Code::RateLimited);
    assert_eq!(a.snapshot.own.score, 0);
    for _ in 0..29 {
        assert_eq!(
            a.send(&mut service, invalid.clone(), 4000).code,
            Code::RateLimited
        );
    }
    assert!(!a.snapshot.own.connected);
    assert_eq!(a.send(&mut service, invalid, 4000).code, Code::Disconnected);
}

#[test]
fn completion_bonus_once_and_rematch_is_new_identity() {
    for mode in [Mode::Race, Mode::Duel] {
        let (mut service, mut a, mut b) = started(mode, 6);
        if mode == Mode::Duel {
            let blanks = a.snapshot.own.cells.iter().filter(|&&v| v == 0).count();
            let mut final_envelope = a.envelope(next_move(&a));
            for i in 0..blanks {
                let cmd = next_move(&a);
                final_envelope = a.envelope(cmd.clone());
                a.send(&mut service, cmd, 4000 + i as u64 * 200);
            }
            let replay = service
                .command(&a.snapshot.room_code, &final_envelope, 8000)
                .unwrap();
            assert_eq!(replay.receipt.score, 410);
            assert_eq!(replay.snapshot.own.board_id, 1);
            assert_eq!(
                a.send(
                    &mut service,
                    Command::Move {
                        board_id: 0,
                        index: 0,
                        value: 1
                    },
                    8000
                )
                .code,
                Code::StaleBoard
            );
        }
        a.send(&mut service, Command::Resign, 9000);
        b.state(&mut service, 9000);
        let old_match = a.snapshot.match_id.clone();
        a.send(&mut service, Command::Rematch, 9000);
        let second_request = b.envelope(Command::Rematch);
        let second = service
            .command(&b.snapshot.room_code, &second_request, 9000)
            .unwrap();
        b.snapshot = second.snapshot;
        assert_ne!(b.snapshot.match_id, old_match);
        assert_eq!(b.snapshot.phase, Phase::Countdown);
        assert_eq!(b.snapshot.own.next_sequence, 1);
        let repeated = service
            .command(&b.snapshot.room_code, &second_request, 9000)
            .unwrap();
        assert_eq!(repeated.receipt, second.receipt);
        assert_eq!(repeated.snapshot.match_id, b.snapshot.match_id);
    }
}

#[test]
fn serialized_snapshot_excludes_private_opponent_state_and_solutions() {
    for mode in [Mode::Race, Mode::Duel] {
        let (mut service, mut a, b) = started(mode, 9);
        a.state(&mut service, 3000);
        let json = serde_json::to_value(&a.snapshot).unwrap();
        let object = json["opponent"].as_object().unwrap();
        assert_eq!(object.len(), 6);
        for forbidden in [
            "cells", "givens", "solution", "token", "guest", "combo", "wrong", "board_id", "seed",
        ] {
            assert!(!object.contains_key(forbidden));
            assert!(!json.as_object().unwrap().contains_key(forbidden));
        }
        assert!(!serde_json::to_string(&a.snapshot)
            .unwrap()
            .contains(&b.token));
        assert!(service
            .join("third", &a.snapshot.room_code, None, "C".into(), 3000)
            .is_err());
    }
}

#[test]
fn restart_has_no_result_and_bad_auth_cannot_take_a_seat() {
    let (mut service, a, _) = started(Mode::Race, 6);
    assert_eq!(
        service.resume(&a.snapshot.room_code, "wrong", 3000).err(),
        Some(Error::Unauthorized)
    );
    let mut restarted = Service::default();
    assert_eq!(
        restarted
            .resume(&a.snapshot.room_code, &a.token, 3000)
            .err(),
        Some(Error::RoomMissing)
    );
    assert_eq!(
        restarted
            .state(&a.snapshot.room_code, &a.token, 1, 3000)
            .err(),
        Some(Error::RoomMissing)
    );
}

#[test]
fn six_scoring_examples_and_wrong_count_tiebreak_are_exact() {
    for size in [6, 9] {
        for (pattern, score, wrong) in [
            ("DDDDD", 60, 0),
            ("DDDDDDDDDDDD", 210, 0),
            ("DDDDYDD", 60, 1),
            ("YDD", 20, 1),
            ("DDYYD", 20, 2),
            ("DYYY", 0, 3),
        ] {
            let (mut service, mut a, _) = started(Mode::Duel, size);
            let mut now = 4000;
            for action in pattern.chars() {
                let mut cmd = next_move(&a);
                if action == 'Y' {
                    if let Command::Move { value, .. } = &mut cmd {
                        *value = *value % size + 1;
                    }
                }
                let code = a.send(&mut service, cmd, now).code;
                assert_eq!(code, if action == 'D' { Code::Ok } else { Code::Wrong });
                now += if action == 'Y' { 2000 } else { 200 };
            }
            assert_eq!(
                (a.snapshot.own.score, a.snapshot.own.wrong),
                (score, wrong),
                "{size} {pattern}"
            );
        }
        let (mut service, mut a, mut b) = started(Mode::Duel, size);
        let mut wrong = next_move(&a);
        if let Command::Move { value, .. } = &mut wrong {
            *value = *value % size + 1;
        }
        a.send(&mut service, wrong.clone(), 4000);
        b.state(&mut service, 93_000);
        assert_eq!(b.snapshot.result.as_ref().unwrap().winner, Some(1));
        let (mut service, mut a, _) = started(Mode::Duel, size);
        a.state(&mut service, 93_000);
        assert_eq!(a.snapshot.result.as_ref().unwrap().winner, None);
    }
}

#[test]
fn invalid_inputs_givens_and_noop_versions_never_change_the_board() {
    for size in [6, 9] {
        let (mut service, mut a, _) = started(Mode::Race, size);
        let original = a.snapshot.own.cells.clone();
        assert_eq!(
            a.send(
                &mut service,
                Command::Sync {
                    base_version: 0,
                    cells: original.clone()
                },
                4000
            )
            .code,
            Code::NoChange
        );
        assert_eq!(a.snapshot.own.board_version, 0);
        let mut given_changed = original.clone();
        let given = given_changed.iter().position(|&v| v != 0).unwrap();
        given_changed[given] = 0;
        for bad in [vec![], vec![0; size as usize * size as usize + 1]] {
            assert!(matches!(
                service.command(
                    &a.snapshot.room_code,
                    &a.envelope(Command::Sync {
                        base_version: 0,
                        cells: bad
                    }),
                    4000
                ),
                Err(Error::InvalidInput)
            ));
        }
        for bad in [given_changed] {
            assert_eq!(
                a.send(
                    &mut service,
                    Command::Sync {
                        base_version: 0,
                        cells: bad
                    },
                    4000
                )
                .code,
                Code::InvalidBoard
            );
            assert_eq!(a.snapshot.own.cells, original);
            assert_eq!(a.snapshot.own.board_version, 0);
        }
        let (mut service, mut a, _) = started(Mode::Duel, size);
        let given = a.snapshot.own.givens.iter().position(|&v| v != 0).unwrap() as u32;
        assert_eq!(
            a.send(
                &mut service,
                Command::Move {
                    board_id: 0,
                    index: given,
                    value: 1
                },
                4000
            )
            .code,
            Code::GivenCell
        );
        assert_eq!(a.snapshot.own.wrong, 0);
        assert_eq!(a.snapshot.own.score, 0);
    }
}

#[test]
fn lobby_restart_uses_new_match_identity_and_preserves_old_receipts() {
    let (mut service, mut a, mut b) = started(Mode::Race, 6);
    let old_resign = a.envelope(Command::Resign);
    let result = service
        .command(&a.snapshot.room_code, &old_resign, 4000)
        .unwrap();
    a.snapshot = result.snapshot;
    b.state(&mut service, 4000);
    a.send(&mut service, Command::Lobby, 4000);
    assert_eq!(a.snapshot.phase, Phase::Lobby);
    assert!(a.snapshot.result.is_none());
    a.send(&mut service, Command::Ready { ready: true }, 4000);
    b.send(&mut service, Command::Ready { ready: true }, 4000);
    assert_ne!(b.snapshot.match_id, old_resign.match_id);
    let replay = service
        .command(&a.snapshot.room_code, &old_resign, 4000)
        .unwrap();
    assert_eq!(replay.receipt, result.receipt);
    assert_eq!(replay.snapshot.phase, Phase::Countdown);
}

#[test]
fn explicit_lobby_exit_releases_guest_and_seat_and_cancels_countdown() {
    for mode in [Mode::Race, Mode::Duel] {
        let (mut service, a, mut b) = started(mode, 6);
        // In a playing match explicit leave is a forfeit, then revokes the token.
        service
            .leave(&a.snapshot.room_code, &a.token, 3100)
            .unwrap();
        service
            .leave(&a.snapshot.room_code, &a.token, 3200)
            .unwrap();
        b.state(&mut service, 3200);
        assert_eq!(b.snapshot.result.as_ref().unwrap().winner, Some(1));
        assert_eq!(
            b.snapshot.result.as_ref().unwrap().reason,
            EndReason::Resigned
        );
        assert_eq!(b.snapshot.result.as_ref().unwrap().at_ms, 3100);
        assert!(b.snapshot.opponent.is_none());
        assert!(matches!(
            service.state(&a.snapshot.room_code, &a.token, 1, 3200),
            Err(Error::Unauthorized)
        ));
        let next = service
            .create("new-a", None, "Next".into(), mode, 6, 3200)
            .unwrap();
        service
            .leave(&next.snapshot.room_code, &next.token, 3200)
            .unwrap();
    }
    let mut service = Service::default();
    let a = service
        .create("a", None, "A".into(), Mode::Race, 6, 0)
        .unwrap();
    let code = a.snapshot.room_code.clone();
    let guest = a.guest.clone();
    let mut a: Bot = a.into();
    let mut b: Bot = service
        .join("b", &code, None, "B".into(), 0)
        .unwrap()
        .into();
    a.send(&mut service, Command::Ready { ready: true }, 0);
    b.send(&mut service, Command::Ready { ready: true }, 0);
    assert_eq!(b.snapshot.phase, Phase::Countdown);
    service.leave(&code, &a.token, 100).unwrap();
    b.state(&mut service, 100);
    assert_eq!(b.snapshot.phase, Phase::Lobby);
    assert!(b.snapshot.opponent.is_none());
    assert!(!b.snapshot.own.ready);
    assert!(service
        .create("a", Some(guest), "A".into(), Mode::Race, 6, 100)
        .is_ok());
    assert!(service.join("c", &code, None, "C".into(), 100).is_ok());
}

#[test]
fn cancelled_countdowns_never_reveal_the_puzzle() {
    for mode in [Mode::Race, Mode::Duel] {
        for size in [6, 9] {
            let mut service = Service::default();
            let mut a: Bot = service
                .create("a", None, "A".into(), mode, size, 0)
                .unwrap()
                .into();
            let mut b: Bot = service
                .join("b", &a.snapshot.room_code, None, "B".into(), 0)
                .unwrap()
                .into();
            a.send(&mut service, Command::Ready { ready: true }, 0);
            b.send(&mut service, Command::Ready { ready: true }, 0);
            assert_eq!(b.snapshot.phase, Phase::Countdown);
            assert!(b.snapshot.own.givens.is_empty());
            assert!(b.snapshot.own.cells.is_empty());
            a.state(&mut service, 0);
            assert!(a.snapshot.own.givens.is_empty());
            a.send(&mut service, Command::Ready { ready: false }, 100);
            a.send(&mut service, Command::Ready { ready: true }, 1000);
            assert!(a.snapshot.own.givens.is_empty());
            a.send(&mut service, Command::Disconnect, 1001);
            a.snapshot = service
                .resume(&a.snapshot.room_code, &a.token, 1002)
                .unwrap();
            a.send(&mut service, Command::Ready { ready: true }, 1003);
            assert!(a.snapshot.own.givens.is_empty());
            a.state(&mut service, 4003);
            assert_eq!(a.snapshot.own.cells.len(), (size as usize).pow(2));
        }
    }
}

#[test]
fn one_seat_cannot_spend_the_other_seats_receipt_budget_or_erase_a_win() {
    let (mut service, mut a, mut b) = started(Mode::Race, 6);
    for _ in 1..8192 {
        assert_eq!(
            b.send(&mut service, Command::Ready { ready: true }, 4000)
                .code,
            Code::InvalidState
        );
    }
    assert!(matches!(
        service.command(
            &b.snapshot.room_code,
            &b.envelope(Command::Ready { ready: true }),
            4000
        ),
        Err(Error::Capacity)
    ));
    a.state(&mut service, 4000);
    assert_eq!(a.snapshot.phase, Phase::Playing);
    let cells = solution(&a);
    a.send(
        &mut service,
        Command::Sync {
            base_version: 0,
            cells,
        },
        4000,
    );
    assert_eq!(a.snapshot.result.as_ref().unwrap().winner, Some(0));
    assert_eq!(
        a.snapshot.result.as_ref().unwrap().reason,
        EndReason::Completed
    );
}

#[test]
fn oversized_payloads_are_not_retained_as_receipts_even_in_a_lobby() {
    let mut service = Service::default();
    let mut a: Bot = service
        .create("a", None, "A".into(), Mode::Race, 9, 0)
        .unwrap()
        .into();
    let bad = a.envelope(Command::Sync {
        base_version: 0,
        cells: vec![0; 8000],
    });
    for _ in 0..1000 {
        assert!(matches!(
            service.command(&a.snapshot.room_code, &bad, 0),
            Err(Error::InvalidInput)
        ));
    }
    a.state(&mut service, 0);
    assert_eq!(a.snapshot.own.next_sequence, 1);
}

#[test]
fn loopback_test_matrix_does_not_share_a_five_room_ceiling() {
    let mut service = Service::default();
    for _ in 0..12 {
        service
            .create("127.0.0.1", None, "Local".into(), Mode::Race, 6, 0)
            .unwrap();
    }
    for _ in 0..5 {
        service
            .create("192.0.2.1", None, "Remote".into(), Mode::Race, 6, 0)
            .unwrap();
    }
    assert!(matches!(
        service.create("192.0.2.1", None, "Remote".into(), Mode::Race, 6, 0),
        Err(Error::RateLimited)
    ));
}
