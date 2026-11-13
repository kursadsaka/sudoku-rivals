use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub fn validate_board(cells: &[u8], size: u8) -> bool {
    sudoku_core::validate_board(cells, size)
}
