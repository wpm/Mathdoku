use std::fmt;
use std::fmt::{Display, Formatter};
use crate::mdk::{Target, N};

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