use serde_json::Value;
use sudoku_core::{
    analyze_techniques, conflicting_cells, find_logical_hint, HintTechnique, HintUnit, PuzzleError,
    SoloCommand, SoloGame, UnitKind,
};

#[test]
fn both_sides_of_each_conflict_are_reported_without_empty_cells() {
    for size in [6, 9] {
        let n = size as usize;
        for other in [n - 1, n * (n - 1), n + 1] {
            let mut cells = vec![0; n * n];
            assert_eq!(conflicting_cells(&cells, size), Ok(vec![]));
            cells[0] = 1;
            cells[other] = 1;
            assert_eq!(conflicting_cells(&cells, size), Ok(vec![0, other as u32]));
        }
        let mut cells = vec![0; n * n];
        cells[0] = size + 1;
        assert_eq!(
            conflicting_cells(&cells, size),
            Err(PuzzleError::InvalidBoard)
        );
    }
    assert_eq!(
        conflicting_cells(&[0; 36], 255),
        Err(PuzzleError::InvalidBoard)
    );
    assert_eq!(conflicting_cells(&[], 6), Err(PuzzleError::InvalidBoard));
}

#[test]
fn naked_single_and_advisory_hint_do_not_change_saved_state() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../../vectors/boards.json")).unwrap();
    for f in fixtures {
        let size = f["size"].as_u64().unwrap() as u8;
        let mut solution: Vec<u8> = serde_json::from_value(f["solution"].clone()).unwrap();
        let value = solution[0];
        solution[0] = 0;
        let mut game = SoloGame::new(size, solution).unwrap();
        let saved = game.save().unwrap();
        let hint = game.hint().unwrap().unwrap();
        assert_eq!(hint.index, 0);
        assert_eq!(hint.value, value);
        assert_eq!(hint.technique, HintTechnique::NakedSingle);
        assert_eq!(hint.unit, None);
        assert_eq!(game.save().unwrap(), saved);
        assert!(!game.can_undo());
        game.apply(SoloCommand::Set {
            index: hint.index,
            value: hint.value,
        })
        .unwrap();
        assert!(game.is_complete());
        assert_eq!(game.hint().unwrap(), None);
        game.apply(SoloCommand::Undo {}).unwrap();
        assert!(!game.is_complete());
        assert_eq!(game.values()[0], 0);
    }
}

#[test]
fn hidden_single_explains_a_unit_without_using_solution_values() {
    // No empty cell has only one candidate. Row 1's digit 2 can only go in C2.
    let cells = [
        1, 0, 3, 0, 0, 0, 0, 0, 6, 0, 2, 3, 2, 0, 0, 0, 0, 0, 0, 6, 1, 0, 3, 0, 0, 0, 0, 0, 0, 2,
        0, 0, 0, 0, 0, 0,
    ];
    let hint = find_logical_hint(&cells, 6).unwrap().unwrap();
    assert_eq!(hint.index, 1);
    assert_eq!(hint.value, 2);
    assert_eq!(hint.technique, HintTechnique::HiddenSingle);
    assert_eq!(
        hint.unit,
        Some(HintUnit {
            kind: UnitKind::Row,
            index: 0
        })
    );
    assert_eq!(
        serde_json::to_value(hint).unwrap(),
        serde_json::json!({
            "index":1, "value":2, "technique":"hidden_single", "unit":{"kind":"row","index":0}
        })
    );
}

#[test]
fn unsupported_positions_and_bad_boards_never_get_a_guessed_hint() {
    assert_eq!(find_logical_hint(&[0; 81], 9), Ok(None));
    let analysis = analyze_techniques(&[0; 81], 9).unwrap();
    assert!(!analysis.solved_with_singles);
    assert_eq!(analysis.remaining_cells, 81);
    assert_eq!(
        analysis.naked_single_steps + analysis.hidden_single_steps,
        0
    );
    let mut cells = vec![0; 36];
    cells[1..6].copy_from_slice(&[1, 2, 3, 4, 5]);
    cells[6] = 6;
    assert_eq!(
        find_logical_hint(&cells, 6),
        Err(PuzzleError::UnsolvableBoard)
    );
    cells[0] = 1;
    assert_eq!(find_logical_hint(&cells, 6), Err(PuzzleError::InvalidBoard));
    assert_eq!(find_logical_hint(&[], 9), Err(PuzzleError::InvalidBoard));
}

#[test]
fn each_supported_step_matches_independent_fixture_solution() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../../vectors/boards.json")).unwrap();
    for f in fixtures {
        let size = f["size"].as_u64().unwrap() as u8;
        let mut cells: Vec<u8> = serde_json::from_value(f["uniquePuzzle"].clone()).unwrap();
        let expected: Vec<u8> = serde_json::from_value(f["solution"].clone()).unwrap();
        let analysis = analyze_techniques(&cells, size).unwrap();
        let mut steps = 0;
        while let Some(hint) = find_logical_hint(&cells, size).unwrap() {
            assert_eq!(hint.value, expected[hint.index as usize]);
            assert_eq!(cells[hint.index as usize], 0);
            cells[hint.index as usize] = hint.value;
            steps += 1;
            assert!(steps <= size as u32 * size as u32);
        }
        assert_eq!(
            steps,
            analysis.naked_single_steps + analysis.hidden_single_steps
        );
        assert!(analysis.solved_with_singles);
        assert_eq!(cells, expected);
    }
}

#[test]
fn adversarial_imported_hint_search_has_a_finite_work_budget() {
    let puzzle =
        ".......12........3..23..4....1..5....4.6.....3.....7......8...59.......1.6..2....";
    let mut cells: Vec<u8> = puzzle
        .bytes()
        .map(|c| if c == b'.' { 0 } else { c - b'0' })
        .collect();
    assert_eq!(cells.len(), 81);
    cells[39] = 1;
    assert!(sudoku_core::validate_board(&cells, 9));
    let game = SoloGame::new(9, cells).unwrap();
    assert_eq!(game.hint(), Err(PuzzleError::SearchLimit));
}
