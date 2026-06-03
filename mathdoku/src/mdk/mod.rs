//! New Mathdoku implementation — work in progress; will eventually replace the top-level crate API.
use crate::mdk::grid::Cell;

pub mod cage;
pub(crate) mod domino_memo;
pub(crate) mod fill;
mod grid;
pub(crate) mod mdd;
pub mod puzzle;

type N = u32;
type Target = u32;

/// Errors returned by mdk operations.
#[derive(Debug)]
pub enum Error {
    /// Invalid Grid size
    InvalidGridSize(usize),
    /// The cell does not exist in the grid.
    InvalidCell(Cell),
    /// The cells do not form a polyomino
    InvalidPolyomino(Vec<Cell>),
}
