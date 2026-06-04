//! New Mathdoku implementation — work in progress; will eventually replace the top-level crate API.

use crate::mdk::old_cage::Cage;
use shape::Cell;

pub(crate) mod domino_table;
pub(crate) mod fill;
mod grid;
pub(crate) mod mdd;
pub mod puzzle;
pub mod memo;
pub mod operator;
pub mod old_cage;
pub mod shape;
mod cage;
pub mod table;
pub mod operation;

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
