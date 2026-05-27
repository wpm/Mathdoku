//! A [`Cage`]: a polyomino with an arithmetic constraint.
//!
//! A cage combines a polyomino (the set of cells it covers) with an
//! [`Operation`] (an [`Operator`] and numeric target). [`Cage::tuples`] enumerates
//! every ordered assignment of values to the cage's cells that satisfies the
//! arithmetic constraint and the all-different rule within each shared row and
//! column of the polyomino.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use crate::arithmetic::Tuple;
use crate::operation::Operation;
use crate::polyomino::Polyomino;
use crate::{Cell, N, tuples};

/// A polyomino with an [`Operation`] constraining its cell values.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Cage {
    polyomino: Polyomino,
    operation: Operation,
}

impl Cage {
    /// Creates a cage from a polyomino and an operation.
    pub const fn new(polyomino: Polyomino, operation: Operation) -> Self {
        Self {
            polyomino,
            operation,
        }
    }

    /// Returns the cells covered by this cage.
    pub fn cells(&self) -> Vec<Cell> {
        self.polyomino.cells()
    }

    /// Returns the operation (operator and target) for this cage.
    pub fn operation(&self) -> Operation {
        self.operation.clone()
    }

    /// Returns a reference to the polyomino for this cage.
    pub const fn polyomino(&self) -> &Polyomino {
        &self.polyomino
    }

    /// Valid ordered value assignments for this cage in an `n`×`n` grid.
    ///
    /// Delegates to the free function `tuples`.
    pub fn tuples(&self, n: N) -> impl Iterator<Item = Tuple> {
        tuples::tuples(n, self.polyomino(), self.operation())
    }
}

impl Ord for Cage {
    fn cmp(&self, other: &Self) -> Ordering {
        self.polyomino.cmp(&other.polyomino)
    }
}

impl PartialOrd for Cage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::test_utils::{col_pair, l_shape, pair, singleton};
    use crate::{M, Operator};

    fn cage(polyomino: Polyomino, operator: Operator, target: M) -> Cage {
        Cage {
            polyomino,
            operation: Operation { operator, target },
        }
    }

    // --- Given ---

    #[test]
    fn given_singleton_yields_one_tuple() {
        let tuples: Vec<_> = cage(singleton(), Operator::Given, 3).tuples(4).collect();
        assert_eq!(tuples, vec![vec![3]]);
    }

    #[test]
    fn given_out_of_range_yields_no_tuples() {
        // target 5 is not in 1..=4
        assert!(
            cage(singleton(), Operator::Given, 5)
                .tuples(4)
                .next()
                .is_none()
        );
    }

    // --- Add ---

    #[test]
    fn add_pair_yields_correct_tuples() {
        // n=4, target=3: only {1,2} works; both orderings survive collinearity (same row)
        let tuples: Vec<_> = cage(pair(), Operator::Add, 3).tuples(4).collect();
        assert!(tuples.contains(&vec![1, 2]));
        assert!(tuples.contains(&vec![2, 1]));
        assert_eq!(tuples.len(), 2);
    }

    #[test]
    fn add_col_pair_collinearity_excludes_duplicates() {
        // col_pair: (0,0),(1,0) — the same column, so values must differ.
        // target=4, n=4: multisets are {1,3},{2,2},{3,1} but {2,2} has duplicate in column.
        let tuples: Vec<_> = cage(col_pair(), Operator::Add, 4).tuples(4).collect();
        for t in &tuples {
            assert_ne!(t[0], t[1], "collinear cells must not repeat a value");
        }
    }

    // --- Subtract ---

    #[test]
    fn subtract_pair_yields_correct_tuples() {
        // n=4, target=1: pairs differing by 1 are (1,2),(2,1),(2,3),(3,2),(3,4),(4,3)
        let tuples: Vec<_> = cage(pair(), Operator::Subtract, 1).tuples(4).collect();
        assert_eq!(tuples.len(), 6);
        for t in &tuples {
            let diff = (i32::from(t[0]) - i32::from(t[1])).unsigned_abs();
            assert_eq!(diff, 1);
        }
    }

    // --- Multiply ---

    #[test]
    fn multiply_pair_yields_correct_tuples() {
        // n=4, target=6: {2,3} → [2,3],[3,2]
        let tuples: Vec<_> = cage(pair(), Operator::Multiply, 6).tuples(4).collect();
        assert!(tuples.contains(&vec![2, 3]));
        assert!(tuples.contains(&vec![3, 2]));
        assert_eq!(tuples.len(), 2);
    }

    // --- Divide ---

    #[test]
    fn divide_pair_yields_correct_tuples() {
        // n=4, target=2: pairs with ratio 2 are (1,2),(2,1),(2,4),(4,2)
        let tuples: Vec<_> = cage(pair(), Operator::Divide, 2).tuples(4).collect();
        assert_eq!(tuples.len(), 4);
        for t in &tuples {
            let (a, b) = (u16::from(t[0]), u16::from(t[1]));
            assert!(a * 2 == b || b * 2 == a);
        }
    }

    // --- collinearity with l-shape ---

    #[test]
    fn l_shape_tuples_satisfy_row_and_column_all_different() {
        // l_shape: (0,0),(1,0),(1,1) — col 0 has cells 0 and 1; row 1 has cells 1 and 2.
        let tuples: Vec<_> = cage(l_shape(), Operator::Add, 6).tuples(4).collect();
        for t in &tuples {
            assert_ne!(t[0], t[1], "col 0: cells (0,0) and (1,0) must differ");
            assert_ne!(t[1], t[2], "row 1: cells (1,0) and (1,1) must differ");
        }
    }

    // --- Operator Display ---

    #[test]
    fn operator_display() {
        assert_eq!(Operator::Add.to_string(), "+");
        assert_eq!(Operator::Subtract.to_string(), "−");
        assert_eq!(Operator::Multiply.to_string(), "×");
        assert_eq!(Operator::Divide.to_string(), "÷");
        assert_eq!(Operator::Given.to_string(), "");
    }

    // --- Operation Display ---

    #[test]
    fn operation_display_with_symbol() {
        assert_eq!(Operation::new(Operator::Add, 12).to_string(), "+12");
        assert_eq!(Operation::new(Operator::Subtract, 3).to_string(), "−3");
        assert_eq!(Operation::new(Operator::Multiply, 24).to_string(), "×24");
        assert_eq!(Operation::new(Operator::Divide, 2).to_string(), "÷2");
    }

    #[test]
    fn operation_display_given_has_no_symbol() {
        assert_eq!(Operation::new(Operator::Given, 7).to_string(), "7");
    }
}
