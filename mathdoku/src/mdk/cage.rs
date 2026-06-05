//! Cage representation pairing a polyomino with its arithmetic constraint.
//!
//! # Constraint kinds
//!
//! ## Commutative (add, multiply)
//!
//! Commutative operators are monotonically non-decreasing: extending a partial
//! tuple can only keep the accumulated result the same or increase it. This
//! monotonicity enables aggressive pruning during construction — branches whose
//! partial result already exceeds the target, or can no longer reach it, are cut
//! immediately. The result is stored as a [`Mdd`]: a DAG whose paths are exactly
//! the valid tuples, compressed by sharing common prefixes and suffixes. The MDD
//! also supports efficient incremental narrowing via [`Narrow::remove`]: forbidden
//! values at a given depth are removed and dead nodes are garbage-collected
//! without rebuilding the diagram from scratch.
//!
//! ## Non-commutative (subtract, divide)
//!
//! Subtract and divide are not monotonic, so the MDD pruning strategy does not
//! apply. They are also inherently binary: Mathdoku defines subtract as
//! `|a − b|` and divide as `max(a, b) / min(a, b)`, neither of which generalises
//! meaningfully beyond a pair. Non-commutative cages are therefore always
//! dominoes (exactly 2 cells), and their constraint is stored as a [`Table`] —
//! the explicit list of valid pairs.
//!
//! ## Given
//!
//! A given cage is a singleton cell whose value is fixed by the puzzle author.
//! There is no arithmetic constraint and no memo: the value is stored directly.
use crate::mdk::mdd::Mdd;
use crate::mdk::operation::{CommutativeOperation, NonCommutativeOperation};
use crate::mdk::polyomino::{Cell, Polyomino};
use crate::mdk::table::Table;
use crate::mdk::{Error, N, Target};

/// The arithmetic constraint and its backing memo for a cage.
enum CageOperation {
    /// A commutative (monotonic) operation: add or multiply.
    Commutative(CommutativeOperation, Target, Mdd),
    /// A non-commutative (non-monotonic) operation: subtract or divide.
    NonCommutative(NonCommutativeOperation, Target, Table),
    /// A single cell with a fixed value; no arithmetic constraint.
    Given(N),
}

/// A cage: a connected group of cells subject to an arithmetic constraint.
///
/// The constraint is one of:
/// - **Commutative** (`Add`, `Multiply`): backed by an [`Mdd`] for efficient narrowing.
/// - **NonCommutative** (`Subtract`, `Divide`): backed by a [`Table`] of explicit pairs.
/// - **Given**: a singleton cell whose value is fixed.
struct Cage {
    polyomino: Polyomino,
    operation: CageOperation,
}

impl Cage {
    /// Constructs a cage for a commutative constraint over `polyomino`.
    ///
    /// Builds an MDD representing all `polyomino.len()`-tuples of values in
    /// `1..=n` whose `operation` equals `target`.
    ///
    /// # Errors
    /// Returns [`Error::EmptyFills`] if no tuples satisfy the constraint.
    pub fn commutative(
        n: usize,
        polyomino: Polyomino,
        operation: CommutativeOperation,
        target: Target,
    ) -> Result<Self, Error> {
        let mdd = Mdd::new(n, polyomino.len(), operation, target)?;
        let operation = CageOperation::Commutative(operation, target, mdd);
        Ok(Self {
            polyomino,
            operation,
        })
    }

    /// Constructs a cage for a non-commutative constraint over `polyomino`.
    ///
    /// Builds a [`Table`] of all pairs of values in `1..=n` whose `operation`
    /// equals `target`. Non-commutative cages must be exactly 2 cells.
    ///
    /// # Errors
    /// Returns [`Error::EmptyFills`] if no pairs satisfy the constraint.
    pub fn non_commutative(
        n: usize,
        polyomino: Polyomino,
        operation: NonCommutativeOperation,
        target: Target,
    ) -> Result<Self, Error> {
        let table = Table::non_commutative(n, operation, target)?;
        let operation = CageOperation::NonCommutative(operation, target, table);
        Ok(Self {
            polyomino,
            operation,
        })
    }

    /// Constructs a given cage: a single cell whose value is fixed to `target`.
    ///
    /// # Errors
    /// Returns [`Error::InvalidPolyomino`] if `cell` cannot form a polyomino
    /// (should never happen for a valid cell).
    pub fn given(cell: Cell, target: N) -> Result<Self, Error> {
        Ok(Self {
            polyomino: Polyomino::from(vec![cell])?,
            operation: CageOperation::Given(target),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdk::operation::CommutativeOperation::{Add, Multiply};
    use crate::mdk::operation::NonCommutativeOperation::{Divide, Subtract};

    fn domino(r0: usize, c0: usize, r1: usize, c1: usize) -> Polyomino {
        Polyomino::from([Cell(r0, c0), Cell(r1, c1)]).unwrap()
    }

    fn triomino(r0: usize, c0: usize, r1: usize, c1: usize, r2: usize, c2: usize) -> Polyomino {
        Polyomino::from([Cell(r0, c0), Cell(r1, c1), Cell(r2, c2)]).unwrap()
    }

    // ---- commutative ----

    #[test]
    fn commutative_add_succeeds() {
        assert!(Cage::commutative(4, domino(1, 1, 1, 2), Add, 5).is_ok());
    }

    #[test]
    fn commutative_multiply_succeeds() {
        assert!(Cage::commutative(4, domino(1, 1, 1, 2), Multiply, 6).is_ok());
    }

    #[test]
    fn commutative_triple_succeeds() {
        assert!(Cage::commutative(4, triomino(1, 1, 1, 2, 1, 3), Add, 6).is_ok());
    }

    #[test]
    fn commutative_infeasible_target_returns_empty_fills() {
        assert!(matches!(
            Cage::commutative(4, domino(1, 1, 1, 2), Add, 9),
            Err(Error::EmptyFills)
        ));
    }

    #[test]
    fn commutative_stores_polyomino() {
        let poly = domino(1, 1, 1, 2);
        let cage = Cage::commutative(4, poly.clone(), Add, 5).unwrap();
        assert_eq!(cage.polyomino, poly);
    }

    // ---- non_commutative ----

    #[test]
    fn non_commutative_subtract_succeeds() {
        assert!(Cage::non_commutative(4, domino(1, 1, 1, 2), Subtract, 1).is_ok());
    }

    #[test]
    fn non_commutative_divide_succeeds() {
        assert!(Cage::non_commutative(4, domino(1, 1, 1, 2), Divide, 2).is_ok());
    }

    #[test]
    fn non_commutative_infeasible_target_returns_empty_fills() {
        // no pair in 1..=4 has |a-b| = 4
        assert!(matches!(
            Cage::non_commutative(4, domino(1, 1, 1, 2), Subtract, 4),
            Err(Error::EmptyFills)
        ));
    }

    #[test]
    fn non_commutative_stores_polyomino() {
        let poly = domino(2, 1, 2, 2);
        let cage = Cage::non_commutative(4, poly.clone(), Subtract, 1).unwrap();
        assert_eq!(cage.polyomino, poly);
    }

    // ---- given ----

    #[test]
    fn given_succeeds() {
        assert!(Cage::given(Cell(1, 1), 3).is_ok());
    }

    #[test]
    fn given_stores_singleton_polyomino() {
        let cage = Cage::given(Cell(2, 3), 5).unwrap();
        assert!(cage.polyomino.contains(&Cell(2, 3)));
        assert_eq!(cage.polyomino.len(), 1);
    }

    #[test]
    fn given_stores_target_as_value() {
        let cage = Cage::given(Cell(1, 1), 7).unwrap();
        assert!(matches!(cage.operation, CageOperation::Given(7)));
    }
}
