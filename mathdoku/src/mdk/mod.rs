//! New Mathdoku implementation — work in progress; will eventually replace the top-level crate API.
use crate::mdk::grid::Cell;

pub(crate) mod fill;
mod grid;
pub(crate) mod mdd;
pub mod puzzle;
pub(crate) mod trie;

type N = u32;
type Target = u32;

/// Errors returned by mdk operations.
pub enum Error {
    /// The cell does not exist in the grid.
    InvalidCell(Cell),
}
