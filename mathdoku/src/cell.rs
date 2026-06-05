//! The primitive grid types: [`Cell`] and numeric type aliases.

use crate::mdk::N;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

/// A cage target (sum, product, difference, ratio, or given value).
pub type Target = u64;
/// An ordered assignment of values to the cells of a cage, one value per cell.
pub type Tuple = Vec<N>;

/// A cell in a Mathdoku grid, identified by 0-based row and column index values
/// in row-major order.
#[must_use]
#[derive(
    Ord, Eq, PartialEq, PartialOrd, Debug, Copy, Clone, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Cell {
    /// 0-based row index.
    pub row: usize,
    /// 0-based column index.
    pub column: usize,
}

impl Cell {
    /// Creates a cell at the given `row` and `column`.
    pub const fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }

    /// Returns the up to four edge-adjacent cells (north, south, west, east).
    ///
    /// Cells above row 0 or left of column 0 are omitted. Cells below or to
    /// the right of the grid boundary are **not** filtered — callers must
    /// apply their own bounds check against the grid size.
    pub fn neighbors_4(self) -> impl Iterator<Item = Self> {
        [
            self.row.checked_sub(1).map(|r| Self::new(r, self.column)),
            Some(Self::new(self.row + 1, self.column)),
            self.column.checked_sub(1).map(|c| Self::new(self.row, c)),
            Some(Self::new(self.row, self.column + 1)),
        ]
        .into_iter()
        .flatten()
    }
}

impl Display for Cell {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.row, self.column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_display_shows_row_and_column() {
        assert_eq!(Cell::new(2, 3).to_string(), "(2, 3)");
    }

    #[test]
    fn cell_ordering_is_row_major() {
        assert!(Cell::new(0, 1) < Cell::new(1, 0));
    }

    #[test]
    fn neighbors_4_interior_yields_four() {
        let n: Vec<Cell> = Cell::new(2, 2).neighbors_4().collect();
        assert_eq!(n.len(), 4);
        assert!(n.contains(&Cell::new(1, 2)));
        assert!(n.contains(&Cell::new(3, 2)));
        assert!(n.contains(&Cell::new(2, 1)));
        assert!(n.contains(&Cell::new(2, 3)));
    }

    #[test]
    fn neighbors_4_top_left_corner_yields_two() {
        let n: Vec<Cell> = Cell::new(0, 0).neighbors_4().collect();
        assert_eq!(n.len(), 2);
        assert!(n.contains(&Cell::new(1, 0)));
        assert!(n.contains(&Cell::new(0, 1)));
    }
}
