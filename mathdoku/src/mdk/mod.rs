//! New Mathdoku implementation — work in progress; will eventually replace the top-level crate API.

use crate::mdk::fill::Fill;
use crate::mdk::polyomino::Polyomino;
use polyomino::Cell;

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
pub mod polyomino;
pub mod puzzle;
pub mod table;
pub mod tuples;

type N = u32;
type Target = u32;

/// Errors returned by mdk operations.
#[derive(Debug)]
pub enum Error {
    /// Invalid grid size
    InvalidGridSize(usize),
    /// The [`Cell`]s do not form a [`Polyomino`]
    InvalidPolyomino(Vec<Cell>),
    /// The [`Cell`] is missing from the specified polyomino or grid
    MissingCell(Cell),
    /// Invalid fill for a cage
    InvalidCageFill(Polyomino, Fill),
    /// No candidate fills for a cage
    EmptyFills,
    /// The index for a [`Cell`] in a cage is out of bounds
    InvalidCellCageIndex(usize),
}
