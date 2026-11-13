use crate::{puzzle::candidate_mask, PuzzleError};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HintTechnique {
    NakedSingle,
    HiddenSingle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitKind {
    Row,
    Column,
    Box,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HintUnit {
    pub kind: UnitKind,
    pub index: u8,
}

/// An advisory logical step. Indices are zero-based. It does not reveal a
/// searched answer or change/lock a cell. A caller may apply it as a normal set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogicalHint {
    pub index: u32,
    pub value: u8,
    pub technique: HintTechnique,
    pub unit: Option<HintUnit>,
}

/// Finds a naked or hidden single using only visible digits. Returns None
/// when these two techniques cannot progress, including an already full board.
/// Invalid/unsatisfiable player boards produce an error, never a guessed hint.
pub fn find_logical_hint(cells: &[u8], size: u8) -> Result<Option<LogicalHint>, PuzzleError> {
    if crate::solve_board(cells, size)?.is_none() {
        return Err(PuzzleError::UnsolvableBoard);
    }
    Ok(single_step(cells, size))
}

fn single_step(cells: &[u8], size: u8) -> Option<LogicalHint> {
    let n = size as usize;
    let masks: Vec<u16> = (0..cells.len())
        .map(|i| {
            if cells[i] == 0 {
                candidate_mask(cells, n, i)
            } else {
                0
            }
        })
        .collect();
    for (index, &mask) in masks.iter().enumerate() {
        if mask.count_ones() == 1 {
            return Some(LogicalHint {
                index: index as u32,
                value: mask.trailing_zeros() as u8 + 1,
                technique: HintTechnique::NakedSingle,
                unit: None,
            });
        }
    }
    let box_rows = if size == 6 { 2 } else { 3 };
    for kind in [UnitKind::Row, UnitKind::Column, UnitKind::Box] {
        for unit in 0..n {
            for value in 1..=size {
                let indices: Vec<usize> = (0..cells.len())
                    .filter(|&i| {
                        let matches = match kind {
                            UnitKind::Row => i / n == unit,
                            UnitKind::Column => i % n == unit,
                            UnitKind::Box => (i / n / box_rows) * (n / 3) + (i % n / 3) == unit,
                        };
                        matches && masks[i] & (1 << (value - 1)) != 0
                    })
                    .collect();
                if indices.len() == 1 {
                    return Some(LogicalHint {
                        index: indices[0] as u32,
                        value,
                        technique: HintTechnique::HiddenSingle,
                        unit: Some(HintUnit {
                            kind,
                            index: unit as u8,
                        }),
                    });
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TechniqueAnalysis {
    pub solved_with_singles: bool,
    pub naked_single_steps: u32,
    pub hidden_single_steps: u32,
    pub remaining_cells: u32,
}

/// Reports the actual supported technique steps; it is not a calibrated human
/// difficulty rating. Boards needing other techniques/search are left unsolved.
pub fn analyze_techniques(cells: &[u8], size: u8) -> Result<TechniqueAnalysis, PuzzleError> {
    if crate::solve_board(cells, size)?.is_none() {
        return Err(PuzzleError::UnsolvableBoard);
    }
    let mut cells = cells.to_vec();
    let mut analysis = TechniqueAnalysis {
        solved_with_singles: false,
        naked_single_steps: 0,
        hidden_single_steps: 0,
        remaining_cells: 0,
    };
    while let Some(hint) = single_step(&cells, size) {
        cells[hint.index as usize] = hint.value;
        match hint.technique {
            HintTechnique::NakedSingle => analysis.naked_single_steps += 1,
            HintTechnique::HiddenSingle => analysis.hidden_single_steps += 1,
        }
    }
    analysis.remaining_cells = cells.iter().filter(|&&v| v == 0).count() as u32;
    analysis.solved_with_singles = analysis.remaining_cells == 0;
    Ok(analysis)
}
