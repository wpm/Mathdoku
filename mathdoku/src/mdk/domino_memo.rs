//! [`DominoMemo`]: memo for binary (2-cell) non-monotonic cage constraints.

use crate::mdk::Error;
use crate::mdk::Error::{InvalidCell, InvalidGridSize, InvalidPolyomino};
use crate::mdk::N;
use crate::mdk::Target;
use crate::mdk::cage::NonCommutative;
use crate::mdk::fill::{Fill, Memo};
use crate::mdk::grid::{Cell, Polyomino};
use std::collections::HashMap;

/// Memo for two-cell non-monotonic cages (subtract, divide).
///
/// Stores all valid `(a, b)` pairs where `op(a, b) == target` as a lookup table
/// indexed by the first cell's value. `fill` and `remove` run in O(n) time.
pub struct DominoMemo {
    /// The two cells in [`Cell`] order.
    cells: [Cell; 2],
    /// Tuples that can fill these cells.
    tuples: Vec<(N, N)>,
}

impl DominoMemo {
    pub fn new(
        n: usize,
        domino: &Polyomino,
        op: NonCommutative,
        target: Target,
    ) -> Result<Self, Error> {
        if n == 1 {
            return Err(InvalidGridSize(n));
        }
        if domino.len() != 2 {
            return Err(InvalidPolyomino(domino.iter().copied().collect()));
        }
        #[allow(clippy::cast_possible_truncation)]
        let m = n as N;
        let tuples = (1..=m)
            .flat_map(|i| (1..=m).map(move |j| (i, j)))
            .filter(|(i, j)| op.apply(*i, *j) == target)
            .collect();
        let mut cell_iter = domino.iter().copied();
        #[allow(clippy::unwrap_used)]
        let mut cells = [cell_iter.next().unwrap(), cell_iter.next().unwrap()];
        cells.sort();
        Ok(Self { cells, tuples })
    }

    fn index(&self, cell: &Cell) -> Result<usize, Error> {
        self.cells
            .iter()
            .position(|&c| c == *cell)
            .ok_or(InvalidCell(*cell))
    }
}

impl Memo for DominoMemo {
    fn fill(&self, cell: &Cell) -> Result<Fill, Error> {
        let index = self.index(cell)?;
        let fill: Vec<N> = self
            .tuples
            .iter()
            .map(|tuple| [tuple.0, tuple.1][index])
            .collect();
        Ok(Fill::from(&fill))
    }

    fn remove(&self, fills: HashMap<Cell, Fill>) -> Result<Self, Error> {
        let mut tuples = self.tuples.clone();
        for (cell, fill) in &fills {
            let index = self.index(cell)?;
            tuples.retain(|tuple| fill.contains([tuple.0, tuple.1][index]));
        }
        Ok(Self {
            cells: self.cells,
            tuples,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdk::fill::Memo;
    use crate::mdk::grid::Polyomino;

    fn pair(r0: usize, c0: usize, r1: usize, c1: usize) -> Polyomino {
        Polyomino::from_cells([Cell::new(r0, c0), Cell::new(r1, c1)]).unwrap()
    }

    fn fill(vals: &[N]) -> Fill {
        Fill::from(vals)
    }

    #[test]
    fn subtract_fill_c0_contains_all_values_with_valid_partner() {
        // n=4, target=1: pairs are (1,2),(2,1),(2,3),(3,2),(3,4),(4,3)
        // c0 candidates: {1,2,3,4}
        let memo = DominoMemo::new(4, &pair(1, 1, 1, 2), NonCommutative::Subtract, 1).unwrap();
        assert_eq!(memo.fill(&Cell::new(1, 1)).unwrap(), fill(&[1, 2, 3, 4]));
    }

    #[test]
    fn subtract_fill_c1_contains_all_values_with_valid_partner() {
        // c1 candidates: {1,2,3,4}
        let memo = DominoMemo::new(4, &pair(1, 1, 1, 2), NonCommutative::Subtract, 1).unwrap();
        assert_eq!(memo.fill(&Cell::new(1, 2)).unwrap(), fill(&[1, 2, 3, 4]));
    }

    #[test]
    fn subtract_fill_invalid_cell_returns_error() {
        let memo = DominoMemo::new(4, &pair(1, 1, 1, 2), NonCommutative::Subtract, 1).unwrap();
        assert!(matches!(memo.fill(&Cell::new(2, 1)), Err(InvalidCell(_))));
    }

    #[test]
    fn divide_fill_c0_target2_n4() {
        // n=4, target=2: pairs are (2,1),(4,2)
        // c0 candidates: {2,4}
        let memo = DominoMemo::new(4, &pair(1, 1, 1, 2), NonCommutative::Divide, 2).unwrap();
        assert_eq!(memo.fill(&Cell::new(1, 1)).unwrap(), fill(&[2, 4]));
    }

    #[test]
    fn divide_fill_c1_target2_n4() {
        // c1 candidates: {1,2}
        let memo = DominoMemo::new(4, &pair(1, 1, 1, 2), NonCommutative::Divide, 2).unwrap();
        assert_eq!(memo.fill(&Cell::new(1, 2)).unwrap(), fill(&[1, 2]));
    }

    #[test]
    fn remove_c0_prunes_pairs() {
        // subtract target=1, n=4; restrict c0 to {1,2}
        let memo = DominoMemo::new(4, &pair(1, 1, 1, 2), NonCommutative::Subtract, 1).unwrap();
        let pruned = memo
            .remove(HashMap::from([(Cell::new(1, 1), fill(&[1, 2]))]))
            .unwrap();
        // remaining pairs: (1,2),(2,1),(2,3) → c0={1,2}, c1={1,2,3}
        assert_eq!(pruned.fill(&Cell::new(1, 1)).unwrap(), fill(&[1, 2]));
        assert_eq!(pruned.fill(&Cell::new(1, 2)).unwrap(), fill(&[1, 2, 3]));
    }

    #[test]
    fn remove_c1_prunes_pairs() {
        // subtract target=1, n=4; restrict c1 to {2,3}
        let memo = DominoMemo::new(4, &pair(1, 1, 1, 2), NonCommutative::Subtract, 1).unwrap();
        let pruned = memo
            .remove(HashMap::from([(Cell::new(1, 2), fill(&[2, 3]))]))
            .unwrap();
        // remaining pairs: (1,2),(2,3),(3,2),(4,3) → c0={1,2,3,4}, c1={2,3}
        assert_eq!(pruned.fill(&Cell::new(1, 1)).unwrap(), fill(&[1, 2, 3, 4]));
        assert_eq!(pruned.fill(&Cell::new(1, 2)).unwrap(), fill(&[2, 3]));
    }

    #[test]
    fn remove_both_cells_prunes_intersection() {
        // subtract target=1, n=4; restrict c0 to {1,2}, c1 to {2,3}
        let memo = DominoMemo::new(4, &pair(1, 1, 1, 2), NonCommutative::Subtract, 1).unwrap();
        let pruned = memo
            .remove(HashMap::from([
                (Cell::new(1, 1), fill(&[1, 2])),
                (Cell::new(1, 2), fill(&[2, 3])),
            ]))
            .unwrap();
        // surviving pairs: (1,2),(2,3) → c0={1,2}, c1={2,3}
        assert_eq!(pruned.fill(&Cell::new(1, 1)).unwrap(), fill(&[1, 2]));
        assert_eq!(pruned.fill(&Cell::new(1, 2)).unwrap(), fill(&[2, 3]));
    }

    #[test]
    fn remove_invalid_cell_returns_error() {
        let memo = DominoMemo::new(4, &pair(1, 1, 1, 2), NonCommutative::Subtract, 1).unwrap();
        let result = memo.remove(HashMap::from([(Cell::new(9, 9), fill(&[1]))]));
        assert!(matches!(result, Err(InvalidCell(_))));
    }

    #[test]
    fn impossible_target_gives_empty_fills() {
        // subtract target=9 on n=4: no pairs exist
        let memo = DominoMemo::new(4, &pair(1, 1, 1, 2), NonCommutative::Subtract, 9).unwrap();
        assert_eq!(memo.fill(&Cell::new(1, 1)).unwrap(), fill(&[]));
        assert_eq!(memo.fill(&Cell::new(1, 2)).unwrap(), fill(&[]));
    }
}
