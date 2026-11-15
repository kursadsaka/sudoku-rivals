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

/// A new game whose puzzle needs exactly the techniques of `difficulty`
/// ("easy", "medium" or "hard"); see `sudoku_core::Difficulty`.
#[wasm_bindgen]
pub fn new_rated_solo(size: f64, seed: f64, difficulty: &str) -> Result<String, JsValue> {
    let size = checked_u8(size)?;
    if !seed.is_finite() || seed.fract() != 0.0 || !(0.0..=u32::MAX as f64).contains(&seed) {
        return Err(JsValue::from_str("InvalidValue"));
    }
    let difficulty = sudoku_core::Difficulty::parse(difficulty)
        .ok_or_else(|| JsValue::from_str("InvalidValue"))?;
    let givens = sudoku_core::generate_rated_puzzle(size, seed as u32, difficulty)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    new_solo(&givens, size as f64)
}

/// JSON rating of the puzzle as given: {level,hardest,blanks,...}; level is
/// null when the supported techniques cannot finish it (older saves).
#[wasm_bindgen]
pub fn solo_difficulty(saved: &str) -> Result<String, JsValue> {
    sudoku_core::SoloGame::load(saved)
        .map_err(|error| JsValue::from_str(&error.to_string()))?
        .difficulty_json()
        .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// JSON null, or {index,value,technique,unit,eliminations}: like `solo_hint`,
/// plus the candidate eliminations a harder step depends on.
#[wasm_bindgen]
pub fn solo_technique_hint(saved: &str) -> Result<String, JsValue> {
    sudoku_core::SoloGame::load(saved)
        .map_err(|error| JsValue::from_str(&error.to_string()))?
        .technique_hint_json()
        .map_err(|error| JsValue::from_str(&error.to_string()))
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

/// Stateful connection lifetime; restore is for an actual app restart, not for
/// every action. Callers execute HTTP descriptors without logging their bodies.
#[wasm_bindgen]
pub struct MultiplayerClient {
    inner: sudoku_core::multiplayer::MultiplayerClient,
}

#[wasm_bindgen]
impl MultiplayerClient {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: f64) -> Result<MultiplayerClient, JsValue> {
        Ok(Self {
            inner: sudoku_core::multiplayer::MultiplayerClient::new(checked_u32(seed)?),
        })
    }

    pub fn restore(saved: &str) -> Result<MultiplayerClient, JsValue> {
        Ok(Self {
            inner: sudoku_core::multiplayer::MultiplayerClient::load(saved)
                .map_err(|error| JsValue::from_str(&error.to_string()))?,
        })
    }

    pub fn dispatch(&mut self, action: &str, now_ms: f64) -> Result<String, JsValue> {
        self.inner
            .dispatch_json(action, checked_time(now_ms)?)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    pub fn view(&self, now_ms: f64) -> Result<String, JsValue> {
        self.inner
            .view_json(checked_time(now_ms)?)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }

    pub fn save(&self) -> Result<String, JsValue> {
        self.inner
            .save()
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }
}

fn checked_u32(value: f64) -> Result<u32, JsValue> {
    if value.is_finite() && value.fract() == 0.0 && (0.0..=u32::MAX as f64).contains(&value) {
        Ok(value as u32)
    } else {
        Err(JsValue::from_str("InvalidValue"))
    }
}
fn checked_time(value: f64) -> Result<u64, JsValue> {
    if value.is_finite() && value.fract() == 0.0 && (0.0..=9_007_199_254_740_991.0).contains(&value)
    {
        Ok(value as u64)
    } else {
        Err(JsValue::from_str("InvalidTime"))
    }
}
