//! A [`Cage`]: a polyomino with an arithmetic constraint.
//!
//! A cage combines a polyomino (the set of cells it covers) with an
//! [`Operation`] (an [`Operator`] and numeric target).

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::hash::Hash;

use crate::Error::InfeasibleCage;
use crate::mdd::MonotonicMDD;
use crate::operation::{Operation, Operator};
use crate::polyomino::Polyomino;
use crate::{Cell, Error, operators_for};

/// A polyomino with an [`Operation`] constraining its cell values.
///
/// The `mdd` field caches the pre-built [`MonotonicMDD`] for Add/Multiply cages
/// once the grid size `n` is known. It is skipped during serialization and
/// deserialization; callers that need it must call [`Cage::build_mdd`] first.
#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cage {
    polyomino: Polyomino,
    operation: Operation,
    /// Pre-built MDD for Add/Multiply cages. Skipped by serde; does not
    /// contribute to equality, ordering, or hashing.
    #[serde(skip)]
    mdd: Option<MonotonicMDD>,
}

impl PartialEq for Cage {
    fn eq(&self, other: &Self) -> bool {
        self.polyomino == other.polyomino && self.operation == other.operation
    }
}

impl Eq for Cage {}

impl Hash for Cage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.polyomino.hash(state);
        self.operation.hash(state);
    }
}

impl Cage {
    /// Creates a cage from a polyomino and an operation.
    ///
    /// # Errors
    /// Returns [`InfeasibleCage`] if the operator is not valid for the polyomino's
    /// size.
    pub fn new(polyomino: Polyomino, operation: Operation) -> Result<Self, Error> {
        if !operators_for(&polyomino).contains(&operation.operator()) {
            return Err(InfeasibleCage(polyomino, operation));
        }
        Ok(Self {
            polyomino,
            operation,
            mdd: None,
        })
    }

    /// Populates the cached MDD for this cage given grid size `n`, and returns
    /// a reference to it. Returns `None` for non-monotonic operators (Given,
    /// Subtract, Divide), which use brute-force enumeration instead.
    #[allow(clippy::cast_possible_truncation)]
    pub fn build_mdd(&mut self, n: usize) -> Option<&MonotonicMDD> {
        use crate::mdd::{Constraint, MonotonicConstraint};
        let op = self.operation();
        let arity = self.cells().len() as u32;
        let target = op.target as u32;
        let constraint = match op.operator() {
            Operator::Add => MonotonicConstraint::Sum(Constraint { target, arity }),
            Operator::Multiply => MonotonicConstraint::Product(Constraint { target, arity }),
            _ => return None,
        };
        self.mdd = Some(MonotonicMDD::new(n as u32, constraint));
        self.mdd.as_ref()
    }

    /// Returns the cached MDD, or `None` if [`build_mdd`] has not been called
    /// or this cage uses a non-monotonic operator.
    ///
    /// [`build_mdd`]: Cage::build_mdd
    #[must_use]
    pub fn mdd(&self) -> Option<&MonotonicMDD> {
        self.mdd.as_ref()
    }

    /// Returns the cells covered by this cage.
    #[must_use]
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

    /// Does the cage contain the given cell?
    #[must_use]
    pub fn contains(&self, cell: Cell) -> bool {
        self.polyomino.contains(cell)
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
mod tests {
    use super::*;
    use crate::test_utils::{col_pair, l_shape, pair, singleton};
    use crate::{Operator, Target};

    fn cage(polyomino: Polyomino, operator: Operator, target: Target) -> Cage {
        Cage::new(polyomino, Operation::new(operator, target)).unwrap()
    }

    // --- Cage::new ---

    #[test]
    fn given_singleton_succeeds() {
        assert!(Cage::new(singleton(), Operation::new(Operator::Given, 3)).is_ok());
    }

    #[test]
    fn add_pair_succeeds() {
        assert!(Cage::new(pair(), Operation::new(Operator::Add, 3)).is_ok());
    }

    #[test]
    fn subtract_pair_succeeds() {
        assert!(Cage::new(pair(), Operation::new(Operator::Subtract, 1)).is_ok());
    }

    #[test]
    fn divide_non_pair_returns_infeasible() {
        // Divide is only valid for exactly two cells.
        assert!(matches!(
            Cage::new(l_shape(), Operation::new(Operator::Divide, 2)),
            Err(InfeasibleCage(_, _))
        ));
    }

    #[test]
    fn subtract_non_pair_returns_infeasible() {
        assert!(matches!(
            Cage::new(l_shape(), Operation::new(Operator::Subtract, 1)),
            Err(InfeasibleCage(_, _))
        ));
    }

    #[test]
    fn cells_returns_polyomino_cells() {
        let c = cage(pair(), Operator::Add, 3);
        assert_eq!(c.cells(), pair().cells());
    }

    #[test]
    fn operation_roundtrips() {
        let op = Operation::new(Operator::Multiply, 6);
        let c = Cage::new(pair(), op.clone()).unwrap();
        assert_eq!(c.operation(), op);
    }

    #[test]
    fn polyomino_roundtrips() {
        let c = cage(l_shape(), Operator::Add, 6);
        assert_eq!(c.polyomino(), &l_shape());
    }

    #[test]
    fn equality_depends_on_polyomino_and_operation() {
        let a = cage(pair(), Operator::Add, 3);
        let b = cage(pair(), Operator::Add, 3);
        let c = cage(pair(), Operator::Add, 4);
        let d = cage(col_pair(), Operator::Add, 3);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn ordering_follows_polyomino() {
        // pair() < col_pair() if polyomino ordering says so; at minimum Ord is consistent.
        let a = cage(pair(), Operator::Add, 3);
        let b = cage(pair(), Operator::Multiply, 6);
        // Same polyomino → equal ordering regardless of operation.
        assert_eq!(a.cmp(&b), Ordering::Equal);
    }

    #[test]
    fn cage_roundtrips_through_json() {
        let original = cage(l_shape(), Operator::Add, 6);
        let json = serde_json::to_string(&original).unwrap();
        let restored: Cage = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn operator_display() {
        assert_eq!(Operator::Add.to_string(), "+");
        assert_eq!(Operator::Subtract.to_string(), "−");
        assert_eq!(Operator::Multiply.to_string(), "×");
        assert_eq!(Operator::Divide.to_string(), "÷");
        assert_eq!(Operator::Given.to_string(), "");
    }

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
