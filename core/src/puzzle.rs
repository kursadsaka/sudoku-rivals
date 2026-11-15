use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PuzzleError {
    InvalidBoard,
    InvalidBlankCount,
    UnsolvableBoard,
    SearchLimit,
    CannotReachBlankCount,
    CannotReachDifficulty,
}
impl fmt::Display for PuzzleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PuzzleError {}

// Bound work on arbitrary imported boards at native/WASM trust boundaries.
const MAX_SEARCH_NODES: u32 = 50_000;

/// Counts up to `limit` solutions; zero limit returns zero. Returns SearchLimit
/// when the node budget is exhausted; this is never interpreted as no solution.

pub fn count_solutions(cells: &[u8], size: u8, limit: u32) -> Result<u32, PuzzleError> {
    if !crate::validate_board(cells, size) {
        return Err(PuzzleError::InvalidBoard);
    }
    let mut budget = MAX_SEARCH_NODES;
    search(
        &mut cells.to_vec(),
        size as usize,
        limit,
        &mut None,
        &mut budget,
    )
}

/// Returns one solution, or None for a conflict-free but unsatisfiable board.
pub fn solve_board(cells: &[u8], size: u8) -> Result<Option<Vec<u8>>, PuzzleError> {
    if !crate::validate_board(cells, size) {
        return Err(PuzzleError::InvalidBoard);
    }
    let mut solution = None;
    let mut budget = MAX_SEARCH_NODES;
    search(
        &mut cells.to_vec(),
        size as usize,
        1,
        &mut solution,
        &mut budget,
    )?;
    Ok(solution)
}

fn search(
    cells: &mut [u8],
    n: usize,
    limit: u32,
    solution: &mut Option<Vec<u8>>,
    budget: &mut u32,
) -> Result<u32, PuzzleError> {
    if limit == 0 {
        return Ok(0);
    }
    if *budget == 0 {
        return Err(PuzzleError::SearchLimit);
    }
    *budget -= 1;
    let mut best = None;
    let mut candidates = Vec::new();
    for index in 0..cells.len() {
        if cells[index] != 0 {
            continue;
        }
        let mask = candidate_mask(cells, n, index);
        let available: Vec<u8> = (1..=n as u8)
            .filter(|v| mask & (1 << (v - 1)) != 0)
            .collect();
        if available.is_empty() {
            return Ok(0);
        }
        if best.is_none() || available.len() < candidates.len() {
            best = Some(index);
            candidates = available;
            if candidates.len() == 1 {
                break;
            }
        }
    }
    let Some(index) = best else {
        if solution.is_none() {
            *solution = Some(cells.to_vec());
        }
        return Ok(1);
    };
    let mut count = 0;
    for value in candidates {
        cells[index] = value;
        count += search(cells, n, limit - count, solution, budget)?;
        if count == limit {
            break;
        }
    }
    cells[index] = 0;
    Ok(count)
}

// Fixed integer arithmetic makes a seed reproducible across native and WASM.
// This is a puzzle permutation, never a source of session tokens or secrets.
pub(crate) fn shuffle<T>(items: &mut [T], state: &mut u32) {
    for index in (1..items.len()).rev() {
        *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        items.swap(index, *state as usize % (index + 1));
    }
}

/// A complete valid grid: one base pattern under digit, row, column, band and
/// stack permutations drawn from `state`. Shared by every generator so a seed
/// keeps meaning the same grid.
pub(crate) fn solved_grid(n: usize, state: &mut u32) -> Vec<u8> {
    let box_rows = if n == 6 { 2 } else { 3 };
    let mut digits: Vec<u8> = (1..=n as u8).collect();
    shuffle(&mut digits, state);
    let mut rows: Vec<usize> = (0..n).collect();
    for band in rows.chunks_mut(box_rows) {
        shuffle(band, state);
    }
    let mut bands: Vec<usize> = (0..n / box_rows).collect();
    shuffle(&mut bands, state);
    let rows: Vec<usize> = bands
        .iter()
        .flat_map(|&b| rows[b * box_rows..(b + 1) * box_rows].iter().copied())
        .collect();
    let mut cols: Vec<usize> = (0..n).collect();
    for stack in cols.chunks_mut(3) {
        shuffle(stack, state);
    }
    let mut stacks: Vec<usize> = (0..n / 3).collect();
    shuffle(&mut stacks, state);
    let cols: Vec<usize> = stacks
        .iter()
        .flat_map(|&s| cols[s * 3..(s + 1) * 3].iter().copied())
        .collect();
    rows.iter()
        .flat_map(|&r| {
            let digits = &digits;
            cols.iter()
                .map(move |&c| digits[(r * 3 + r / box_rows + c) % n])
        })
        .collect()
}

/// Generates a unique puzzle with an exact requested number of blanks.
/// Returns an error if this removal order cannot reach that count. Blank count
/// is not human difficulty; callers must not label it easy/hard from this alone.
/// Seeds vary permutations/removal order, not the full space of Sudoku grids.
pub fn generate_puzzle(size: u8, seed: u32, blanks: u8) -> Result<Vec<u8>, PuzzleError> {
    let n = match size {
        6 => 6,
        9 => 9,
        _ => return Err(PuzzleError::InvalidBoard),
    };
    if blanks as usize > n * n {
        return Err(PuzzleError::InvalidBlankCount);
    }
    let mut state = seed;
    let mut cells = solved_grid(n, &mut state);
    let mut order: Vec<usize> = (0..n * n).collect();
    shuffle(&mut order, &mut state);
    let mut removed = 0;
    for index in order {
        if removed == blanks {
            return Ok(cells);
        }
        let value = cells[index];
        cells[index] = 0;
        if count_solutions(&cells, size, 2)? == 1 {
            removed += 1;
        } else {
            cells[index] = value;
        }
    }
    if removed == blanks {
        Ok(cells)
    } else {
        Err(PuzzleError::CannotReachBlankCount)
    }
}

pub(crate) fn are_peers(a: usize, b: usize, n: usize) -> bool {
    let box_rows = if n == 6 { 2 } else { 3 };
    a / n == b / n
        || a % n == b % n
        || (a / n / box_rows == b / n / box_rows && a % n / 3 == b % n / 3)
}

pub(crate) fn candidate_mask(cells: &[u8], n: usize, index: usize) -> u16 {
    let mut mask = (1 << n) - 1;
    for (other, &value) in cells.iter().enumerate() {
        if value != 0 && are_peers(index, other, n) {
            mask &= !(1 << (value - 1));
        }
    }
    mask
}

/// Reports both ends of each visible row/column/box conflict. Invalid shape or
/// out-of-range digits return an error rather than misleading empty conflicts.
pub fn conflicting_cells(cells: &[u8], size: u8) -> Result<Vec<u32>, PuzzleError> {
    let n = match size {
        6 => 6,
        9 => 9,
        _ => return Err(PuzzleError::InvalidBoard),
    };
    if cells.len() != n * n || cells.iter().any(|&v| v > size) {
        return Err(PuzzleError::InvalidBoard);
    }
    let mut conflicts = vec![false; cells.len()];
    for a in 0..cells.len() {
        if cells[a] == 0 {
            continue;
        }
        for b in a + 1..cells.len() {
            if cells[a] == cells[b] && are_peers(a, b, n) {
                conflicts[a] = true;
                conflicts[b] = true;
            }
        }
    }
    Ok(conflicts
        .iter()
        .enumerate()
        .filter_map(|(i, &conflict)| conflict.then_some(i as u32))
        .collect())
}
