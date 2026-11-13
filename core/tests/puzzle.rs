use serde_json::Value;
use sudoku_core::{count_solutions, generate_puzzle, is_complete, solve_board, PuzzleError};

#[test]
fn fixture_solution_counts_and_solver_preserve_givens() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../../vectors/boards.json")).unwrap();
    for f in fixtures {
        let size = f["size"].as_u64().unwrap() as u8;
        let puzzle: Vec<u8> = serde_json::from_value(f["uniquePuzzle"].clone()).unwrap();
        let expected: Vec<u8> = serde_json::from_value(f["solution"].clone()).unwrap();
        assert_eq!(count_solutions(&puzzle, size, 2), Ok(1));
        assert_eq!(solve_board(&puzzle, size), Ok(Some(expected.clone())));
        assert_eq!(count_solutions(&expected, size, 2), Ok(1));
        let empty: Vec<u8> = serde_json::from_value(f["multipleSolutions"].clone()).unwrap();
        assert_eq!(count_solutions(&empty, size, 2), Ok(2));
        assert_eq!(count_solutions(&empty, size, 0), Ok(0));
        let invalid: Vec<u8> = serde_json::from_value(f["invalidDuplicate"].clone()).unwrap();
        assert_eq!(solve_board(&invalid, size), Err(PuzzleError::InvalidBoard));
    }
}

#[test]
fn valid_partial_can_still_be_unsatisfiable() {
    // Cell 0 needs 6 from its row, but its column already contains 6.
    let mut cells = vec![0; 36];
    cells[1..6].copy_from_slice(&[1, 2, 3, 4, 5]);
    cells[6] = 6;
    assert!(sudoku_core::validate_board(&cells, 6));
    assert_eq!(solve_board(&cells, 6), Ok(None));
    assert_eq!(count_solutions(&cells, 6, 2), Ok(0));
    assert_eq!(solve_board(&[], 9), Err(PuzzleError::InvalidBoard));
    assert_eq!(count_solutions(&[], 255, 2), Err(PuzzleError::InvalidBoard));
}

#[test]
fn generated_puzzles_are_deterministic_unique_and_have_exact_blanks() {
    for (size, blanks) in [(6, 18), (9, 43)] {
        for seed in 0..256 {
            let puzzle = generate_puzzle(size, seed, blanks).unwrap();
            assert_eq!(generate_puzzle(size, seed, blanks).unwrap(), puzzle);
            assert_eq!(puzzle.iter().filter(|&&v| v == 0).count(), blanks as usize);
            assert_eq!(count_solutions(&puzzle, size, 2), Ok(1));
            let solution = solve_board(&puzzle, size).unwrap().unwrap();
            assert!(is_complete(&solution, size));
            assert!(puzzle
                .iter()
                .zip(solution)
                .all(|(&given, value)| given == 0 || given == value));
        }
        assert!(is_complete(
            &generate_puzzle(size, u32::MAX, 0).unwrap(),
            size
        ));
    }
    assert_eq!(
        generate_puzzle(6, 0, 37),
        Err(PuzzleError::InvalidBlankCount)
    );
    assert_eq!(generate_puzzle(5, 0, 0), Err(PuzzleError::InvalidBoard));
    assert_eq!(
        generate_puzzle(6, 0, 36),
        Err(PuzzleError::CannotReachBlankCount)
    );
}

#[test]
fn cross_platform_seed_vectors() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../../vectors/boards.json")).unwrap();
    for f in fixtures {
        let size = f["size"].as_u64().unwrap() as u8;
        let expected: Vec<u8> = serde_json::from_value(f["generated"]["givens"].clone()).unwrap();
        assert_eq!(
            generate_puzzle(size, 42, f["generated"]["blanks"].as_u64().unwrap() as u8).unwrap(),
            expected
        );
    }
}
