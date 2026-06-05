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
use crate::mdk::fill::Fill;
use crate::mdk::mdd::Mdd;
use crate::mdk::memo::Memo;
use crate::mdk::operation::{CommutativeOperator, NonCommutativeOperator};
use crate::mdk::polyomino::{Cell, Polyomino};
use crate::mdk::table::Table;
use crate::mdk::{Error, N, Target};

/// The constraint for a cage and its backing memo.
#[derive(Clone, PartialEq, Eq, Debug)]
enum CageOperation {
    /// A commutative (monotonic) operation: add or multiply.
    Commutative(CommutativeOperator, Target, Mdd),
    /// A non-commutative (non-monotonic) operation: subtract or divide.
    NonCommutative(NonCommutativeOperator, Target, Table),
    /// A single cell with a fixed value.
    Given(N),
}

/// A cage: a connected group of cells subject to an arithmetic constraint.
///
/// The constraint is one of:
/// - **Commutative** (`Add`, `Multiply`): backed by an [`Mdd`] for efficient narrowing.
/// - **`NonCommutative`** (`Subtract`, `Divide`): backed by a [`Table`] of explicit pairs.
/// - **Given**: a singleton cell whose value is fixed.
#[derive(Debug, Clone)]
pub struct Cage {
    /// The grid size `n` (values are drawn from `1..=n`).
    n: usize,
    /// The cells belonging to this cage.
    pub polyomino: Polyomino,
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
        operation: CommutativeOperator,
        target: Target,
    ) -> Result<Self, Error> {
        let mdd = Mdd::new(n, polyomino.len(), operation, target)?;
        let operation = CageOperation::Commutative(operation, target, mdd);
        Ok(Self {
            n,
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
        operation: NonCommutativeOperator,
        target: Target,
    ) -> Result<Self, Error> {
        let table = Table::non_commutative(n, operation, target)?;
        let operation = CageOperation::NonCommutative(operation, target, table);
        Ok(Self {
            n,
            polyomino,
            operation,
        })
    }

    /// Constructs a given cage: a single cell whose value is fixed to `target`.
    ///
    /// Always succeeds for a valid `cell`; returns `Err` only if the cell cannot
    /// form a polyomino, which cannot happen for a single non-empty cell.
    pub fn given(cell: Cell, n: usize, target: N) -> Result<Self, Error> {
        Ok(Self {
            n,
            polyomino: Polyomino::from(vec![cell])?,
            operation: CageOperation::Given(target),
        })
    }

    /// Returns the candidate [`Fill`] for `cell`.
    ///
    /// # Errors
    /// Returns [`Error::MissingCell`] if `cell` is not in a [`Cage`].
    pub fn get(&self, cell: Cell) -> Result<Fill, Error> {
        let index = self.polyomino_index(cell)?;
        let fill = match &self.operation {
            CageOperation::Commutative(_, _, memo) => memo.get(index)?,
            CageOperation::NonCommutative(_, _, memo) => memo.get(index)?,
            CageOperation::Given(n) => Fill::from(self.n, &[*n]),
        };
        Ok(fill)
    }

    /// Assigns `fill` as the candidate set for `cell`, narrowing the memo.
    ///
    /// Returns a new `Cage` with the updated constraint. For a `Given` cage,
    /// succeeds only if `fill` contains the given value.
    ///
    /// # Errors
    /// - [`Error::MissingCell`] if `cell` is not in this cage.
    /// - [`Error::InvalidCageFill`] if `fill` is incompatible with the constraint.
    /// - [`Error::EmptyFills`] if no tuples survive after narrowing.
    pub fn set(&self, cell: Cell, fill: Fill) -> Result<Self, Error> {
        let index = self.polyomino_index(cell)?;
        let operation = match &self.operation {
            CageOperation::Commutative(op, target, memo) => {
                let reset = memo.reset();
                CageOperation::Commutative(
                    *op,
                    *target,
                    reset.narrow(self.fills_with(&reset, index, fill)?)?,
                )
            }
            CageOperation::NonCommutative(op, target, memo) => {
                let reset = memo.reset();
                CageOperation::NonCommutative(
                    *op,
                    *target,
                    reset.narrow(self.fills_with(&reset, index, fill)?)?,
                )
            }
            CageOperation::Given(v) => {
                if fill.contains(*v) {
                    self.operation.clone()
                } else {
                    return Err(Error::InvalidCageFill(self.polyomino.clone(), fill));
                }
            }
        };
        Ok(Self {
            n: self.n,
            polyomino: self.polyomino.clone(),
            operation,
        })
    }

    /// Builds a fills vector from `memo`, replacing position `index` with `fill`.
    fn fills_with<M: Memo>(&self, memo: &M, index: usize, fill: Fill) -> Result<Vec<Fill>, Error> {
        let mut fills: Vec<Fill> = (0..self.polyomino.len())
            .map(|i| memo.get(i))
            .collect::<Result<_, _>>()?;
        fills[index] = fill;
        Ok(fills)
    }

    /// Returns the index of `cell` in its containing [`Cage`].
    ///
    /// # Errors
    /// Returns [`Error::MissingCell`] if `cell` is not in a [`Cage`].
    fn polyomino_index(&self, cell: Cell) -> Result<usize, Error> {
        self.polyomino
            .iter()
            .position(|c| *c == cell)
            .ok_or(Error::MissingCell(cell))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdk::operation::CommutativeOperator::{Add, Multiply};
    use crate::mdk::operation::NonCommutativeOperator::{Divide, Subtract};

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
        assert!(Cage::given(Cell(1, 1), 4, 3).is_ok());
    }

    #[test]
    fn given_stores_singleton_polyomino() {
        let cage = Cage::given(Cell(2, 3), 4, 5).unwrap();
        assert!(cage.polyomino.contains(&Cell(2, 3)));
        assert_eq!(cage.polyomino.len(), 1);
    }

    #[test]
    fn given_stores_target_as_value() {
        let cage = Cage::given(Cell(1, 1), 4, 7).unwrap();
        assert_eq!(cage.operation, CageOperation::Given(7));
    }

    // ---- get ----

    #[test]
    fn get_missing_cell_returns_error() {
        let cage = Cage::commutative(4, domino(1, 1, 1, 2), Add, 5).unwrap();
        assert!(matches!(cage.get(Cell(9, 9)), Err(Error::MissingCell(_))));
    }

    #[test]
    fn get_given_returns_singleton_fill() {
        let cage = Cage::given(Cell(1, 1), 4, 3).unwrap();
        assert_eq!(cage.get(Cell(1, 1)).unwrap(), Fill::from(4, &[3]));
    }

    // ---- set ----

    #[test]
    fn set_missing_cell_returns_error() {
        let cage = Cage::commutative(4, domino(1, 1, 1, 2), Add, 5).unwrap();
        assert!(matches!(
            cage.set(Cell(9, 9), Fill::from(4, &[1])),
            Err(Error::MissingCell(_))
        ));
    }

    #[test]
    fn set_commutative_narrows_fills() {
        // add to 5 in n=4: (1,4),(2,3),(3,2),(4,1)
        // assign pos 0 = {1,2} → remaining tuples (1,4),(2,3) → pos 1 = {3,4}
        let cage = Cage::commutative(4, domino(1, 1, 1, 2), Add, 5).unwrap();
        let narrowed = cage.set(Cell(1, 1), Fill::from(4, &[1, 2])).unwrap();
        assert_eq!(narrowed.get(Cell(1, 1)).unwrap(), Fill::from(4, &[1, 2]));
        assert_eq!(narrowed.get(Cell(1, 2)).unwrap(), Fill::from(4, &[3, 4]));
    }

    #[test]
    fn set_commutative_empty_fill_returns_empty_fills() {
        // no tuple summing to 5 has pos 0 ∈ {}
        let cage = Cage::commutative(4, domino(1, 1, 1, 2), Add, 5).unwrap();
        assert!(matches!(
            cage.set(Cell(1, 1), Fill::from(4, &[])),
            Err(Error::EmptyFills)
        ));
    }

    #[test]
    fn set_widens_after_narrowing() {
        // narrow to pos 0 = {1}, then widen back to {1,2} — must restore full support
        // add to 5 in n=4: pos 0 = {1} → only (1,4) survives
        // widen to {1,2} → (1,4) and (2,3) both survive, pos 1 = {3,4}
        let cage = Cage::commutative(4, domino(1, 1, 1, 2), Add, 5).unwrap();
        let narrowed = cage.set(Cell(1, 1), Fill::from(4, &[1])).unwrap();
        assert_eq!(narrowed.get(Cell(1, 2)).unwrap(), Fill::from(4, &[4]));
        let widened = narrowed.set(Cell(1, 1), Fill::from(4, &[1, 2])).unwrap();
        assert_eq!(widened.get(Cell(1, 1)).unwrap(), Fill::from(4, &[1, 2]));
        assert_eq!(widened.get(Cell(1, 2)).unwrap(), Fill::from(4, &[3, 4]));
    }

    #[test]
    fn set_non_commutative_narrows_fills() {
        // subtract 1 in n=4: (1,2),(2,1),(2,3),(3,2),(3,4),(4,3)
        // assign pos 0 = {1} → remaining tuples (1,2) → pos 1 = {2}
        let cage = Cage::non_commutative(4, domino(1, 1, 1, 2), Subtract, 1).unwrap();
        let narrowed = cage.set(Cell(1, 1), Fill::from(4, &[1])).unwrap();
        assert_eq!(narrowed.get(Cell(1, 1)).unwrap(), Fill::from(4, &[1]));
        assert_eq!(narrowed.get(Cell(1, 2)).unwrap(), Fill::from(4, &[2]));
    }

    #[test]
    fn set_given_compatible_fill_succeeds() {
        let cage = Cage::given(Cell(1, 1), 4, 3).unwrap();
        assert!(cage.set(Cell(1, 1), Fill::from(4, &[2, 3])).is_ok());
    }

    #[test]
    fn set_given_incompatible_fill_returns_invalid_cage_fill() {
        let cage = Cage::given(Cell(1, 1), 4, 3).unwrap();
        assert!(matches!(
            cage.set(Cell(1, 1), Fill::from(4, &[1, 2])),
            Err(Error::InvalidCageFill(_, _))
        ));
    }
}
