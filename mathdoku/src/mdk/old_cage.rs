//! [`Cage`] and the operator types used to construct one.
use crate::mdk::old_memo::CageMemo;
use crate::mdk::operator::Operator;
use crate::mdk::polyomino::{Cell, Polyomino};
use crate::mdk::{N, Target};
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter};

/// A polyomino paired with an arithmetic operation and its memoized candidate fills.
#[must_use]
#[derive(Clone)]
pub struct Cage {
    /// The cells covered by this cage.
    pub polyomino: Polyomino,
    /// The arithmetic constraint that the cage's cell values must satisfy.
    operation: Operation,
    /// Memoization of the possible [`Fill`]s.
    pub memo: Option<CageMemo>,
}

impl Cage {
    /// Creates a new `operation` cage in `polyomino` on a [`Grid`] of size `n`.
    pub fn new(_n: N, polyomino: Polyomino, operation: Operation) -> Self {
        // todo!("Create the appropriate memo for the operator.");
        match operation.0 {
            Operator::Add => {}
            Operator::Multiply => {}
            Operator::Subtract => {}
            Operator::Divide => {}
            Operator::Given => {}
        }
        Self {
            polyomino,
            operation,
            memo: None,
        }
    }
}

impl std::fmt::Debug for Cage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cage")
            .field("polyomino", &self.polyomino)
            .field("operation", &self.operation.to_string())
            .finish_non_exhaustive()
    }
}

impl PartialEq for Cage {
    fn eq(&self, other: &Self) -> bool {
        self.polyomino == other.polyomino && self.operation == other.operation
    }
}

impl Eq for Cage {}

impl IntoIterator for Cage {
    type Item = Cell;
    type IntoIter = std::collections::btree_set::IntoIter<Cell>;

    fn into_iter(self) -> Self::IntoIter {
        self.polyomino.into_iter()
    }
}

impl PartialOrd for Cage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Cage {
    fn cmp(&self, other: &Self) -> Ordering {
        self.polyomino
            .cmp(&other.polyomino)
            .then_with(|| self.operation.cmp(&other.operation))
    }
}

/// An operator paired with a target value.
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone)]
pub struct Operation(
    /// The arithmetic operator.
    pub Operator,
    /// The target value the operator must produce.
    pub Target,
);

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0, self.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdk::polyomino::{Cell, Polyomino};
    use std::cmp::Ordering;

    fn singleton() -> Polyomino {
        Polyomino::from([Cell(1, 1)]).unwrap()
    }

    fn pair() -> Polyomino {
        Polyomino::from([Cell(1, 1), Cell(1, 2)]).unwrap()
    }

    fn col_pair() -> Polyomino {
        Polyomino::from([Cell(1, 1), Cell(2, 1)]).unwrap()
    }

    fn l_shape() -> Polyomino {
        Polyomino::from([Cell(1, 1), Cell(1, 2), Cell(2, 1)]).unwrap()
    }

    fn cage(polyomino: Polyomino, operator: Operator, target: Target) -> Cage {
        Cage::new(4, polyomino, Operation(operator, target))
    }

    // --- Cage::new ---

    #[test]
    fn given_singleton_succeeds() {
        let c = cage(singleton(), Operator::Given, 3);
        assert_eq!(c.polyomino, singleton());
    }

    #[test]
    fn add_pair_succeeds() {
        let c = cage(pair(), Operator::Add, 3);
        assert_eq!(c.polyomino, pair());
    }

    #[test]
    fn subtract_pair_succeeds() {
        let c = cage(pair(), Operator::Subtract, 1);
        assert_eq!(c.polyomino, pair());
    }

    // --- equality and ordering ---

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
    fn ordering_follows_polyomino_then_operation() {
        let a = cage(pair(), Operator::Add, 3);
        let b = cage(pair(), Operator::Add, 3);
        let c = cage(col_pair(), Operator::Add, 3);
        assert_eq!(a.cmp(&b), Ordering::Equal);
        assert_ne!(a.cmp(&c), Ordering::Equal);
    }

    // --- cells ---

    // --- IntoIterator ---

    #[test]
    fn into_iter_yields_cells_in_order() {
        let c = cage(pair(), Operator::Add, 3);
        let cells: Vec<Cell> = c.into_iter().collect();
        assert_eq!(cells, vec![Cell(1, 1), Cell(1, 2)]);
    }

    #[test]
    fn into_iter_singleton_yields_one_cell() {
        let c = cage(singleton(), Operator::Given, 5);
        let cells: Vec<Cell> = c.into_iter().collect();
        assert_eq!(cells, vec![Cell(1, 1)]);
    }

    #[test]
    fn into_iter_l_shape_yields_cells_in_row_major_order() {
        let c = cage(l_shape(), Operator::Add, 6);
        let cells: Vec<Cell> = c.into_iter().collect();
        assert_eq!(cells, vec![Cell(1, 1), Cell(1, 2), Cell(2, 1)]);
    }

    // --- Display ---

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
        assert_eq!(Operation(Operator::Add, 12).to_string(), "+ 12");
        assert_eq!(Operation(Operator::Subtract, 3).to_string(), "− 3");
        assert_eq!(Operation(Operator::Multiply, 24).to_string(), "× 24");
        assert_eq!(Operation(Operator::Divide, 2).to_string(), "÷ 2");
    }

    #[test]
    fn operation_display_given_has_no_symbol() {
        assert_eq!(Operation(Operator::Given, 7).to_string(), " 7");
    }
}
