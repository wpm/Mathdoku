//! New Mathdoku implementation — work in progress; will eventually replace the top-level crate API.

use crate::mdk::old_cage::Cage;
use shape::Cell;

mod cage;
pub(crate) mod fill;
mod grid;
pub mod mdd;
pub mod memo;
pub mod old_cage;
pub(crate) mod old_mdd;
pub mod old_memo;
pub mod operation;
pub mod operator;
pub mod puzzle;
pub mod shape;
pub mod table;
pub mod tuples;

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
    /// No [`Fill`]s for a [`Cage`].
    EmptyFills,
    /// Index out of bounds.
    IndexOutOfBounds(usize),
}
