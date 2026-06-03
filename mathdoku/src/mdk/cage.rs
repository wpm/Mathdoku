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
