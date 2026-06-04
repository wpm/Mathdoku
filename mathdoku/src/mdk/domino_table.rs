//! [`DominoTable`]: memo for binary (2-cell) non-monotonic cage constraints.

use crate::mdk::Error;
use crate::mdk::Error::{InvalidGridSize, InvalidPolyomino, MissingCell};
use crate::mdk::{N, Target};
use crate::mdk::old_cage::Operation;
use crate::mdk::operator::Operator;
use crate::mdk::fill::Fill;
use crate::mdk::memo::Memo;
use std::collections::HashMap;
use crate::mdk::shape::{Cell, Polyomino};

/// Memo for two-cell non-commutative cages (subtract, divide).
///
/// Stores all valid `(a, b)` pairs where `op(a, b) == target` as a lookup table
/// indexed by the first cell's value. `fill` and `remove` run in O(n) time.
#[derive(Clone)]
pub struct DominoTable {
    /// The two cells in [`Cell`] order.
    cells: [Cell; 2],
    /// Tuples that can fill these cells.
    tuples: Vec<(N, N)>,
}

impl DominoTable {
    pub fn new(n: usize, domino: &Polyomino, operation: Operation) -> Result<Self, Error> {
        if n == 1 {
            return Err(InvalidGridSize(n));
        }
        if domino.len() != 2 {
            return Err(InvalidPolyomino(domino.iter().copied().collect()));
        }
        #[allow(clippy::cast_possible_truncation)]
        let m = n as N;
        #[allow(clippy::todo)]
        let tuples: Vec<(N, N)> = (1..=m)
            .flat_map(|i| (1..=m).map(move |j| (i, j)))
            .filter(|&(i, j)| {
                let result: Target = match operation.0 {
                    Operator::Subtract => i.abs_diff(j),
                    Operator::Divide => i / j,
                    _ => todo!(),
                };
                result == operation.1
            })
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
            .ok_or(MissingCell(*cell))
    }
}

impl Memo for DominoTable {
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
    use crate::mdk::old_cage::Operation;
    use crate::mdk::shape::Polyomino;
    use crate::mdk::memo::Memo;
    use crate::mdk::operator::Operator::{Divide, Subtract};

    fn pair(r0: usize, c0: usize, r1: usize, c1: usize) -> Polyomino {
        Polyomino::from_cells([Cell(r0, c0), Cell(r1, c1)]).unwrap()
    }

    fn fill(vals: &[N]) -> Fill {
        Fill::from(vals)
    }

    #[test]
    fn subtract_fill_c0_contains_all_values_with_valid_partner() {
        // n=4, target=1: pairs are (1,2),(2,1),(2,3),(3,2),(3,4),(4,3)
        // c0 candidates: {1,2,3,4}
        let memo = DominoTable::new(4, &pair(1, 1, 1, 2), Operation(Subtract, 1)).unwrap();
        assert_eq!(memo.fill(&Cell(1, 1)).unwrap(), fill(&[1, 2, 3, 4]));
    }

    #[test]
    fn subtract_fill_c1_contains_all_values_with_valid_partner() {
        // c1 candidates: {1,2,3,4}
        let memo = DominoTable::new(4, &pair(1, 1, 1, 2), Operation(Subtract, 1)).unwrap();
        assert_eq!(memo.fill(&Cell(1, 2)).unwrap(), fill(&[1, 2, 3, 4]));
    }

    #[test]
    fn subtract_fill_invalid_cell_returns_error() {
        let memo = DominoTable::new(4, &pair(1, 1, 1, 2), Operation(Subtract, 1)).unwrap();
        assert!(matches!(memo.fill(&Cell(2, 1)), Err(MissingCell(_))));
    }

    #[test]
    fn divide_fill_c0_target2_n4() {
        // n=4, target=2: pairs are (2,1),(4,2)
        // c0 candidates: {2,4}
        let memo = DominoTable::new(4, &pair(1, 1, 1, 2), Operation(Divide, 2)).unwrap();
        assert_eq!(memo.fill(&Cell(1, 1)).unwrap(), fill(&[2, 4]));
    }

    #[test]
    fn divide_fill_c1_target2_n4() {
        // c1 candidates: {1,2}
        let memo = DominoTable::new(4, &pair(1, 1, 1, 2), Operation(Divide, 2)).unwrap();
        assert_eq!(memo.fill(&Cell(1, 2)).unwrap(), fill(&[1, 2]));
    }

    #[test]
    fn remove_c0_prunes_pairs() {
        // subtract target=1, n=4; restrict c0 to {1,2}
        let memo = DominoTable::new(4, &pair(1, 1, 1, 2), Operation(Subtract, 1)).unwrap();
        let pruned = memo
            .remove(HashMap::from([(Cell(1, 1), fill(&[1, 2]))]))
            .unwrap();
        // remaining pairs: (1,2),(2,1),(2,3) → c0={1,2}, c1={1,2,3}
        assert_eq!(pruned.fill(&Cell(1, 1)).unwrap(), fill(&[1, 2]));
        assert_eq!(pruned.fill(&Cell(1, 2)).unwrap(), fill(&[1, 2, 3]));
    }

    #[test]
    fn remove_c1_prunes_pairs() {
        // subtract target=1, n=4; restrict c1 to {2,3}
        let memo = DominoTable::new(4, &pair(1, 1, 1, 2), Operation(Subtract, 1)).unwrap();
        let pruned = memo
            .remove(HashMap::from([(Cell(1, 2), fill(&[2, 3]))]))
            .unwrap();
        // remaining pairs: (1,2),(2,3),(3,2),(4,3) → c0={1,2,3,4}, c1={2,3}
        assert_eq!(pruned.fill(&Cell(1, 1)).unwrap(), fill(&[1, 2, 3, 4]));
        assert_eq!(pruned.fill(&Cell(1, 2)).unwrap(), fill(&[2, 3]));
    }

    #[test]
    fn remove_both_cells_prunes_intersection() {
        // subtract target=1, n=4; restrict c0 to {1,2}, c1 to {2,3}
        let memo = DominoTable::new(4, &pair(1, 1, 1, 2), Operation(Subtract, 1)).unwrap();
        let pruned = memo
            .remove(HashMap::from([
                (Cell(1, 1), fill(&[1, 2])),
                (Cell(1, 2), fill(&[2, 3])),
            ]))
            .unwrap();
        // surviving pairs: (1,2),(2,3) → c0={1,2}, c1={2,3}
        assert_eq!(pruned.fill(&Cell(1, 1)).unwrap(), fill(&[1, 2]));
        assert_eq!(pruned.fill(&Cell(1, 2)).unwrap(), fill(&[2, 3]));
    }

    #[test]
    fn remove_invalid_cell_returns_error() {
        let memo = DominoTable::new(4, &pair(1, 1, 1, 2), Operation(Subtract, 1)).unwrap();
        let result = memo.remove(HashMap::from([(Cell(9, 9), fill(&[1]))]));
        assert!(matches!(result, Err(MissingCell(_))));
    }

    #[test]
    fn impossible_target_gives_empty_fills() {
        // subtract target=9 on n=4: no pairs exist
        let memo = DominoTable::new(4, &pair(1, 1, 1, 2), Operation(Subtract, 9)).unwrap();
        assert_eq!(memo.fill(&Cell(1, 1)).unwrap(), fill(&[]));
        assert_eq!(memo.fill(&Cell(1, 2)).unwrap(), fill(&[]));
    }
}
