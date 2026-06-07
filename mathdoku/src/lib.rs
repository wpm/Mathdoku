//! Mathdoku puzzle generator and solver.
//!
//! ## Core types
//!
//! | Type | Role |
//! |------|------|
//! | [`Cell`] | A grid position identified by `(row, column)`, 1-indexed. |
//! | [`Fill`] | A bitmap set of candidate values `1..=9` for a cell. |
//! | [`Polyomino`] | A connected set of cells forming a cage shape. |
//! | [`Puzzle`] | An `n×n` cage structure with constraint propagation. |
//! | [`CageOperator`] | The arithmetic operator for a cage (`Add`, `Subtract`, etc.). |
//!
//! ## Entry points
//!
//! - **Generate** a random puzzle with [`generate()`].
//! - **Construct** a puzzle programmatically with [`Puzzle::new`] and [`Puzzle::insert`].
//! - **Inspect** cell values with [`Puzzle::get`].
//! - **Solve** with [`Puzzle::solutions`].
//! - **Query valid operators** for a polyomino with [`operators_for`].

#![deny(missing_docs)]
#![allow(dead_code)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::print_stderr
    )
)]

mod generate;
mod latin_square;
mod mdk;
pub(crate) mod solutions;

pub use generate::generate;
pub use latin_square::generate_latin_square;
pub use mdk::cage::{Cage, Operation};
pub use mdk::fill::Fill;
pub use mdk::polyomino::{Cell, Polyomino};
pub use mdk::puzzle::{CageOperator, Grid, Puzzle, operators_for};
pub use mdk::{Error, N, T};

/// Alias for [`CageOperator`], kept for backward compatibility.
pub type Operator = CageOperator;
/// Alias for [`T`], kept for backward compatibility.
pub type Target = T;
