uniffi::setup_scaffolding!();

#[uniffi::export]
pub fn validate_board(cells: Vec<u8>, size: u8) -> bool {
    sudoku_core::validate_board(&cells, size)
}

#[derive(Debug, uniffi::Error)]
pub enum SoloError {
    InvalidInput { reason: String },
}
impl std::fmt::Display for SoloError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput { reason } => f.write_str(reason),
        }
    }
}
impl std::error::Error for SoloError {}
impl From<sudoku_core::SoloError> for SoloError {
    fn from(error: sudoku_core::SoloError) -> Self {
        Self::InvalidInput {
            reason: error.to_string(),
        }
    }
}

/// Returns versioned offline state using only binding-safe values.
#[uniffi::export]
pub fn new_solo(givens: Vec<u8>, size: u8) -> Result<String, SoloError> {
    Ok(sudoku_core::SoloGame::new(size, givens)?.save()?)
}

#[uniffi::export]
pub fn update_solo(saved: String, command: String) -> Result<String, SoloError> {
    let mut game = sudoku_core::SoloGame::load(&saved)?;
    game.apply_json(&command)?;
    Ok(game.save()?)
}

#[uniffi::export]
pub fn solo_is_complete(saved: String) -> Result<bool, SoloError> {
    Ok(sudoku_core::SoloGame::load(&saved)?.is_complete())
}

#[cfg(test)]
mod tests {
    #[test]
    fn solo_boundary() {
        let saved = super::new_solo(vec![0; 36], 6).unwrap();
        let saved =
            super::update_solo(saved, r#"{"type":"set","index":0,"value":1}"#.into()).unwrap();
        assert!(!super::solo_is_complete(saved).unwrap());
        assert!(super::update_solo("{}".into(), "{}".into()).is_err());
        assert!(super::validate_board(vec![0; 36], 6));
        let empty = super::new_solo(vec![0; 36], 6).unwrap();
        assert_eq!(super::solo_hint(empty.clone()).unwrap(), "null");
        let first =
            super::update_solo(empty, r#"{"type":"set","index":0,"value":1}"#.into()).unwrap();
        let conflict =
            super::update_solo(first, r#"{"type":"set","index":1,"value":1}"#.into()).unwrap();
        assert_eq!(super::solo_conflicts(conflict.clone()).unwrap(), vec![0, 1]);
        assert!(super::solo_hint(conflict).is_err());
        assert!(super::new_generated_solo(6, 42, 18).is_ok());
    }
}

/// Blank count is a generation parameter, not a human difficulty rating.
#[uniffi::export]
pub fn new_generated_solo(size: u8, seed: u32, blanks: u8) -> Result<String, SoloError> {
    let givens = sudoku_core::generate_puzzle(size, seed, blanks).map_err(|error| {
        SoloError::InvalidInput {
            reason: error.to_string(),
        }
    })?;
    new_solo(givens, size)
}

#[uniffi::export]
pub fn solo_conflicts(saved: String) -> Result<Vec<u32>, SoloError> {
    Ok(sudoku_core::SoloGame::load(&saved)?.conflicts())
}

/// JSON null, or {index,value,technique,unit}; indices are zero-based.
#[uniffi::export]
pub fn solo_hint(saved: String) -> Result<String, SoloError> {
    sudoku_core::SoloGame::load(&saved)?
        .hint_json()
        .map_err(|error| SoloError::InvalidInput {
            reason: error.to_string(),
        })
}
