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
        let rated = super::new_rated_solo(9, 42, "hard".into()).unwrap();
        assert!(super::solo_difficulty(rated.clone())
            .unwrap()
            .contains(r#""level":"hard""#));
        assert!(super::solo_technique_hint(rated)
            .unwrap()
            .contains(r#""eliminations""#));
        assert!(super::new_rated_solo(6, 42, "expert".into()).is_ok());
        assert!(super::new_rated_solo(9, 42, "master".into()).is_err());
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

/// A new game whose puzzle needs exactly the techniques of `difficulty`
/// ("easy", "medium" or "hard"). Added next to `new_generated_solo`, which is
/// unchanged.
#[uniffi::export]
pub fn new_rated_solo(size: u8, seed: u32, difficulty: String) -> Result<String, SoloError> {
    let invalid = |reason: String| SoloError::InvalidInput { reason };
    let difficulty = sudoku_core::Difficulty::parse(&difficulty)
        .ok_or_else(|| invalid("InvalidValue".into()))?;
    let givens = sudoku_core::generate_rated_puzzle(size, seed, difficulty)
        .map_err(|error| invalid(error.to_string()))?;
    new_solo(givens, size)
}

/// JSON rating of the puzzle as given: {level,hardest,blanks,...}; level is
/// null when the supported techniques cannot finish it.
#[uniffi::export]
pub fn solo_difficulty(saved: String) -> Result<String, SoloError> {
    sudoku_core::SoloGame::load(&saved)?
        .difficulty_json()
        .map_err(|error| SoloError::InvalidInput {
            reason: error.to_string(),
        })
}

/// JSON null, or {index,value,technique,unit,eliminations}. `solo_hint` keeps
/// its singles-only answer for existing callers.
#[uniffi::export]
pub fn solo_technique_hint(saved: String) -> Result<String, SoloError> {
    sudoku_core::SoloGame::load(&saved)?
        .technique_hint_json()
        .map_err(|error| SoloError::InvalidInput {
            reason: error.to_string(),
        })
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

#[derive(Debug, uniffi::Error)]
pub enum ClientError {
    InvalidInput { reason: String },
    Unavailable,
}
impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput { reason } => f.write_str(reason),
            Self::Unavailable => f.write_str("Unavailable"),
        }
    }
}
impl std::error::Error for ClientError {}
impl From<sudoku_core::multiplayer::ClientError> for ClientError {
    fn from(error: sudoku_core::multiplayer::ClientError) -> Self {
        Self::InvalidInput {
            reason: error.to_string(),
        }
    }
}

/// Platform I/O executes the returned descriptors. Never log request bodies or
/// opaque saves: they carry seat credentials. view() contains only presentation.
#[derive(uniffi::Object)]
pub struct MultiplayerClient {
    inner: std::sync::Mutex<sudoku_core::multiplayer::MultiplayerClient>,
}

#[uniffi::export]
impl MultiplayerClient {
    #[uniffi::constructor]
    pub fn new(seed: u32) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            inner: std::sync::Mutex::new(sudoku_core::multiplayer::MultiplayerClient::new(seed)),
        })
    }

    #[uniffi::constructor]
    pub fn restore(saved: String) -> Result<std::sync::Arc<Self>, ClientError> {
        Ok(std::sync::Arc::new(Self {
            inner: std::sync::Mutex::new(sudoku_core::multiplayer::MultiplayerClient::load(
                &saved,
            )?),
        }))
    }

    pub fn dispatch(&self, action: String, now_ms: u64) -> Result<String, ClientError> {
        let mut client = self.inner.lock().map_err(|_| ClientError::Unavailable)?;
        Ok(client.dispatch_json(&action, now_ms)?)
    }

    pub fn view(&self, now_ms: u64) -> Result<String, ClientError> {
        let client = self.inner.lock().map_err(|_| ClientError::Unavailable)?;
        Ok(client.view_json(now_ms)?)
    }

    pub fn save(&self) -> Result<String, ClientError> {
        let client = self.inner.lock().map_err(|_| ClientError::Unavailable)?;
        Ok(client.save()?)
    }
}
