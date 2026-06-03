//! [`BinaryMemo`]: memo for binary (2-cell) non-monotonic cage constraints.
use crate::mdk::Error;
use crate::mdk::N;
use crate::mdk::Target;
use crate::mdk::cage::NonMonotonicOp;
use crate::mdk::fill::{Fill, Memo};
use crate::mdk::grid::{Cell, Polyomino};
use std::collections::HashMap;

/// Memo for binary non-monotonic cages (subtract, divide).
///
/// Stores all valid `(a, b)` pairs where `op(a, b) == target` as a lookup table
/// indexed by the first cell's value. `fill` and `remove` run in O(n) time.
pub struct BinaryMemo {
    /// The two cells in polyomino order.
    cells: [Cell; 2],
    /// `nodes[a-1]` is the list of valid `b` values when the first cell has value `a`.
    nodes: Vec<Vec<N>>,
}

impl BinaryMemo {
    /// Builds the memo for a binary `polyomino` with `op` and `target` on a grid of size `n`.
    ///
    /// Enumerates all pairs `(a, b)` in `1..=n` satisfying `op(a, b) == target`.
    ///
    /// # Panics
    ///
    /// Panics if `polyomino` does not contain exactly 2 cells.
    #[must_use]
    pub fn new(n: usize, polyomino: &Polyomino, op: NonMonotonicOp, target: Target) -> Self {
        assert_eq!(polyomino.len(), 2, "polyomino has exactly 2 cells");
        let mut cell_iter = polyomino.iter().copied();
        #[allow(clippy::expect_used)]
        let c0 = cell_iter.next().expect("len == 2");
        #[allow(clippy::expect_used)]
        let c1 = cell_iter.next().expect("len == 2");

        #[allow(clippy::cast_possible_truncation)]
        let n = n as N;
        let mut nodes = vec![Vec::new(); n as usize];
        for a in 1..=n {
            for b in 1..=n {
                if op.apply(a, b) == target {
                    #[allow(clippy::cast_possible_truncation)]
                    nodes[(a - 1) as usize].push(b);
                }
            }
        }
        Self {
            cells: [c0, c1],
            nodes,
        }
    }
}

impl Memo for BinaryMemo {
    /// Returns the candidate fill for `cell`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCell`] if `cell` is not one of the two cage cells.
    fn fill(&self, cell: &Cell) -> Result<Fill, Error> {
        if cell == &self.cells[0] {
            let vals: Vec<N> = self
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, bs)| !bs.is_empty())
                .map(|(i, _)| {
                    #[allow(clippy::cast_possible_truncation)]
                    {
                        i as N + 1
                    }
                })
                .collect();
            Ok(Fill::from(&vals))
        } else if cell == &self.cells[1] {
            let vals: Vec<N> = self
                .nodes
                .iter()
                .flatten()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            Ok(Fill::from(&vals))
        } else {
            Err(Error::InvalidCell(*cell))
        }
    }

    /// Returns a new memo with paths pruned to the candidates in `fills`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCell`] if any cell in `fills` is not part of the cage.
    fn remove(&self, fills: HashMap<Cell, Fill>) -> Result<Self, Error> {
        for cell in fills.keys() {
            if cell != &self.cells[0] && cell != &self.cells[1] {
                return Err(Error::InvalidCell(*cell));
            }
        }
        let f0 = fills.get(&self.cells[0]);
        let f1 = fills.get(&self.cells[1]);
        let nodes = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, bs)| {
                #[allow(clippy::cast_possible_truncation)]
                let a = i as N + 1;
                if f0.is_some_and(|f| !f.contains(a)) {
                    return Vec::new();
                }
                f1.map_or_else(
                    || bs.clone(),
                    |f| bs.iter().copied().filter(|&b| f.contains(b)).collect(),
                )
            })
            .collect();
        Ok(Self {
            cells: self.cells,
            nodes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdk::fill::Memo;
    use crate::mdk::grid::Polyomino;

    fn pair(r0: usize, c0: usize, r1: usize, c1: usize) -> Polyomino {
        Polyomino::from_cells([Cell::new(r0, c0), Cell::new(r1, c1)])
    }

    fn fill(vals: &[N]) -> Fill {
        Fill::from(vals)
    }

    #[test]
    fn subtract_fill_c0_contains_all_values_with_valid_partner() {
        // n=4, target=1: pairs are (1,2),(2,1),(2,3),(3,2),(3,4),(4,3)
        // c0 candidates: {1,2,3,4}
        let memo = BinaryMemo::new(4, &pair(1, 1, 1, 2), NonMonotonicOp::Subtract, 1);
        assert_eq!(memo.fill(&Cell::new(1, 1)).unwrap(), fill(&[1, 2, 3, 4]));
    }

    #[test]
    fn subtract_fill_c1_contains_all_values_with_valid_partner() {
        // c1 candidates: {1,2,3,4}
        let memo = BinaryMemo::new(4, &pair(1, 1, 1, 2), NonMonotonicOp::Subtract, 1);
        assert_eq!(memo.fill(&Cell::new(1, 2)).unwrap(), fill(&[1, 2, 3, 4]));
    }

    #[test]
    fn subtract_fill_invalid_cell_returns_error() {
        let memo = BinaryMemo::new(4, &pair(1, 1, 1, 2), NonMonotonicOp::Subtract, 1);
        assert!(matches!(
            memo.fill(&Cell::new(2, 1)),
            Err(Error::InvalidCell(_))
        ));
    }

    #[test]
    fn divide_fill_c0_target2_n4() {
        // n=4, target=2: pairs are (2,1),(4,2)
        // c0 candidates: {2,4}
        let memo = BinaryMemo::new(4, &pair(1, 1, 1, 2), NonMonotonicOp::Divide, 2);
        assert_eq!(memo.fill(&Cell::new(1, 1)).unwrap(), fill(&[2, 4]));
    }

    #[test]
    fn divide_fill_c1_target2_n4() {
        // c1 candidates: {1,2}
        let memo = BinaryMemo::new(4, &pair(1, 1, 1, 2), NonMonotonicOp::Divide, 2);
        assert_eq!(memo.fill(&Cell::new(1, 2)).unwrap(), fill(&[1, 2]));
    }

    #[test]
    fn remove_c0_prunes_pairs() {
        // subtract target=1, n=4; restrict c0 to {1,2}
        let memo = BinaryMemo::new(4, &pair(1, 1, 1, 2), NonMonotonicOp::Subtract, 1);
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
        let memo = BinaryMemo::new(4, &pair(1, 1, 1, 2), NonMonotonicOp::Subtract, 1);
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
        let memo = BinaryMemo::new(4, &pair(1, 1, 1, 2), NonMonotonicOp::Subtract, 1);
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
        let memo = BinaryMemo::new(4, &pair(1, 1, 1, 2), NonMonotonicOp::Subtract, 1);
        let result = memo.remove(HashMap::from([(Cell::new(9, 9), fill(&[1]))]));
        assert!(matches!(result, Err(Error::InvalidCell(_))));
    }

    #[test]
    fn impossible_target_gives_empty_fills() {
        // subtract target=9 on n=4: no pairs exist
        let memo = BinaryMemo::new(4, &pair(1, 1, 1, 2), NonMonotonicOp::Subtract, 9);
        assert_eq!(memo.fill(&Cell::new(1, 1)).unwrap(), fill(&[]));
        assert_eq!(memo.fill(&Cell::new(1, 2)).unwrap(), fill(&[]));
    }
}
