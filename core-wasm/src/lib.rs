use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub fn validate_board(cells: &[u8], size: u8) -> bool {
    sudoku_core::validate_board(cells, size)
}

/// Returns versioned offline state; errors are JavaScript exceptions.
#[wasm_bindgen]
pub fn new_solo(givens: &[u8], size: u8) -> Result<String, JsValue> {
    sudoku_core::SoloGame::new(size, givens.to_vec())
        .and_then(|game| game.save())
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Load validates the entire save before applying a tagged JSON command.
#[wasm_bindgen]
pub fn update_solo(saved: &str, command: &str) -> Result<String, JsValue> {
    let mut game = sudoku_core::SoloGame::load(saved)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    game.apply_json(command)
        .and_then(|_| game.save())
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

#[wasm_bindgen]
pub fn solo_is_complete(saved: &str) -> Result<bool, JsValue> {
    sudoku_core::SoloGame::load(saved)
        .map(|game| game.is_complete())
        .map_err(|error| JsValue::from_str(&error.to_string()))
}
