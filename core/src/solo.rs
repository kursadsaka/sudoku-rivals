use serde::{Deserialize, Serialize};
use std::fmt;

const VERSION: u8 = 1;
// Resource limits for untrusted saves, not puzzle or difficulty rules.
const MAX_SAVE_BYTES: usize = 1_048_576;
const MAX_HISTORY: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoloError {
    InvalidBoard,
    InvalidCell,
    InvalidValue,
    GivenCell,
    FilledCell,
    InvalidSave,
    InvalidCommand,
    UnsupportedVersion,
}
impl fmt::Display for SoloError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SoloError {}

/// Zero-based cell indices and digits 1..=size at every platform boundary.
/// Set accepts zero to erase. Invalid commands leave the game unchanged.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SoloCommand {
    Set { index: u32, value: u8 },
    ToggleNote { index: u32, value: u8 },
    Erase { index: u32 },
    Undo {},
    Redo {},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cell {
    value: u8,
    // Bit zero is digit 1. Notes are manual and need not be legal candidates.
    notes: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Edit {
    index: u32,
    before: Cell,
    after: Cell,
}

/// Offline solo state. Conflicting player digits are retained so callers can
/// report mistakes; givens must form a valid partial board. No solver is implied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SavedGame")]
pub struct SoloGame {
    version: u8,
    size: u8,
    givens: Vec<u8>,
    cells: Vec<Cell>,
    undo: Vec<Edit>,
    redo: Vec<Edit>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedGame {
    version: u8,
    size: u8,
    givens: Vec<u8>,
    cells: Vec<Cell>,
    undo: Vec<Edit>,
    redo: Vec<Edit>,
}
impl TryFrom<SavedGame> for SoloGame {
    type Error = SoloError;
    fn try_from(saved: SavedGame) -> Result<Self, Self::Error> {
        let game = Self {
            version: saved.version,
            size: saved.size,
            givens: saved.givens,
            cells: saved.cells,
            undo: saved.undo,
            redo: saved.redo,
        };
        game.check()?;
        Ok(game)
    }
}

impl SoloGame {
    pub fn new(size: u8, givens: Vec<u8>) -> Result<Self, SoloError> {
        if !crate::validate_board(&givens, size) {
            return Err(SoloError::InvalidBoard);
        }
        Ok(Self {
            version: VERSION,
            size,
            cells: givens
                .iter()
                .map(|&value| Cell { value, notes: 0 })
                .collect(),
            givens,
            undo: Vec::new(),
            redo: Vec::new(),
        })
    }

    pub fn size(&self) -> u8 {
        self.size
    }
    pub fn givens(&self) -> &[u8] {
        &self.givens
    }
    pub fn values(&self) -> Vec<u8> {
        self.cells.iter().map(|c| c.value).collect()
    }
    pub fn notes(&self, index: u32) -> Result<Vec<u8>, SoloError> {
        let cell = self
            .cells
            .get(index as usize)
            .ok_or(SoloError::InvalidCell)?;
        Ok((1..=self.size)
            .filter(|v| cell.notes & (1 << (v - 1)) != 0)
            .collect())
    }
    pub fn is_valid(&self) -> bool {
        crate::validate_board(&self.values(), self.size)
    }
    pub fn is_complete(&self) -> bool {
        crate::is_complete(&self.values(), self.size)
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Returns whether the command changed state. No-ops preserve redo history.
    /// Setting a digit clears that cell's notes; erase clears digit and notes.
    /// Notes in peer cells are never changed automatically.
    pub fn apply(&mut self, command: SoloCommand) -> Result<bool, SoloError> {
        let (index, after) = match command {
            SoloCommand::Undo {} => {
                if let Some(edit) = self.undo.pop() {
                    self.cells[edit.index as usize] = edit.before;
                    self.redo.push(edit);
                    return Ok(true);
                }
                return Ok(false);
            }
            SoloCommand::Redo {} => {
                if let Some(edit) = self.redo.pop() {
                    self.cells[edit.index as usize] = edit.after;
                    self.undo.push(edit);
                    return Ok(true);
                }
                return Ok(false);
            }
            SoloCommand::Set { index, value } => {
                self.editable(index)?;
                if value > self.size {
                    return Err(SoloError::InvalidValue);
                }
                (index, Cell { value, notes: 0 })
            }
            SoloCommand::Erase { index } => {
                self.editable(index)?;
                (index, Cell { value: 0, notes: 0 })
            }
            SoloCommand::ToggleNote { index, value } => {
                self.editable(index)?;
                if value == 0 || value > self.size {
                    return Err(SoloError::InvalidValue);
                }
                let mut cell = self.cells[index as usize];
                if cell.value != 0 {
                    return Err(SoloError::FilledCell);
                }
                cell.notes ^= 1 << (value - 1);
                (index, cell)
            }
        };
        let before = self.cells[index as usize];
        if before == after {
            return Ok(false);
        }
        if self.undo.len() == MAX_HISTORY {
            self.undo.remove(0);
        }
        self.undo.push(Edit {
            index,
            before,
            after,
        });
        self.redo.clear();
        self.cells[index as usize] = after;
        Ok(true)
    }

    /// JSON command boundary shared by Swift, Kotlin and WASM adapters.
    pub fn apply_json(&mut self, command: &str) -> Result<bool, SoloError> {
        if command.len() > 1024 {
            return Err(SoloError::InvalidCommand);
        }
        let command = serde_json::from_str(command).map_err(|_| SoloError::InvalidCommand)?;
        self.apply(command)
    }

    /// Visible conflicts only; no solution information is consulted.
    pub fn conflicts(&self) -> Vec<u32> {
        crate::conflicting_cells(&self.values(), self.size).unwrap_or_default()
    }

    /// An advisory single-step explanation; this does not alter the saved game.
    pub fn hint(&self) -> Result<Option<crate::LogicalHint>, crate::PuzzleError> {
        crate::find_logical_hint(&self.values(), self.size)
    }

    pub fn hint_json(&self) -> Result<String, crate::PuzzleError> {
        serde_json::to_string(&self.hint()?).map_err(|_| crate::PuzzleError::InvalidBoard)
    }

    /// The next placement with the eliminations behind it; covers every step
    /// of a rated puzzle. JSON null when the supported techniques are stuck.
    pub fn technique_hint_json(&self) -> Result<String, crate::PuzzleError> {
        serde_json::to_string(&crate::find_technique_hint(&self.values(), self.size)?)
            .map_err(|_| crate::PuzzleError::InvalidBoard)
    }

    /// Rating of the puzzle as given, independent of the player's progress.
    /// It is derived from the givens, so saves need no extra field.
    pub fn difficulty_json(&self) -> Result<String, crate::PuzzleError> {
        serde_json::to_string(&crate::rate_puzzle(&self.givens, self.size)?)
            .map_err(|_| crate::PuzzleError::InvalidBoard)
    }

    fn editable(&self, index: u32) -> Result<(), SoloError> {
        match self.givens.get(index as usize) {
            None => Err(SoloError::InvalidCell),
            Some(0) => Ok(()),
            Some(_) => Err(SoloError::GivenCell),
        }
    }

    pub fn save(&self) -> Result<String, SoloError> {
        serde_json::to_string(self).map_err(|_| SoloError::InvalidSave)
    }

    /// Validates current cells and both history chains before returning a game.
    /// Callers replace their active game only after this returns Ok.
    pub fn load(json: &str) -> Result<Self, SoloError> {
        if json.len() > MAX_SAVE_BYTES {
            return Err(SoloError::InvalidSave);
        }
        #[derive(Deserialize)]
        struct Version {
            version: u64,
        }
        let version: Version = serde_json::from_str(json).map_err(|_| SoloError::InvalidSave)?;
        if version.version != VERSION as u64 {
            return Err(SoloError::UnsupportedVersion);
        }
        let saved: SavedGame = serde_json::from_str(json).map_err(|_| SoloError::InvalidSave)?;
        Self::try_from(saved)
    }

    fn valid_cell(&self, cell: Cell) -> bool {
        cell.value <= self.size
            && cell.notes < (1 << self.size)
            && (cell.value == 0 || cell.notes == 0)
    }

    fn check(&self) -> Result<(), SoloError> {
        if self.version != VERSION {
            return Err(SoloError::UnsupportedVersion);
        }
        if !crate::validate_board(&self.givens, self.size)
            || self.cells.len() != self.givens.len()
            || self.undo.len() + self.redo.len() > MAX_HISTORY
        {
            return Err(SoloError::InvalidSave);
        }
        for (cell, &given) in self.cells.iter().zip(&self.givens) {
            if !self.valid_cell(*cell) || (given != 0 && (cell.value != given || cell.notes != 0)) {
                return Err(SoloError::InvalidSave);
            }
        }
        for (history, backwards) in [(&self.undo, true), (&self.redo, false)] {
            let mut cells = self.cells.clone();
            for edit in history.iter().rev() {
                if self.editable(edit.index).is_err()
                    || !self.valid_cell(edit.before)
                    || !self.valid_cell(edit.after)
                    || edit.before == edit.after
                {
                    return Err(SoloError::InvalidSave);
                }
                let (expected, replacement) = if backwards {
                    (edit.after, edit.before)
                } else {
                    (edit.before, edit.after)
                };
                let cell = &mut cells[edit.index as usize];
                if *cell != expected {
                    return Err(SoloError::InvalidSave);
                }
                *cell = replacement;
            }
        }
        Ok(())
    }
}
