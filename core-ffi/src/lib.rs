uniffi::setup_scaffolding!();

#[uniffi::export]
pub fn validate_board(cells: Vec<u8>, size: u8) -> bool {
    sudoku_core::validate_board(&cells, size)
}
