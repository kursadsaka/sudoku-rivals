use serde_json::Value;
use sudoku_core::{
    count_solutions, find_technique_hint, generate_puzzle, generate_rated_puzzle, rate_puzzle,
    solve_board, Difficulty, HintTechnique, PuzzleError, SoloGame, Technique, UnitKind,
};

/// What following the hints to the end of a puzzle takes.
#[derive(Default)]
struct HintPath {
    /// Hidden singles shown in a row or column that is not just its last cell.
    line_singles: u32,
    naked_singles: u32,
    locked_candidates: u32,
    pairs: u32,
}

/// Follows hints to the end. Every step must be explainable and must agree
/// with the solution.
fn solve_with_hints(puzzle: &[u8], size: u8) -> HintPath {
    let solution = solve_board(puzzle, size).unwrap().unwrap();
    let mut cells = puzzle.to_vec();
    let mut path = HintPath::default();
    while cells.contains(&0) {
        let hint = find_technique_hint(&cells, size)
            .unwrap()
            .expect("a rated puzzle always has a next step");
        assert_eq!(cells[hint.index as usize], 0);
        assert_eq!(hint.value, solution[hint.index as usize]);
        assert_eq!(
            hint.unit.is_some(),
            hint.technique == HintTechnique::HiddenSingle
        );
        for elimination in &hint.eliminations {
            assert!(elimination.technique >= Technique::LockedCandidates);
            let locked = elimination.technique == Technique::LockedCandidates;
            assert_eq!(elimination.digits.len(), if locked { 1 } else { 2 });
            assert_eq!(elimination.target.is_some(), locked);
            assert!(elimination.cells.len() >= 2);
            assert!(elimination
                .cells
                .iter()
                .all(|&cell| cells[cell as usize] == 0));
            if locked {
                path.locked_candidates += 1;
            } else {
                path.pairs += 1;
            }
        }
        match (hint.technique, hint.unit) {
            (HintTechnique::NakedSingle, _) => path.naked_singles += 1,
            (_, Some(unit)) if unit.kind != UnitKind::Box => {
                // The last empty cell of a row or column belongs to the first level.
                let n = size as usize;
                let (row, column) = (hint.index as usize / n, hint.index as usize % n);
                let open = (0..n)
                    .filter(|&i| {
                        let cell = if unit.kind == UnitKind::Row {
                            row * n + i
                        } else {
                            i * n + column
                        };
                        cells[cell] == 0
                    })
                    .count();
                if open > 1 && hint.eliminations.is_empty() {
                    path.line_singles += 1;
                }
            }
            _ => {}
        }
        cells[hint.index as usize] = hint.value;
    }
    assert_eq!(cells, solution);
    path
}

#[test]
fn rated_puzzles_match_their_level_are_unique_and_fully_explainable() {
    for size in [6u8, 9] {
        for level in Difficulty::ALL {
            for seed in [0, 1, 42, 7_777, u32::MAX] {
                let puzzle = generate_rated_puzzle(size, seed, level).unwrap();
                assert_eq!(generate_rated_puzzle(size, seed, level).unwrap(), puzzle);
                assert_eq!(count_solutions(&puzzle, size, 2), Ok(1));
                let rating = rate_puzzle(&puzzle, size).unwrap();
                assert_eq!(rating.level, Some(level), "{size} {level:?} {seed}");
                assert_eq!(rating.remaining_cells, 0);
                assert_eq!(
                    rating.blanks as usize,
                    puzzle.iter().filter(|&&value| value == 0).count()
                );
                let path = solve_with_hints(&puzzle, size);
                let eliminations = path.locked_candidates + path.pairs;
                match level {
                    // Boxes and last cells only.
                    Difficulty::Beginner => {
                        assert_eq!(rating.hardest, Some(Technique::BoxSingle));
                        assert_eq!(path.line_singles + path.naked_singles + eliminations, 0);
                    }
                    // A row or column has to show a digit's only place.
                    Difficulty::Easy => {
                        assert_eq!(rating.hardest, Some(Technique::LineSingle));
                        assert!(rating.line_singles > 0);
                        assert!(path.line_singles > 0);
                        assert_eq!(path.naked_singles + eliminations, 0);
                    }
                    // A cell has to be solved by counting its candidates.
                    Difficulty::Medium => {
                        assert_eq!(rating.hardest, Some(Technique::NakedSingle));
                        assert!(rating.naked_singles > 0);
                        assert!(path.naked_singles > 0);
                        assert_eq!(eliminations, 0);
                    }
                    // Singles get stuck; locked candidates free them, pairs are never needed.
                    Difficulty::Hard => {
                        assert_eq!(rating.hardest, Some(Technique::LockedCandidates));
                        assert!(rating.locked_candidates > 0);
                        assert!(path.locked_candidates > 0);
                        assert_eq!(rating.naked_pairs + rating.hidden_pairs, 0);
                    }
                    // Locked candidates get stuck too; a pair is needed.
                    Difficulty::Expert => {
                        assert!(rating.hardest >= Some(Technique::NakedPair));
                        assert!(rating.naked_pairs + rating.hidden_pairs > 0);
                        assert!(path.pairs > 0);
                    }
                }
            }
        }
    }
}

#[test]
fn every_level_needs_strictly_more_than_the_level_below() {
    // The level is the lowest technique set that finishes the puzzle, so a puzzle
    // of one level is never also rated as another, in either size.
    for size in [6u8, 9] {
        let mut seen = Vec::new();
        for level in Difficulty::ALL {
            let puzzle = generate_rated_puzzle(size, 2026, level).unwrap();
            assert_eq!(rate_puzzle(&puzzle, size).unwrap().level, Some(level));
            seen.push(puzzle);
        }
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), Difficulty::ALL.len());
    }
    assert!(Difficulty::Beginner < Difficulty::Easy && Difficulty::Hard < Difficulty::Expert);
}

#[test]
fn six_by_six_expert_bank_is_real_and_its_rearrangements_keep_the_level() {
    // Every bank puzzle is what the search calls Expert, with one solution.
    let bank = sudoku_core::expert_6x6_bank();
    let mut distinct = std::collections::BTreeSet::new();
    for text in bank {
        let puzzle: Vec<u8> = text.bytes().map(|digit| digit - b'0').collect();
        assert_eq!(puzzle.len(), 36);
        assert_eq!(count_solutions(&puzzle, 6, 2), Ok(1));
        let rating = rate_puzzle(&puzzle, 6).unwrap();
        assert_eq!(rating.level, Some(Difficulty::Expert));
        assert!(rating.naked_pairs + rating.hidden_pairs > 0);
        distinct.insert(puzzle);
    }
    assert_eq!(distinct.len(), bank.len());
    // A seed picks a bank puzzle and re-arranges it; the result is a different
    // board that needs exactly the same techniques.
    let mut produced = std::collections::BTreeSet::new();
    for seed in 0..400u32 {
        let puzzle =
            generate_rated_puzzle(6, seed.wrapping_mul(2_654_435_761), Difficulty::Expert).unwrap();
        assert_eq!(count_solutions(&puzzle, 6, 2), Ok(1));
        assert_eq!(
            rate_puzzle(&puzzle, 6).unwrap().level,
            Some(Difficulty::Expert)
        );
        produced.insert(puzzle);
    }
    assert!(produced.len() >= 399, "{} distinct of 400", produced.len());
    assert!(produced.iter().all(|puzzle| !distinct.contains(puzzle)));
    // The bank entries are reproducible by the on-demand search.
    let searched = sudoku_core::search_rated_puzzle(6, 17, Difficulty::Expert).unwrap();
    assert_eq!(
        rate_puzzle(&searched, 6).unwrap().level,
        Some(Difficulty::Expert)
    );
}

#[test]
fn rating_reports_unsupported_and_invalid_boards_honestly() {
    // An empty board has many solutions: no technique applies, so no level is claimed.
    let open = rate_puzzle(&[0; 81], 9).unwrap();
    assert_eq!(
        (open.level, open.hardest, open.remaining_cells),
        (None, None, 81)
    );
    assert_eq!(find_technique_hint(&[0; 81], 9), Ok(None));
    // A solved board needs nothing.
    let solved = solve_board(&generate_rated_puzzle(6, 3, Difficulty::Easy).unwrap(), 6)
        .unwrap()
        .unwrap();
    assert_eq!(
        rate_puzzle(&solved, 6).unwrap().level,
        Some(Difficulty::Beginner)
    );
    assert_eq!(find_technique_hint(&solved, 6), Ok(None));
    let mut conflict = vec![0; 36];
    conflict[0] = 1;
    conflict[1] = 1;
    assert_eq!(rate_puzzle(&conflict, 6), Err(PuzzleError::InvalidBoard));
    assert_eq!(
        find_technique_hint(&conflict, 6),
        Err(PuzzleError::InvalidBoard)
    );
    assert_eq!(rate_puzzle(&[0; 16], 4), Err(PuzzleError::InvalidBoard));
    assert_eq!(
        generate_rated_puzzle(4, 0, Difficulty::Easy),
        Err(PuzzleError::InvalidBoard)
    );
    // A wrong but conflict-free digit makes the board unsatisfiable: an error, never a guess.
    let puzzle = generate_rated_puzzle(9, 5, Difficulty::Medium).unwrap();
    let solution = solve_board(&puzzle, 9).unwrap().unwrap();
    let mut wrong = puzzle.clone();
    let cell = puzzle.iter().position(|&value| value == 0).unwrap();
    let candidate = (1..=9u8)
        .find(|&value| {
            value != solution[cell] && {
                wrong[cell] = value;
                sudoku_core::validate_board(&wrong, 9)
            }
        })
        .unwrap();
    wrong[cell] = candidate;
    assert_eq!(
        find_technique_hint(&wrong, 9),
        Err(PuzzleError::UnsolvableBoard)
    );
    assert_eq!(Difficulty::parse("expert"), Some(Difficulty::Expert));
    assert_eq!(Difficulty::parse("Hard"), None);
    assert_eq!(Difficulty::parse("master"), None);
    assert!(Difficulty::ALL
        .iter()
        .all(|&level| Difficulty::parse(level.name()) == Some(level)));
    let names: Vec<&str> = Difficulty::ALL.iter().map(|level| level.name()).collect();
    assert_eq!(names, ["beginner", "easy", "medium", "hard", "expert"]);
}

#[test]
fn solo_game_rates_its_givens_without_changing_the_save_format() {
    let puzzle = generate_rated_puzzle(9, 9, Difficulty::Expert).unwrap();
    let mut game = SoloGame::new(9, puzzle.clone()).unwrap();
    let saved = game.save().unwrap();
    // Same six fields as before: the rating is derived, not stored.
    let keys: Vec<String> = serde_json::from_str::<serde_json::Map<String, Value>>(&saved)
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(keys, ["cells", "givens", "redo", "size", "undo", "version"]);
    let rating: Value = serde_json::from_str(&game.difficulty_json().unwrap()).unwrap();
    assert_eq!(rating["level"], "expert");
    // Progress does not change the puzzle's rating.
    let hint: Value = serde_json::from_str(&game.technique_hint_json().unwrap()).unwrap();
    game.apply_json(&format!(
        r#"{{"type":"set","index":{},"value":{}}}"#,
        hint["index"], hint["value"]
    ))
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&game.difficulty_json().unwrap()).unwrap()["level"],
        "expert"
    );
    // Puzzles from the blank-count generator are rated too, or left unrated: never mislabelled.
    for seed in 0..20 {
        let legacy = generate_puzzle(9, seed, 43).unwrap();
        let rating = rate_puzzle(&legacy, 9).unwrap();
        assert_eq!(rating.level.is_some(), rating.remaining_cells == 0);
    }
}

#[test]
fn cross_platform_rated_vectors() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../../vectors/boards.json")).unwrap();
    for fixture in fixtures {
        let size = fixture["size"].as_u64().unwrap() as u8;
        let seed = fixture["rated"]["seed"].as_u64().unwrap() as u32;
        for level in Difficulty::ALL {
            let expected: Vec<u8> =
                serde_json::from_value(fixture["rated"][level.name()].clone()).unwrap();
            assert_eq!(generate_rated_puzzle(size, seed, level).unwrap(), expected);
        }
    }
}

/// Yield and timing behind the level windows and attempt bounds. Run with
/// `cargo test -p sudoku-core --release --test difficulty -- --ignored --nocapture`
/// (`SEEDS=200` for a quick pass).
#[test]
#[ignore]
fn calibration_report() {
    let seeds: u32 = std::env::var("SEEDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(2000);
    for size in [6u8, 9] {
        for level in Difficulty::ALL {
            let started = std::time::Instant::now();
            let (mut failed, mut blanks) = (0, Vec::new());
            let mut times = Vec::new();
            let mut hardest = std::collections::BTreeMap::new();
            for seed in 0..seeds {
                let one = std::time::Instant::now();
                match generate_rated_puzzle(size, seed.wrapping_mul(2_654_435_761), level) {
                    Ok(puzzle) => {
                        let rating = rate_puzzle(&puzzle, size).unwrap();
                        assert_eq!(rating.level, Some(level));
                        blanks.push(rating.blanks);
                        *hardest
                            .entry(format!("{:?}", rating.hardest.unwrap()))
                            .or_insert(0) += 1;
                    }
                    Err(_) => failed += 1,
                }
                times.push(one.elapsed());
            }
            blanks.sort_unstable();
            times.sort_unstable();
            println!(
                "{size}x{size} {level:?}: failed {failed}/{seeds}, mean {:?}, p99 {:?}, slowest {:?}, blanks min {} median {} max {}, hardest {hardest:?}",
                started.elapsed() / seeds,
                times[times.len() * 99 / 100],
                times.last().unwrap(),
                blanks.first().unwrap_or(&0),
                blanks.get(blanks.len() / 2).unwrap_or(&0),
                blanks.last().unwrap_or(&0),
            );
        }
    }
}
