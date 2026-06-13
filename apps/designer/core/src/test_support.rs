//! Shared fixtures for Designer tests.
//!
//! Compiled into this crate's own unit tests, and exported to the UI and
//! Tauri crates' tests behind the `test-support` feature. Not part of the
//! crate's stable API.

#![allow(
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::must_use_candidate
)]

use crate::{AppState, apply_loaded, serialize_save};
#[cfg(feature = "without-solution")]
use crate::{State, insert_cage};
use mathdoku::{Cage, CageOperator, Cell, N, Polyomino, Puzzle, Target};

/// The canonical 3×3 Latin square used by the fixtures below:
/// ```text
/// 1 2 3
/// 2 3 1
/// 3 1 2
/// ```
const LATIN_3X3: [[N; 3]; 3] = [[1, 2, 3], [2, 3, 1], [3, 1, 2]];

/// Builds [`Cell`]s from 0-indexed `(row, column)` positions.
pub fn cells(positions: &[(usize, usize)]) -> Vec<Cell> {
    positions.iter().map(|&(r, c)| Cell::new(r, c)).collect()
}

/// Builds a [`Polyomino`] from 0-indexed `(row, column)` positions.
pub fn poly(positions: &[(usize, usize)]) -> Polyomino {
    Polyomino::from_cells(&cells(positions)).unwrap()
}

/// Builds a [`Cage`] from `(row, column)` positions, an operator, and a target.
pub fn cage_at(n: N, positions: &[(usize, usize)], op: CageOperator, target: u64) -> Cage {
    let target = Target::try_from(target).unwrap();
    Cage::new(n, poly(positions), op, target).unwrap()
}

/// Every cell of an `n`×`n` grid in row-major order.
pub fn all_cells(n: usize) -> Vec<Cell> {
    (0..n)
        .flat_map(|r| (0..n).map(move |c| Cell::new(r, c)))
        .collect()
}

/// A 3×3 [`Puzzle`] pinned to [`LATIN_3X3`] by nine `Given` cages — exactly
/// one solution.
pub fn given_3x3() -> Puzzle {
    let mut puzzle = Puzzle::new(3).unwrap();
    for (r, row) in LATIN_3X3.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            puzzle = puzzle
                .insert_cage(&cage_at(3, &[(r, c)], CageOperator::Given, u64::from(v)))
                .unwrap()
                .unwrap();
        }
    }
    puzzle
}

/// The `Add`-6 cage covering row `r` of a 3×3 grid. Each row is then forced to
/// be a permutation of `{1, 2, 3}`.
pub fn row_sum_cage(r: usize) -> Cage {
    cage_at(3, &[(r, 0), (r, 1), (r, 2)], CageOperator::Add, 6)
}

/// A 3×3 [`Puzzle`] covered by three [`row_sum_cage`]s. Every row is forced to
/// be a permutation of `{1, 2, 3}`, so the solutions are exactly the 12 order-3
/// Latin squares.
pub fn row_sums_3x3() -> Puzzle {
    let mut puzzle = Puzzle::new(3).unwrap();
    for r in 0..3 {
        puzzle = puzzle.insert_cage(&row_sum_cage(r)).unwrap().unwrap();
    }
    puzzle
}

/// A Without-Solution [`State`] whose puzzle is [`given_3x3`] — exactly one
/// completion.
#[cfg(feature = "without-solution")]
pub fn unique_3x3() -> State {
    let mut st = State::new(3).unwrap();
    st.puzzle = given_3x3();
    st
}

/// [`LATIN_3X3`] as a fully solved [`Puzzle`], as used by the
/// target-derivation tests.
pub fn known_3x3_solution() -> Puzzle {
    let square: Vec<Vec<N>> = LATIN_3X3.iter().map(|row| row.to_vec()).collect();
    Puzzle::from_latin_square(3, &square).unwrap()
}

/// A With-Solution [`AppState`] whose solution is [`known_3x3_solution`]
/// and whose puzzle has no cages yet.
pub fn with_solution_3x3() -> AppState {
    let solution = known_3x3_solution();
    AppState {
        puzzle: Some(Puzzle::new(3).unwrap()),
        solution: Some(solution),
        ..AppState::default()
    }
}

/// A fresh Without-Solution `n`×`n` [`AppState`].
///
/// Built directly rather than through the feature-gated `new_empty` command
/// so it is usable in both build configurations (solution-less states remain
/// loadable from save files even when the `without-solution` feature is off).
pub fn without_solution(n: usize) -> AppState {
    AppState {
        puzzle: Some(Puzzle::new(n).unwrap()),
        dirty: true,
        ..AppState::default()
    }
}

/// A 3×3 [`AppState`] pinned to [`LATIN_3X3`] by nine `Given` cages inserted
/// through the [`insert_cage`] command — exactly one completion.
#[cfg(feature = "without-solution")]
pub fn unique_3x3_app_state() -> AppState {
    let mut state = without_solution(3);
    for (r, row) in LATIN_3X3.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            let _ = insert_cage(
                &mut state,
                poly(&[(r, c)]),
                CageOperator::Given,
                Some(Target::from(v)),
            )
            .unwrap();
        }
    }
    state
}

/// Serializes `source`, loads it into a fresh state, and returns the loaded state.
pub fn save_round_trip(source: &AppState) -> AppState {
    let json = serialize_save(source).unwrap();
    let mut loaded = AppState::default();
    let _ = apply_loaded(&mut loaded, &json).unwrap();
    loaded
}
