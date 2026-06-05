//! New Mathdoku implementation — work in progress; will eventually replace the top-level crate API.

use crate::mdk::fill::Fill;
use crate::mdk::polyomino::{Cell, Polyomino};

mod cage;
pub mod csp;
pub(crate) mod fill;
mod grid;
pub mod mdd;
pub mod memo;
pub mod operator;
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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidGridSize(n) => write!(f, "invalid grid size: {n}"),
            Self::InvalidPolyomino(cells) => write!(f, "cells do not form a polyomino: {cells:?}"),
            Self::MissingCell(cell) => write!(f, "cell not in grid or polyomino: {cell}"),
            Self::InvalidCageFill(poly, fill) => {
                write!(f, "invalid fill {fill} for cage {poly:?}")
            }
            Self::EmptyFills => write!(f, "no candidate fills for cage"),
            Self::InvalidCellCageIndex(i) => write!(f, "cell cage index out of bounds: {i}"),
            Self::InvalidCellValue(cell, n) => {
                write!(f, "value {n} not a candidate for cell {cell}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdk::fill::Fill;
    use crate::mdk::polyomino::{Cell, Polyomino};

    #[test]
    fn error_display_invalid_grid_size() {
        assert_eq!(
            Error::InvalidGridSize(0).to_string(),
            "invalid grid size: 0"
        );
    }

    #[test]
    fn error_display_missing_cell() {
        assert_eq!(
            Error::MissingCell(Cell(2, 3)).to_string(),
            "cell not in grid or polyomino: (2, 3)"
        );
    }

    #[test]
    fn error_display_empty_fills() {
        assert_eq!(Error::EmptyFills.to_string(), "no candidate fills for cage");
    }

    #[test]
    fn error_display_invalid_cell_value() {
        assert_eq!(
            Error::InvalidCellValue(Cell(1, 1), 5).to_string(),
            "value 5 not a candidate for cell (1, 1)"
        );
    }

    #[test]
    fn error_display_invalid_cell_cage_index() {
        assert_eq!(
            Error::InvalidCellCageIndex(3).to_string(),
            "cell cage index out of bounds: 3"
        );
    }

    #[test]
    fn error_display_invalid_polyomino() {
        assert_eq!(
            Error::InvalidPolyomino(vec![Cell(1, 1), Cell(3, 3)]).to_string(),
            "cells do not form a polyomino: [Cell(1, 1), Cell(3, 3)]"
        );
    }

    #[test]
    fn error_display_invalid_cage_fill() {
        let poly = Polyomino::from([Cell(1, 1)]).unwrap();
        let fill = Fill::from(&[1, 2]);
        assert!(
            Error::InvalidCageFill(poly, fill)
                .to_string()
                .contains("invalid fill")
        );
    }
}
