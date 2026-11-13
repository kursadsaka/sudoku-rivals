use serde_json::Value;
use sudoku_core::validate_board;

#[test]
fn independent_boards_and_invalid_inputs() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../../vectors/boards.json")).unwrap();
    for fixture in fixtures {
        let size = fixture["size"].as_u64().unwrap() as u8;
        for (key, expected) in [
            ("uniquePuzzle", true),
            ("solution", true),
            ("invalidDuplicate", false),
            ("multipleSolutions", true),
        ] {
            let board: Vec<u8> = serde_json::from_value(fixture[key].clone()).unwrap();
            assert_eq!(validate_board(&board, size), expected, "{size} {key}");
        }
    }
    assert!(!validate_board(&[0; 36], 9));
    assert!(!validate_board(&[0; 36], 5));
    let mut board = [0; 36];
    board[0] = 7;
    assert!(!validate_board(&board, 6));
    let mut board = [0; 36];
    board[0] = 1;
    board[8] = 1;
    assert!(!validate_board(&board, 6));
}
