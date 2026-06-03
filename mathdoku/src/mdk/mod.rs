//! New Mathdoku implementation — work in progress; will eventually replace the top-level crate API.

use crate::mdk::cage::Cage;
use crate::mdk::grid::Cell;

pub mod cage;
pub(crate) mod domino_table;
pub(crate) mod fill;
mod grid;
pub(crate) mod mdd;
pub mod puzzle;
pub mod memo;
pub mod operator;

type N = u32;
type Target = u32;

/// Errors returned by mdk operations.
#[derive(Debug)]
pub enum Error {
    /// Invalid [`Grid`] size
    InvalidGridSize(usize),
    /// The [`Cell`]s do not form a polyomino
    InvalidPolyomino(Vec<Cell>),
    /// The [`Cell`] is missing from the specified [`Polyomino`] or [`Grid`].
    MissingCell(Cell),
    /// Specified [`Cage`] is not in the [`Puzzle`].
    MissingCage(Cage),
}
