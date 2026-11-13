pub mod multiplayer;
pub mod protocol;

mod puzzle;
pub use puzzle::{conflicting_cells, count_solutions, generate_puzzle, solve_board, PuzzleError};

mod hint;
pub use hint::{
    analyze_techniques, find_logical_hint, HintTechnique, HintUnit, LogicalHint, TechniqueAnalysis,
    UnitKind,
};

mod solo;
pub use solo::{SoloCommand, SoloError, SoloGame};

/// Checks whether a partially filled 6×6 or 9×9 board has conflicts.
/// Zero represents an empty cell. This does not check solvability or completion.
pub fn validate_board(cells: &[u8], size: u8) -> bool {
    conflicting_cells(cells, size).is_ok_and(|conflicts| conflicts.is_empty())
}

/// A complete board obeys all constraints and has no empty cells.
/// Neither this nor validation establishes uniqueness or puzzle difficulty.
pub fn is_complete(cells: &[u8], size: u8) -> bool {
    validate_board(cells, size) && !cells.contains(&0)
}
