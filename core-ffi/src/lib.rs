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
    }
}
