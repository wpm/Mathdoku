use crate::mdk::Error;
use crate::mdk::Error::InvalidPolyomino;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A grid position identified by `(row, column)`, both 1-indexed.
#[derive(Ord, Eq, PartialEq, Hash, PartialOrd, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Cell(pub usize, pub usize);

/// A set of edge-adjacent cells.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug)]
pub struct Polyomino(BTreeSet<Cell>);

impl Polyomino {
    /// Constructs a polyomino from `cells`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidPolyomino`] if the cells are empty or not edge-connected.
    pub fn from(cells: impl IntoIterator<Item = Cell>) -> Result<Self, Error> {
        let cells: Vec<Cell> = cells.into_iter().collect();
        if is_edge_adjacent(&cells) {
            Ok(Self(BTreeSet::from_iter(cells)))
        } else {
            Err(InvalidPolyomino(cells))
        }
    }

    /// Returns the number of cells in this polyomino.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if this polyomino contains no cells.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` if `cell` is part of this polyomino.
    #[must_use]
    pub fn contains(&self, cell: &Cell) -> bool {
        self.0.contains(cell)
    }

    /// Returns an iterator over the cells of this polyomino in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = &Cell> {
        self.0.iter()
    }
}

/// Returns `true` if the cells form an edge-connected component.
///
/// Uses DFS from the first cell. When checking neighbours, only looks right
/// (col+1) and down (row+1) while iterating — sufficient because the set is
/// sorted row-major and back-edges (left/up) are discovered from the other end.
fn is_edge_adjacent(cells: &[Cell]) -> bool {
    if cells.is_empty() {
        return false;
    }
    let mut visited: BTreeSet<Cell> = BTreeSet::new();
    let mut stack: Vec<Cell> = vec![cells[0]];
    while let Some(cell) = stack.pop() {
        if visited.insert(cell) {
            let Cell(r, c) = cell;
            for neighbor in [
                Cell(r, c + 1),
                Cell(r + 1, c),
                Cell(r, c.wrapping_sub(1)),
                Cell(r.wrapping_sub(1), c),
            ] {
                if cells.contains(&neighbor) {
                    stack.push(neighbor);
                }
            }
        }
    }
    visited.len() == cells.len()
}

impl IntoIterator for Polyomino {
    type Item = Cell;
    type IntoIter = std::collections::btree_set::IntoIter<Cell>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::mdk::Error::InvalidPolyomino;
    use crate::mdk::polyomino::{Cell, Polyomino};

    #[test]
    fn polyomino_single_cell_is_connected() {
        assert!(Polyomino::from([Cell(1, 1)]).is_ok());
    }

    #[test]
    fn polyomino_horizontal_pair_is_connected() {
        assert!(Polyomino::from([Cell(1, 1), Cell(1, 2)]).is_ok());
    }

    #[test]
    fn polyomino_vertical_pair_is_connected() {
        assert!(Polyomino::from([Cell(1, 1), Cell(2, 1)]).is_ok());
    }

    #[test]
    fn polyomino_l_shape_is_connected() {
        assert!(Polyomino::from([Cell(1, 1), Cell(1, 2), Cell(2, 1)]).is_ok());
    }

    #[test]
    fn polyomino_empty_is_disconnected() {
        assert!(matches!(Polyomino::from([]), Err(InvalidPolyomino(_))));
    }

    #[test]
    fn polyomino_diagonal_pair_is_disconnected() {
        assert!(matches!(
            Polyomino::from([Cell(1, 1), Cell(2, 2)]),
            Err(InvalidPolyomino(_))
        ));
    }

    #[test]
    fn polyomino_two_separate_pairs_is_disconnected() {
        assert!(matches!(
            Polyomino::from([Cell(1, 1), Cell(1, 2), Cell(3, 3), Cell(3, 4)]),
            Err(InvalidPolyomino(_))
        ));
    }

    #[test]
    fn polyomino_into_iter_yields_cells_in_order() {
        let p = Polyomino::from([Cell(2, 1), Cell(1, 2), Cell(1, 1)]).unwrap();
        let cells: Vec<Cell> = p.into_iter().collect();
        assert_eq!(cells, vec![Cell(1, 1), Cell(1, 2), Cell(2, 1)]);
    }

    #[test]
    fn polyomino_into_iter_singleton() {
        let p = Polyomino::from([Cell(3, 4)]).unwrap();
        let cells: Vec<Cell> = p.into_iter().collect();
        assert_eq!(cells, vec![Cell(3, 4)]);
    }
}
