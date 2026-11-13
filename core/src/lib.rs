mod solo;
pub use solo::{SoloCommand, SoloError, SoloGame};

/// Checks whether a partially filled 6×6 or 9×9 board has conflicts.
/// Zero represents an empty cell. This does not check solvability or completion.
pub fn validate_board(cells: &[u8], size: u8) -> bool {
    let (n, box_rows, box_columns) = match size {
        6 => (6, 2, 3),
        9 => (9, 3, 3),
        _ => return false,
    };
    if cells.len() != n * n || cells.iter().any(|&value| value as usize > n) {
        return false;
    }
    for row in 0..n {
        for col in 0..n {
            let value = cells[row * n + col];
            if value == 0 {
                continue;
            }
            for other in 0..n {
                if (other != col && cells[row * n + other] == value)
                    || (other != row && cells[other * n + col] == value)
                {
                    return false;
                }
            }
            let start_row = row / box_rows * box_rows;
            let start_col = col / box_columns * box_columns;
            for r in start_row..start_row + box_rows {
                for c in start_col..start_col + box_columns {
                    if (r != row || c != col) && cells[r * n + c] == value {
                        return false;
                    }
                }
            }
        }
    }
    true
}

/// A complete board obeys all constraints and has no empty cells.
/// Neither this nor validation establishes uniqueness or puzzle difficulty.
pub fn is_complete(cells: &[u8], size: u8) -> bool {
    validate_board(cells, size) && !cells.contains(&0)
}
