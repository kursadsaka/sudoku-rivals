use serde_json::{json, Value};
use sudoku_core::{is_complete, validate_board, SoloCommand::*, SoloError, SoloGame};

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!("../../vectors/boards.json")).unwrap()
}

#[test]
fn fixture_games_protect_givens_and_complete_in_both_sizes() {
    for f in fixtures() {
        let size = f["size"].as_u64().unwrap() as u8;
        let givens: Vec<u8> = serde_json::from_value(f["uniquePuzzle"].clone()).unwrap();
        let solution: Vec<u8> = serde_json::from_value(f["solution"].clone()).unwrap();
        let mut game = SoloGame::new(size, givens.clone()).unwrap();
        assert!(game.is_valid());
        assert!(!game.is_complete());
        for (index, &given) in givens.iter().enumerate() {
            if given == 0 {
                game.apply(Set {
                    index: index as u32,
                    value: solution[index],
                })
                .unwrap();
            } else {
                let before = game.clone();
                for cmd in [
                    Set {
                        index: index as u32,
                        value: given,
                    },
                    Erase {
                        index: index as u32,
                    },
                    ToggleNote {
                        index: index as u32,
                        value: 1,
                    },
                ] {
                    assert_eq!(game.apply(cmd), Err(SoloError::GivenCell));
                    assert_eq!(game, before);
                }
            }
        }
        assert!(game.is_complete());
        assert_eq!(game.values(), solution);
        let saved = game.save().unwrap();
        assert_eq!(SoloGame::load(&saved).unwrap(), game);
        assert!(game.apply(Undo).unwrap());
        assert!(!game.is_complete());
        assert!(game.apply(Redo).unwrap());
        assert!(game.is_complete());
        let bad = serde_json::from_value(f["invalidDuplicate"].clone()).unwrap();
        assert_eq!(SoloGame::new(size, bad), Err(SoloError::InvalidBoard));
        assert!(SoloGame::new(size, vec![0; size as usize * size as usize]).is_ok());
    }
}

#[test]
fn isolated_row_column_and_box_conflicts_and_shape_errors() {
    for size in [6, 9] {
        let n = size as usize;
        for other in [n - 1, (n - 1) * n, n + 1] {
            let mut cells = vec![0; n * n];
            cells[0] = 1;
            cells[other] = 1;
            assert!(!validate_board(&cells, size));
            assert!(!is_complete(&cells, size));
        }
        for length in [0, n * n - 1, n * n + 1] {
            assert_eq!(
                SoloGame::new(size, vec![0; length]),
                Err(SoloError::InvalidBoard)
            );
        }
        let mut board = vec![0; n * n];
        board[n * n - 1] = size + 1;
        assert!(!validate_board(&board, size));
    }
    for size in [0, 1, 5, 255] {
        assert!(SoloGame::new(size, vec![]).is_err());
    }
}

#[test]
fn notes_erase_history_branching_and_atomic_errors() {
    for size in [6, 9] {
        let mut game = SoloGame::new(size, vec![0; size as usize * size as usize]).unwrap();
        assert!(!game.apply(Undo).unwrap());
        assert!(!game.apply(Redo).unwrap());
        for value in [1, size] {
            game.apply(ToggleNote { index: 0, value }).unwrap();
        }
        assert_eq!(game.notes(0).unwrap(), vec![1, size]);
        game.apply(ToggleNote { index: 0, value: 1 }).unwrap();
        assert_eq!(game.notes(0).unwrap(), vec![size]);
        game.apply(Set { index: 0, value: 1 }).unwrap();
        assert!(game.notes(0).unwrap().is_empty());
        for cmd in [
            ToggleNote { index: 0, value: 1 },
            Set {
                index: u32::MAX,
                value: 1,
            },
            Set {
                index: 1,
                value: size + 1,
            },
            ToggleNote { index: 1, value: 0 },
            ToggleNote {
                index: 1,
                value: 255,
            },
            Erase { index: u32::MAX },
        ] {
            let before = game.clone();
            assert!(game.apply(cmd).is_err());
            assert_eq!(game, before);
        }
        game.apply(Set { index: 1, value: 1 }).unwrap();
        assert!(!game.is_valid()); // Conflicts remain editable and serializable.
        assert_eq!(SoloGame::load(&game.save().unwrap()).unwrap(), game);
        game.apply(Undo).unwrap();
        assert!(game.is_valid());
        let before = game.clone();
        assert!(!game.apply(Erase { index: 1 }).unwrap());
        assert_eq!(game, before);
        assert_eq!(SoloGame::load(&game.save().unwrap()).unwrap(), game);
        game.apply(Set { index: 1, value: 2 }).unwrap();
        assert!(!game.can_redo());
        game.apply(Erase { index: 0 }).unwrap();
        game.apply(Undo).unwrap();
        assert_eq!(game.values()[0], 1);
        game.apply(Undo).unwrap();
        game.apply(Undo).unwrap();
        assert_eq!(game.notes(0).unwrap(), vec![size]);
        game.apply(Erase { index: 0 }).unwrap();
        assert!(game.notes(0).unwrap().is_empty());
        game.apply(Undo).unwrap();
        assert_eq!(game.notes(0).unwrap(), vec![size]);
        assert!(game.notes(u32::MAX).is_err());
    }
}

#[test]
fn corrupt_saves_never_create_unsafe_state() {
    let mut game = SoloGame::new(6, vec![0; 36]).unwrap();
    game.apply(Set { index: 0, value: 1 }).unwrap();
    game.apply(ToggleNote { index: 1, value: 6 }).unwrap();
    game.apply(Undo).unwrap();
    let saved = game.save().unwrap();
    let original: Value = serde_json::from_str(&saved).unwrap();
    for (path, replacement) in [
        ("/version", json!(2)),
        ("/size", json!(255)),
        ("/givens", json!([])),
        ("/givens/0", json!(2)),
        ("/cells", json!([])),
        ("/cells/0/value", json!(7)),
        ("/cells/0/notes", json!(1)),
        ("/cells/1/notes", json!(64)),
        ("/undo/0/index", json!(4294967295u32)),
        ("/undo/0/before/value", json!(7)),
        ("/undo/0/after/value", json!(2)),
        ("/redo/0/before/notes", json!(1)),
        ("/redo/0/after/notes", json!(128)),
    ] {
        let mut corrupt = original.clone();
        *corrupt.pointer_mut(path).unwrap() = replacement;
        let json = corrupt.to_string();
        assert!(SoloGame::load(&json).is_err(), "{path}");
        // Public serde deserialization enforces invariants too.
        assert!(serde_json::from_str::<SoloGame>(&json).is_err(), "{path}");
    }
    for input in ["", "null", "[]", "{}", "{", &saved[..saved.len() - 1]] {
        assert!(SoloGame::load(input).is_err());
    }
    let mut corrupt = original.clone();
    corrupt["extra"] = json!(true);
    assert!(SoloGame::load(&corrupt.to_string()).is_err());
    corrupt = original.clone();
    corrupt["undo"] = json!(vec![original["undo"][0].clone(); 4097]);
    assert!(SoloGame::load(&corrupt.to_string()).is_err());
    assert!(SoloGame::load(&" ".repeat(1_048_577)).is_err());
    let before = game.clone();
    for cmd in [
        "{}",
        "null",
        r#"{"type":"set","index":-1,"value":1}"#,
        r#"{"type":"erase","index":1,"extra":1}"#,
        r#"{"type":"hint"}"#,
    ] {
        assert!(game.apply_json(cmd).is_err());
        assert_eq!(game, before);
    }
    assert_eq!(SoloGame::load(&saved).unwrap(), game);
}

#[test]
fn bounded_history_remains_loadable_after_rollover_and_replay() {
    let mut game = SoloGame::new(6, vec![0; 36]).unwrap();
    for i in 0..4100 {
        game.apply(Set {
            index: 0,
            value: (i % 2 + 1) as u8,
        })
        .unwrap();
    }
    for _ in 0..4096 {
        assert!(game.apply(Undo).unwrap());
    }
    assert!(!game.can_undo());
    assert_eq!(SoloGame::load(&game.save().unwrap()).unwrap(), game);
    for _ in 0..4096 {
        assert!(game.apply(Redo).unwrap());
    }
    assert!(!game.can_redo());
    assert_eq!(SoloGame::load(&game.save().unwrap()).unwrap(), game);
}
