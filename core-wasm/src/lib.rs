use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub fn validate_board(cells: &[u8], size: f64) -> bool {
    checked_u8(size).is_ok_and(|size| sudoku_core::validate_board(cells, size))
}

/// Returns versioned offline state; errors are JavaScript exceptions.
#[wasm_bindgen]
pub fn new_solo(givens: &[u8], size: f64) -> Result<String, JsValue> {
    sudoku_core::SoloGame::new(checked_u8(size)?, givens.to_vec())
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

/// Blank count is a generation parameter, not a human difficulty rating.
#[wasm_bindgen]
pub fn new_generated_solo(size: f64, seed: f64, blanks: f64) -> Result<String, JsValue> {
    let size = checked_u8(size)?;
    let blanks = checked_u8(blanks)?;
    if !seed.is_finite() || seed.fract() != 0.0 || !(0.0..=u32::MAX as f64).contains(&seed) {
        return Err(JsValue::from_str("InvalidValue"));
    }
    let givens = sudoku_core::generate_puzzle(size, seed as u32, blanks)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    new_solo(&givens, size as f64)
}

#[wasm_bindgen]
pub fn solo_conflicts(saved: &str) -> Result<Vec<u32>, JsValue> {
    sudoku_core::SoloGame::load(saved)
        .map(|game| game.conflicts())
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// JSON null, or {index,value,technique,unit}; indices are zero-based.
#[wasm_bindgen]
pub fn solo_hint(saved: &str) -> Result<String, JsValue> {
    sudoku_core::SoloGame::load(saved)
        .map_err(|error| JsValue::from_str(&error.to_string()))?
        .hint_json()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

fn checked_u8(value: f64) -> Result<u8, JsValue> {
    if value.is_finite() && value.fract() == 0.0 && (0.0..=u8::MAX as f64).contains(&value) {
        Ok(value as u8)
    } else {
        Err(JsValue::from_str("InvalidValue"))
    }
}
