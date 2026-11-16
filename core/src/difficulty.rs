//! Technique-based difficulty: a logical solver that only uses named human
//! techniques, a rating derived from the hardest technique a puzzle needs, a
//! generator that digs a puzzle for a requested level, and hints that explain
//! every step of such a puzzle.
//!
//! A level is a statement about techniques, not about blank counts. Each of
//! the five needs one technique more than the level below it:
//!
//! * `Beginner` – every step is found by looking at one box (the digit has one
//!   place left in the box) or is the last empty cell of a row, column or box.
//! * `Easy`     – boxes are not enough; some digit must be placed because it
//!   has one place left in a row or a column.
//! * `Medium`   – looking for a digit's one place is not enough; at least one
//!   cell must be solved by counting its candidates (naked single).
//! * `Hard`     – singles run out; candidates must first be removed with
//!   locked candidates (pointing or claiming).
//! * `Expert`   – locked candidates are not enough either; a naked or hidden
//!   pair must remove candidates.
//!
//! A puzzle the logical solver finishes has exactly one solution, because
//! every step it takes is forced. Puzzles needing anything beyond these
//! techniques are left unrated rather than guessed at.

use crate::hint::{HintTechnique, HintUnit, UnitKind};
use crate::puzzle::shuffle;
use crate::PuzzleError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Beginner,
    Easy,
    Medium,
    Hard,
    Expert,
}
impl Difficulty {
    pub const ALL: [Difficulty; 5] = [
        Difficulty::Beginner,
        Difficulty::Easy,
        Difficulty::Medium,
        Difficulty::Hard,
        Difficulty::Expert,
    ];

    /// Stable lowercase name used at every platform boundary.
    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Beginner => "beginner",
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
            Difficulty::Expert => "expert",
        }
    }
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.name() == name)
    }
    /// The hardest technique a puzzle of this level may need.
    fn ceiling(self) -> Technique {
        match self {
            Difficulty::Beginner => Technique::BoxSingle,
            Difficulty::Easy => Technique::LineSingle,
            Difficulty::Medium => Technique::NakedSingle,
            Difficulty::Hard => Technique::LockedCandidates,
            Difficulty::Expert => Technique::HiddenPair,
        }
    }
    /// The ceiling of the level below, if there is one.
    fn floor(self) -> Option<Technique> {
        let below = Self::ALL
            .iter()
            .position(|&level| level == self)?
            .checked_sub(1)?;
        Some(Self::ALL[below].ceiling())
    }
    /// Blank counts per level and size: how deep the puzzle is dug before the
    /// level's own technique is asked for, and the most blanks it may have.
    /// They keep a level's puzzles a similar length; they never decide the
    /// level, the techniques do.
    fn blanks(self, n: usize) -> (usize, usize) {
        match (n, self) {
            (6, Difficulty::Beginner) => (17, 17),
            (6, Difficulty::Easy) => (18, 26),
            // On 6×6 anything beyond hidden singles only shows up in sparse
            // puzzles; the generator digs those as deep as the level allows
            // instead of using a window.
            (6, _) => (36, 36),
            (_, Difficulty::Beginner) => (38, 38),
            (_, Difficulty::Easy) => (40, 50),
            (_, Difficulty::Medium) => (44, 54),
            (_, Difficulty::Hard) => (47, 58),
            (_, Difficulty::Expert) => (50, 60),
        }
    }
}

/// Ordered from easiest to hardest to spot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Technique {
    /// A digit with one place left in a box, or the last empty cell of a unit.
    BoxSingle,
    /// A digit with one place left in a row or a column.
    LineSingle,
    NakedSingle,
    LockedCandidates,
    NakedPair,
    HiddenPair,
}

/// What solving the puzzle with the supported techniques takes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DifficultyRating {
    /// None: the supported techniques cannot finish this board.
    pub level: Option<Difficulty>,
    /// Hardest technique used on the easiest-first solving path.
    pub hardest: Option<Technique>,
    pub blanks: u32,
    /// Box and line singles together.
    pub hidden_singles: u32,
    /// Of those, the ones only a row or a column shows.
    pub line_singles: u32,
    pub naked_singles: u32,
    pub locked_candidates: u32,
    pub naked_pairs: u32,
    pub hidden_pairs: u32,
    /// Cells the supported techniques could not fill (zero when rated).
    pub remaining_cells: u32,
}

/// One candidate elimination that had to happen before the hinted placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HintElimination {
    pub technique: Technique,
    /// One digit for locked candidates, two for a pair.
    pub digits: Vec<u8>,
    /// Where the pattern is seen.
    pub unit: HintUnit,
    /// Locked candidates only: the crossing unit the digit is removed from.
    pub target: Option<HintUnit>,
    /// Zero-based cells that form the pattern.
    pub cells: Vec<u32>,
}

/// A placement the player can make now, with the eliminations that justify it
/// when singles alone do not. Indices are zero-based; nothing is revealed from
/// a searched solution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TechniqueHint {
    pub index: u32,
    pub value: u8,
    pub technique: HintTechnique,
    pub unit: Option<HintUnit>,
    pub eliminations: Vec<HintElimination>,
}

struct Geometry {
    n: usize,
    /// Rows `0..n`, columns `n..2n`, boxes `2n..3n`.
    units: Vec<Vec<usize>>,
    /// Row, column and box unit of each cell.
    home: Vec<[usize; 3]>,
    peers: Vec<Vec<usize>>,
}
impl Geometry {
    fn new(n: usize) -> Self {
        let box_rows = if n == 6 { 2 } else { 3 };
        let mut units = vec![Vec::with_capacity(n); 3 * n];
        let mut home = Vec::with_capacity(n * n);
        for index in 0..n * n {
            let (row, column) = (index / n, index % n);
            let cell = [
                row,
                n + column,
                2 * n + (row / box_rows) * (n / 3) + column / 3,
            ];
            for unit in cell {
                units[unit].push(index);
            }
            home.push(cell);
        }
        let peers = (0..n * n)
            .map(|index| {
                (0..n * n)
                    .filter(|&other| other != index && crate::puzzle::are_peers(index, other, n))
                    .collect()
            })
            .collect();
        Self {
            n,
            units,
            home,
            peers,
        }
    }
    fn hint_unit(&self, unit: usize) -> HintUnit {
        HintUnit {
            kind: [UnitKind::Row, UnitKind::Column, UnitKind::Box][unit / self.n],
            index: (unit % self.n) as u8,
        }
    }
    fn contains(&self, unit: usize, index: usize) -> bool {
        self.home[index][unit / self.n] == unit
    }
}

/// A (cell, digit-bit) candidate.
type Candidate = (usize, u16);

struct Elimination {
    technique: Technique,
    digits: u16,
    unit: usize,
    target: Option<usize>,
    cells: Vec<usize>,
    removed: Vec<Candidate>,
    /// Candidates that must already be gone for the pattern to hold.
    requires: Vec<Candidate>,
}

struct Grid<'g> {
    geo: &'g Geometry,
    cells: Vec<u8>,
    candidates: Vec<u16>,
}
impl<'g> Grid<'g> {
    fn new(geo: &'g Geometry, cells: &[u8]) -> Self {
        let all = (1u16 << geo.n) - 1;
        let candidates = (0..cells.len())
            .map(|index| {
                if cells[index] != 0 {
                    return 0;
                }
                geo.peers[index].iter().fold(all, |mask, &peer| {
                    if cells[peer] == 0 {
                        mask
                    } else {
                        mask & !(1 << (cells[peer] - 1))
                    }
                })
            })
            .collect();
        Self {
            geo,
            cells: cells.to_vec(),
            candidates,
        }
    }
    fn place(&mut self, index: usize, value: u8) {
        self.cells[index] = value;
        self.candidates[index] = 0;
        for &peer in &self.geo.peers[index] {
            self.candidates[peer] &= !(1 << (value - 1));
        }
    }
    fn remaining(&self) -> u32 {
        self.cells.iter().filter(|&&value| value == 0).count() as u32
    }

    fn hidden_single_in(&self, units: std::ops::Range<usize>) -> Option<(usize, u8, usize)> {
        for unit in units {
            for value in 1..=self.geo.n as u8 {
                let bit = 1 << (value - 1);
                let mut places = self.geo.units[unit]
                    .iter()
                    .filter(|&&index| self.candidates[index] & bit != 0);
                if let (Some(&index), None) = (places.next(), places.next()) {
                    return Some((index, value, unit));
                }
            }
        }
        None
    }
    /// What a look at one box shows, plus the last empty cell of any unit:
    /// the first things a player learns to see.
    fn box_single(&self) -> Option<(usize, u8, usize)> {
        let n = self.geo.n;
        self.hidden_single_in(2 * n..3 * n).or_else(|| {
            (0..2 * n).find_map(|unit| {
                let mut empty = self.geo.units[unit]
                    .iter()
                    .filter(|&&index| self.cells[index] == 0);
                let (Some(&index), None) = (empty.next(), empty.next()) else {
                    return None;
                };
                let mask = self.candidates[index];
                (mask.count_ones() == 1).then(|| (index, mask.trailing_zeros() as u8 + 1, unit))
            })
        })
    }
    /// A digit with one place left in a row or a column.
    fn line_single(&self) -> Option<(usize, u8, usize)> {
        self.hidden_single_in(0..2 * self.geo.n)
    }
    fn naked_single(&self) -> Option<(usize, u8)> {
        self.candidates
            .iter()
            .position(|mask| mask.count_ones() == 1)
            .map(|index| (index, self.candidates[index].trailing_zeros() as u8 + 1))
    }

    fn remove(&mut self, elimination: &Elimination) {
        for &(index, bit) in &elimination.removed {
            self.candidates[index] &= !bit;
        }
    }

    /// Pointing (a box confines a digit to one line) and claiming (a line
    /// confines a digit to one box): the digit leaves the rest of the other unit.
    fn locked_candidates(&self) -> Option<Elimination> {
        let n = self.geo.n;
        for unit in (2 * n..3 * n).chain(0..2 * n) {
            let is_box = unit >= 2 * n;
            for value in 1..=n as u8 {
                let bit = 1u16 << (value - 1);
                let places: Vec<usize> = self.geo.units[unit]
                    .iter()
                    .copied()
                    .filter(|&index| self.candidates[index] & bit != 0)
                    .collect();
                if places.len() < 2 {
                    continue;
                }
                let crossing: &[usize] = if is_box { &[0, 1] } else { &[2] };
                for &kind in crossing {
                    let target = self.geo.home[places[0]][kind];
                    if !places.iter().all(|&index| self.geo.contains(target, index)) {
                        continue;
                    }
                    let removed: Vec<Candidate> = self.geo.units[target]
                        .iter()
                        .filter(|&&index| {
                            !self.geo.contains(unit, index) && self.candidates[index] & bit != 0
                        })
                        .map(|&index| (index, bit))
                        .collect();
                    if removed.is_empty() {
                        continue;
                    }
                    let requires = self.geo.units[unit]
                        .iter()
                        .filter(|&&index| !self.geo.contains(target, index))
                        .map(|&index| (index, bit))
                        .collect();
                    return Some(Elimination {
                        technique: Technique::LockedCandidates,
                        digits: bit,
                        unit,
                        target: Some(target),
                        cells: places,
                        removed,
                        requires,
                    });
                }
            }
        }
        None
    }

    /// Two cells of a unit that can only hold the same two digits: those
    /// digits leave every other cell of the unit.
    fn naked_pair(&self) -> Option<Elimination> {
        let all = (1u16 << self.geo.n) - 1;
        for (unit, cells) in self.geo.units.iter().enumerate() {
            for (position, &first) in cells.iter().enumerate() {
                let pair = self.candidates[first];
                if pair.count_ones() != 2 {
                    continue;
                }
                for &second in &cells[position + 1..] {
                    if self.candidates[second] != pair {
                        continue;
                    }
                    let removed: Vec<Candidate> = cells
                        .iter()
                        .filter(|&&index| index != first && index != second)
                        .filter(|&&index| self.candidates[index] & pair != 0)
                        .map(|&index| (index, self.candidates[index] & pair))
                        .collect();
                    if removed.is_empty() {
                        continue;
                    }
                    return Some(Elimination {
                        technique: Technique::NakedPair,
                        digits: pair,
                        unit,
                        target: None,
                        cells: vec![first, second],
                        removed,
                        requires: vec![(first, all & !pair), (second, all & !pair)],
                    });
                }
            }
        }
        None
    }

    /// Two digits of a unit that fit only the same two cells: every other
    /// candidate leaves those cells.
    fn hidden_pair(&self) -> Option<Elimination> {
        let n = self.geo.n;
        for (unit, cells) in self.geo.units.iter().enumerate() {
            let places = |bit: u16| -> Vec<usize> {
                cells
                    .iter()
                    .copied()
                    .filter(|&index| self.candidates[index] & bit != 0)
                    .collect()
            };
            for low in 0..n {
                let first = places(1 << low);
                if first.len() != 2 {
                    continue;
                }
                for high in low + 1..n {
                    if places(1 << high) != first {
                        continue;
                    }
                    let pair = 1u16 << low | 1 << high;
                    let removed: Vec<Candidate> = first
                        .iter()
                        .filter(|&&index| self.candidates[index] & !pair != 0)
                        .map(|&index| (index, self.candidates[index] & !pair))
                        .collect();
                    if removed.is_empty() {
                        continue;
                    }
                    let requires = cells
                        .iter()
                        .filter(|index| !first.contains(index))
                        .map(|&index| (index, pair))
                        .collect();
                    return Some(Elimination {
                        technique: Technique::HiddenPair,
                        digits: pair,
                        unit,
                        target: None,
                        cells: first,
                        removed,
                        requires,
                    });
                }
            }
        }
        None
    }

    /// The easiest elimination available within `ceiling`.
    fn eliminate(&self, ceiling: Technique) -> Option<Elimination> {
        if ceiling < Technique::LockedCandidates {
            return None;
        }
        self.locked_candidates()
            .or_else(|| (ceiling >= Technique::NakedPair).then(|| self.naked_pair())?)
            .or_else(|| (ceiling >= Technique::HiddenPair).then(|| self.hidden_pair())?)
    }
}

#[derive(Default)]
struct Path {
    used: [u32; 6],
    remaining: u32,
}

/// Solves as far as techniques up to `ceiling` reach, always taking the
/// easiest available step.
fn solve(grid: &mut Grid, ceiling: Technique) -> Path {
    let mut path = Path::default();
    loop {
        if let Some((index, value, _)) = grid.box_single() {
            grid.place(index, value);
            path.used[Technique::BoxSingle as usize] += 1;
        } else if let Some((index, value, _)) = grid
            .line_single()
            .filter(|_| ceiling >= Technique::LineSingle)
        {
            grid.place(index, value);
            path.used[Technique::LineSingle as usize] += 1;
        } else if let Some((index, value)) = grid
            .naked_single()
            .filter(|_| ceiling >= Technique::NakedSingle)
        {
            grid.place(index, value);
            path.used[Technique::NakedSingle as usize] += 1;
        } else if let Some(elimination) = grid.eliminate(ceiling) {
            grid.remove(&elimination);
            path.used[elimination.technique as usize] += 1;
        } else {
            break;
        }
    }
    path.remaining = grid.remaining();
    path
}

fn board_size(cells: &[u8], size: u8) -> Result<usize, PuzzleError> {
    if !matches!(size, 6 | 9) || !crate::validate_board(cells, size) {
        return Err(PuzzleError::InvalidBoard);
    }
    Ok(size as usize)
}

fn level_of(geo: &Geometry, cells: &[u8]) -> (Option<Difficulty>, Path) {
    let mut last = Path::default();
    for level in Difficulty::ALL {
        last = solve(&mut Grid::new(geo, cells), level.ceiling());
        if last.remaining == 0 {
            return (Some(level), last);
        }
    }
    (None, last)
}

/// Rates a board by the techniques it needs. The level is the lowest one
/// whose techniques finish the board; `None` means they cannot.
pub fn rate_puzzle(cells: &[u8], size: u8) -> Result<DifficultyRating, PuzzleError> {
    let geo = Geometry::new(board_size(cells, size)?);
    let (level, path) = level_of(&geo, cells);
    const ORDER: [Technique; 6] = [
        Technique::BoxSingle,
        Technique::LineSingle,
        Technique::NakedSingle,
        Technique::LockedCandidates,
        Technique::NakedPair,
        Technique::HiddenPair,
    ];
    Ok(DifficultyRating {
        level,
        hardest: ORDER
            .into_iter()
            .rev()
            .find(|&technique| path.used[technique as usize] > 0),
        blanks: cells.iter().filter(|&&value| value == 0).count() as u32,
        hidden_singles: path.used[0] + path.used[1],
        line_singles: path.used[1],
        naked_singles: path.used[2],
        locked_candidates: path.used[3],
        naked_pairs: path.used[4],
        hidden_pairs: path.used[5],
        remaining_cells: path.remaining,
    })
}

/// A complete grid filled by a seeded search: the most constrained cell gets
/// its candidates in shuffled order. Unlike the permuted base pattern this
/// reaches arbitrary grids, which is where the rarer hard 6×6 puzzles live.
fn random_grid(geo: &Geometry, state: &mut u32) -> Vec<u8> {
    fn fill(grid: &mut Grid, state: &mut u32) -> bool {
        let Some(index) = (0..grid.cells.len())
            .filter(|&index| grid.cells[index] == 0)
            .min_by_key(|&index| grid.candidates[index].count_ones())
        else {
            return true;
        };
        let mut values: Vec<u8> = (1..=grid.geo.n as u8)
            .filter(|value| grid.candidates[index] & (1 << (value - 1)) != 0)
            .collect();
        shuffle(&mut values, state);
        for value in values {
            let before = (grid.cells.clone(), grid.candidates.clone());
            grid.place(index, value);
            if fill(grid, state) {
                return true;
            }
            (grid.cells, grid.candidates) = before;
        }
        false
    }
    let mut grid = Grid::new(geo, &vec![0; geo.n * geo.n]);
    fill(&mut grid, state);
    grid.cells
}

// Attempts per call. A seed that misses the level is retried with a derived
// seed. 9×9 hits within a handful of attempts; the upper 6×6 levels are rare
// (about one dig in 70 needs a naked single, one in 470 needs an elimination),
// so they get a bound that makes running out practically impossible. Measured
// figures are in tests/difficulty.rs (`calibration_report`).
fn attempts(n: usize) -> u32 {
    if n == 6 {
        12_000
    } else {
        400
    }
}

/// The seed of one attempt. Seed and attempt are mixed, not added: a caller
/// that derives nearby seeds (the match server takes one per duel board) must
/// never walk into another seed's attempts and get the same puzzle twice.
pub(crate) fn attempt_seed(seed: u32, attempt: u32) -> u32 {
    fn mix(mut x: u32) -> u32 {
        x ^= x >> 16;
        x = x.wrapping_mul(0x7FEB_352D);
        x ^= x >> 15;
        x = x.wrapping_mul(0x846C_A68B);
        x ^ (x >> 16)
    }
    mix(seed ^ mix(attempt ^ 0x9E37_79B9))
}

/// Generates a puzzle whose rating is exactly `difficulty`.
///
/// Cells are removed from a complete grid only while the level's techniques
/// still finish the board, so the result is solvable without guessing and has
/// one solution. The same seed gives the same puzzle on every platform.
pub fn generate_rated_puzzle(
    size: u8,
    seed: u32,
    difficulty: Difficulty,
) -> Result<Vec<u8>, PuzzleError> {
    if size == 6 && difficulty == Difficulty::Expert {
        return Ok(crate::expert6::puzzle(seed));
    }
    search_rated_puzzle(size, seed, difficulty)
}

/// The search behind `generate_rated_puzzle`, with no shortcut for the level
/// that is too rare to search for on demand. Used to build and to check the
/// 6×6 Expert bank.
#[doc(hidden)]
pub fn search_rated_puzzle(
    size: u8,
    seed: u32,
    difficulty: Difficulty,
) -> Result<Vec<u8>, PuzzleError> {
    let n = match size {
        6 => 6,
        9 => 9,
        _ => return Err(PuzzleError::InvalidBoard),
    };
    let geo = Geometry::new(n);
    let (depth, most) = difficulty.blanks(n);
    let ceiling = difficulty.ceiling();
    let floor = difficulty.floor().unwrap_or(ceiling);
    let solvable =
        |cells: &[u8], ceiling| solve(&mut Grid::new(&geo, cells), ceiling).remaining == 0;
    'attempt: for attempt in 0..attempts(n) {
        let mut state = attempt_seed(seed, attempt);
        let mut cells = random_grid(&geo, &mut state);
        let mut order: Vec<usize> = (0..n * n).collect();
        shuffle(&mut order, &mut state);
        if n == 6 && difficulty > Difficulty::Easy {
            // Sparse 6×6: dig as deep as this level's techniques allow and keep
            // the puzzle only if the level below can no longer solve it.
            for &index in &order {
                let value = std::mem::take(&mut cells[index]);
                if !solvable(&cells, ceiling) {
                    cells[index] = value;
                }
            }
            if !solvable(&cells, floor) {
                return Ok(cells);
            }
            continue;
        }
        // 1. Dig to the level's depth with the techniques of the level below.
        let mut removed = 0;
        for &index in &order {
            if removed == depth {
                break;
            }
            let value = std::mem::take(&mut cells[index]);
            if solvable(&cells, floor) {
                removed += 1;
            } else {
                cells[index] = value;
            }
        }
        if difficulty == Difficulty::Beginner {
            if removed == depth {
                return Ok(cells);
            }
            continue;
        }
        // 2. Find the removal that the level below cannot follow but this
        //    level can: it is what makes the puzzle need this level's
        //    technique. While there is none, dig one cell deeper and look again.
        loop {
            if removed >= most {
                continue 'attempt;
            }
            let mut deeper = None;
            let mut crossed = false;
            for &index in &order {
                let value = cells[index];
                if value == 0 {
                    continue;
                }
                cells[index] = 0;
                if solvable(&cells, floor) {
                    deeper.get_or_insert(index);
                } else if solvable(&cells, ceiling) {
                    crossed = true;
                    break;
                }
                cells[index] = value;
            }
            removed += 1;
            if crossed {
                break;
            }
            match deeper {
                Some(index) => cells[index] = 0,
                None => continue 'attempt,
            }
        }
        debug_assert_eq!(level_of(&geo, &cells).0, Some(difficulty));
        return Ok(cells);
    }
    Err(PuzzleError::CannotReachDifficulty)
}

/// Finds the next placement using the supported techniques on the visible
/// digits. When no single is available it applies eliminations until one
/// appears and reports only the eliminations that placement depends on.
/// Returns None when the techniques cannot progress; invalid or unsatisfiable
/// boards are an error, never a guess.
pub fn find_technique_hint(cells: &[u8], size: u8) -> Result<Option<TechniqueHint>, PuzzleError> {
    let geo = Geometry::new(board_size(cells, size)?);
    if crate::solve_board(cells, size)?.is_none() {
        return Err(PuzzleError::UnsolvableBoard);
    }
    let mut grid = Grid::new(&geo, cells);
    let mut applied: Vec<Elimination> = Vec::new();
    loop {
        let all = (1u16 << geo.n) - 1;
        // What had to be eliminated for the placement to be forced.
        let found =
            if let Some((index, value, unit)) = grid.box_single().or_else(|| grid.line_single()) {
                let bit = 1u16 << (value - 1);
                let needs: Vec<Candidate> = geo.units[unit]
                    .iter()
                    .filter(|&&other| other != index)
                    .map(|&other| (other, bit))
                    .collect();
                Some((index, value, HintTechnique::HiddenSingle, Some(unit), needs))
            } else {
                grid.naked_single().map(|(index, value)| {
                    let needs = vec![(index, all & !(1 << (value - 1)))];
                    (index, value, HintTechnique::NakedSingle, None, needs)
                })
            };
        if let Some((index, value, technique, unit, mut needs)) = found {
            // Walk back: keep an elimination only if the placement, or a kept
            // later elimination, depends on a candidate it removed.
            let mut kept = Vec::new();
            for elimination in applied.iter().rev() {
                let relevant = elimination.removed.iter().any(|&(cell, bits)| {
                    needs
                        .iter()
                        .any(|&(needed, mask)| needed == cell && mask & bits != 0)
                });
                if relevant {
                    needs.extend(elimination.requires.iter().copied());
                    kept.push(elimination);
                }
            }
            let digits = |mask: u16| -> Vec<u8> {
                (1..=geo.n as u8)
                    .filter(|value| mask & (1 << (value - 1)) != 0)
                    .collect()
            };
            return Ok(Some(TechniqueHint {
                index: index as u32,
                value,
                technique,
                unit: unit.map(|unit| geo.hint_unit(unit)),
                eliminations: kept
                    .into_iter()
                    .rev()
                    .map(|elimination| HintElimination {
                        technique: elimination.technique,
                        digits: digits(elimination.digits),
                        unit: geo.hint_unit(elimination.unit),
                        target: elimination.target.map(|unit| geo.hint_unit(unit)),
                        cells: elimination.cells.iter().map(|&cell| cell as u32).collect(),
                    })
                    .collect(),
            }));
        }
        match grid.eliminate(Technique::HiddenPair) {
            Some(elimination) => {
                grid.remove(&elimination);
                applied.push(elimination);
            }
            None => return Ok(None),
        }
    }
}
