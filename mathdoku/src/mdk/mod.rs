//! New Mathdoku implementation — work in progress; will eventually replace the top-level crate API.

use crate::mdk::fill::Fill;
use crate::mdk::polyomino::Polyomino;
use polyomino::Cell;

mod cage;
pub mod csp;
pub(crate) mod fill;
mod grid;
pub mod mdd;
pub mod memo;
pub mod operation;
pub mod polyomino;
pub mod puzzle;
pub(crate) mod regin;
pub mod table;
pub mod tuples;

/// A cell value in the range `1..=9`.
pub type N = u8;

/// The accumulated result of an arithmetic cage operation (sum or product of [`N`] values).
///
/// Sums and products of up to nine 9s can reach 729, which overflows `u8` and `u16`.
/// `u32` is wide enough for any realistic Mathdoku constraint.
type T = u32;

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
    /// Value not permitted in this [`Cell`].
    InvalidCellValue(Cell, N),
}
