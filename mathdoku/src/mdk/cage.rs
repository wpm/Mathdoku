//! [`Cage`] and the operator types used to construct one.
use crate::mdk::domino_memo::DominoMemo;
use crate::mdk::fill::Memo;
use crate::mdk::grid::Polyomino;
use crate::mdk::mdd::Mdd;
use crate::mdk::{N, Target};
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter};

/// A polyomino paired with an arithmetic operation and its memoized candidate fills.
pub struct Cage {
    /// The cells that make up this cage.
    pub polyomino: Polyomino,
    /// The arithmetic constraint that the cage's cell values must satisfy.
    operation: Operation,
    /// `None` for `Given` cages, which have no arithmetic constraint to memoize.
    pub memo: Option<Box<dyn Memo>>,
}

impl Cage {
    /// Creates a new cage for `polyomino` with `operation` on a grid of size `n`.
    ///
    /// # Panics
    ///
    /// Panics if `operation` is subtract or divide and `polyomino` is not a domino (2 cells).
    #[must_use]
    pub fn new(n: usize, polyomino: Polyomino, operation: Operation) -> Self {
        let memo: Option<Box<dyn Memo>> = match operation.0 {
            Operator::Add => Some(Box::new(Mdd::new(
                n,
                &polyomino,
                Commutative::Add,
                operation.1,
            ))),
            Operator::Multiply => Some(Box::new(Mdd::new(
                n,
                &polyomino,
                Commutative::Multiply,
                operation.1,
            ))),
            Operator::Subtract => Some(Box::new(
                #[allow(clippy::expect_used)]
                DominoMemo::new(n, &polyomino, NonCommutative::Subtract, operation.1)
                    .expect("subtract cage must be a domino"),
            )),
            Operator::Divide => Some(Box::new(
                #[allow(clippy::expect_used)]
                DominoMemo::new(n, &polyomino, NonCommutative::Divide, operation.1)
                    .expect("divide cage must be a domino"),
            )),
            Operator::Given => None,
        };
        Self {
            polyomino,
            operation,
            memo,
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

/// Operators valid for domino cages: all except `Given`.
pub enum Arithmetic {
    Add,
    Multiply,
    Subtract,
    Divide,
}

/// Operators valid for monotonic cages (MDD-backed): addition and multiplication.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Commutative {
    Add,
    Multiply,
}

impl Commutative {
    /// Applies this operator to `values`, returning the result.
    pub(crate) fn apply(self, values: &[N]) -> Target {
        match self {
            Self::Add => values.iter().sum(),
            Self::Multiply => values.iter().product(),
        }
    }
}

/// Operators valid for non-monotonic binary cages: subtraction and division.
#[derive(Copy, Clone)]
pub enum NonCommutative {
    Subtract,
    Divide,
}

impl NonCommutative {
    /// Applies this operator to the pair `(x, y)`, returning the result.
    pub(crate) const fn apply(self, x: N, y: N) -> Target {
        match self {
            Self::Subtract => x.abs_diff(y),
            Self::Divide => x / y,
        }
    }
}

/// The arithmetic operator applied to a cage's cell values.
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone)]
pub enum Operator {
    /// Sum of all cell values equals the target.
    Add,
    /// Product of all cell values equals the target.
    Multiply,
    /// Absolute difference of the two cell values equals the target.
    Subtract,
    /// Quotient of the larger cell value divided by the smaller equals the target.
    Divide,
    /// Cell value is given directly; no arithmetic constraint.
    Given,
}

impl Display for Operator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Add => "+",
            Self::Subtract => "−",
            Self::Multiply => "×",
            Self::Divide => "÷",
            Self::Given => "",
        };
        write!(f, "{s}")
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
    use crate::mdk::grid::{Cell, Polyomino};
    use std::cmp::Ordering;

    fn singleton() -> Polyomino {
        Polyomino::from_cells([Cell::new(1, 1)])
    }

    fn pair() -> Polyomino {
        Polyomino::from_cells([Cell::new(1, 1), Cell::new(1, 2)])
    }

    fn col_pair() -> Polyomino {
        Polyomino::from_cells([Cell::new(1, 1), Cell::new(2, 1)])
    }

    fn l_shape() -> Polyomino {
        Polyomino::from_cells([Cell::new(1, 1), Cell::new(1, 2), Cell::new(2, 1)])
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

    // --- Cage::memo / fill ---

    #[test]
    fn add_memo_returns_non_empty_fill() {
        let c = cage(pair(), Operator::Add, 3);
        let fill = c.memo.as_ref().unwrap().fill(&Cell::new(1, 1)).unwrap();
        assert!(!fill.is_empty());
    }

    #[test]
    fn subtract_memo_returns_non_empty_fill() {
        let c = cage(pair(), Operator::Subtract, 1);
        let fill = c.memo.as_ref().unwrap().fill(&Cell::new(1, 1)).unwrap();
        assert!(!fill.is_empty());
    }

    #[test]
    fn divide_memo_returns_non_empty_fill() {
        let c = cage(pair(), Operator::Divide, 2);
        let fill = c.memo.as_ref().unwrap().fill(&Cell::new(1, 1)).unwrap();
        assert!(!fill.is_empty());
    }

    #[test]
    fn multiply_memo_returns_non_empty_fill() {
        let c = cage(pair(), Operator::Multiply, 6);
        let fill = c.memo.as_ref().unwrap().fill(&Cell::new(1, 1)).unwrap();
        assert!(!fill.is_empty());
    }

    #[test]
    fn given_has_no_memo() {
        let c = cage(singleton(), Operator::Given, 3);
        assert!(c.memo.is_none());
    }

    #[test]
    fn infeasible_add_gives_empty_fill() {
        let c = cage(pair(), Operator::Add, 99);
        let fill = c.memo.as_ref().unwrap().fill(&Cell::new(1, 1)).unwrap();
        assert!(fill.is_empty());
    }

    #[test]
    fn infeasible_subtract_gives_empty_fill() {
        let c = cage(pair(), Operator::Subtract, 9);
        let fill = c.memo.as_ref().unwrap().fill(&Cell::new(1, 1)).unwrap();
        assert!(fill.is_empty());
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

    #[test]
    fn cells_returns_polyomino_cells() {
        let c = cage(pair(), Operator::Add, 3);
        assert_eq!(c.polyomino.cells(), pair().cells());
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

    // --- l_shape cage ---

    #[test]
    fn add_l_shape_memo_returns_non_empty_fill() {
        let c = cage(l_shape(), Operator::Add, 6);
        let fill = c.memo.as_ref().unwrap().fill(&Cell::new(1, 1)).unwrap();
        assert!(!fill.is_empty());
    }
}
