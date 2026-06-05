//! Cage representation pairing a polyomino with its arithmetic constraint.
//!
//! # Invariant: complete tuple support
//!
//! A `Cage` always stores the *complete* set of value tuples consistent with
//! both its arithmetic constraint and the current per-cell candidate fills.
//! Concretely: if a cell's candidate fill is `{a, b}`, then every tuple in
//! the backing memo has a value in `{a, b}` at that position, and every value
//! in `{a, b}` appears at that position in at least one tuple.
//!
//! This means [`Cage::set`] always recalculates from the full original tuple
//! set rather than narrowing incrementally. Widening a cell's fill therefore
//! restores tuples that were previously excluded, and narrowing it removes them.
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
//! the valid tuples, compressed by sharing common prefixes and suffixes.
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
use crate::mdk::csp::Constraint;
use crate::mdk::fill::Fill;
use crate::mdk::grid::Grid;
use crate::mdk::mdd::Mdd;
use crate::mdk::memo::Memo;
use crate::mdk::operation::{CommutativeOperator, NonCommutativeOperator};
use crate::mdk::polyomino::{Cell, Polyomino};
use crate::mdk::table::Table;
use crate::mdk::{Error, Error::EmptyFills, N, T};

/// The constraint for a cage and its backing memo.
#[derive(Clone, PartialEq, Eq, Debug)]
enum CageOperation {
    /// A commutative (monotonic) operation: add or multiply.
    Commutative(CommutativeOperator, T, Mdd),
    /// A non-commutative (non-monotonic) operation: subtract or divide.
    NonCommutative(NonCommutativeOperator, T, Table),
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
    /// Returns [`EmptyFills`] if no tuples satisfy the constraint.
    pub fn commutative(
        n: usize,
        polyomino: Polyomino,
        operation: CommutativeOperator,
        target: T,
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
    /// Returns [`EmptyFills`] if no pairs satisfy the constraint.
    pub fn non_commutative(
        n: usize,
        polyomino: Polyomino,
        operation: NonCommutativeOperator,
        target: T,
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
            CageOperation::Given(n) => Fill::from(&[*n]),
        };
        Ok(fill)
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

impl Constraint<Grid, Cell, Fill, Error> for Cage {
    fn propagate(&self, state: &Grid) -> Result<(Grid, Vec<Cell>), Error> {
        let cells: Vec<Cell> = self.polyomino.iter().copied().collect();
        let old_fills: Vec<Fill> = cells
            .iter()
            .map(|&c| state.get(c))
            .collect::<Result<_, _>>()?;
        let new_fills = match &self.operation {
            CageOperation::Given(n) => {
                // Singleton cell: fill is always the fixed value, intersected with current state.
                let singleton = Fill::from(&[*n]);
                vec![if old_fills[0].contains(*n) {
                    singleton
                } else {
                    Fill::default()
                }]
            }
            CageOperation::Commutative(_, _, memo) => match memo.narrow(old_fills.clone()) {
                Ok(narrowed) => (0..cells.len())
                    .map(|i| narrowed.get(i).unwrap_or_default())
                    .collect(),
                Err(EmptyFills) => vec![Fill::default(); cells.len()],
                Err(e) => return Err(e),
            },
            CageOperation::NonCommutative(_, _, memo) => match memo.narrow(old_fills.clone()) {
                Ok(narrowed) => (0..cells.len())
                    .map(|i| narrowed.get(i).unwrap_or_default())
                    .collect(),
                Err(EmptyFills) => vec![Fill::default(); cells.len()],
                Err(e) => return Err(e),
            },
        };
        Ok(state.apply_fills(&cells, &old_fills, new_fills))
    }

    fn in_scope(&self, variable: Cell) -> bool {
        self.polyomino.contains(&variable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdk::grid::Grid;
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
            Err(EmptyFills)
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
            Err(EmptyFills)
        ));
    }

    #[test]
    fn non_commutative_stores_polyomino() {
        let poly = domino(2, 1, 2, 2);
        let cage = Cage::non_commutative(4, poly.clone(), Subtract, 1).unwrap();
        assert_eq!(cage.polyomino, poly);
    }

    // ---- Constraint::propagate ----

    fn full_grid(n: usize) -> Grid {
        Grid::new(n)
    }

    #[test]
    fn cage_propagate_given_pins_cell() {
        let cage = Cage::given(Cell(1, 1), 4, 3).unwrap();
        let (new_g, changed) = cage.propagate(&full_grid(4)).unwrap();
        assert_eq!(new_g.get(Cell(1, 1)).unwrap(), Fill::from(&[3]));
        assert_eq!(changed, vec![Cell(1, 1)]);
    }

    #[test]
    fn cage_propagate_add_prunes_impossible_values() {
        // Add 3 in a 4×4: valid pairs summing to 3 are (1,2),(2,1) — only values {1,2}
        let cage = Cage::commutative(4, domino(1, 1, 1, 2), Add, 3).unwrap();
        let (new_g, _) = cage.propagate(&full_grid(4)).unwrap();
        assert_eq!(new_g.get(Cell(1, 1)).unwrap(), Fill::from(&[1, 2]));
        assert_eq!(new_g.get(Cell(1, 2)).unwrap(), Fill::from(&[1, 2]));
    }

    #[test]
    fn cage_propagate_cross_cell_add_prunes_partner() {
        // Add 5 in 4×4: valid pairs are (1,4),(2,3),(3,2),(4,1).
        // Pin cell A to {4}: only (4,1) survives, so B must narrow to {1}.
        let cage = Cage::commutative(4, domino(1, 1, 1, 2), Add, 5).unwrap();
        let g = full_grid(4).set(Cell(1, 1), Fill::from(&[4]));
        let (new_g, changed) = cage.propagate(&g).unwrap();
        assert_eq!(new_g.get(Cell(1, 2)).unwrap(), Fill::from(&[1]));
        assert!(changed.contains(&Cell(1, 2)));
    }

    #[test]
    fn cage_propagate_cross_cell_subtract_prunes_partner() {
        // Subtract 3 in 4×4: only valid pair is (4,1).
        // Pin cell A to {4}: B must narrow to {1}.
        let cage = Cage::non_commutative(4, domino(1, 1, 1, 2), Subtract, 3).unwrap();
        let g = full_grid(4).set(Cell(1, 1), Fill::from(&[4]));
        let (new_g, _) = cage.propagate(&g).unwrap();
        assert_eq!(new_g.get(Cell(1, 2)).unwrap(), Fill::from(&[1]));
    }

    #[test]
    fn cage_propagate_no_valid_tuple_empties_values() {
        // Grid has both cells pinned to {4}; Add 3 has no tuple (4,?) summing to 3
        let g = full_grid(4)
            .set(Cell(1, 1), Fill::from(&[4]))
            .set(Cell(1, 2), Fill::from(&[4]));
        let cage = Cage::commutative(4, domino(1, 1, 1, 2), Add, 3).unwrap();
        let (new_g, changed) = cage.propagate(&g).unwrap();
        assert!(new_g.get(Cell(1, 1)).unwrap().is_empty());
        assert!(new_g.get(Cell(1, 2)).unwrap().is_empty());
        assert_eq!(changed.len(), 2);
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
        assert_eq!(cage.get(Cell(1, 1)).unwrap(), Fill::from(&[3]));
    }
}
