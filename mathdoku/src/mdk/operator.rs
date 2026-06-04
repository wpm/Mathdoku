use crate::mdk::{N, Target};
use std::fmt;
use std::fmt::{Display, Formatter};

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

/// Operators valid for monotonic cages (MDD-backed): addition and multiplication.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Commutative {
    Add,
    Multiply,
}

impl Commutative {
    /// Applies this operator to `ns`, returning the result.
    pub(crate) fn apply(self, ns: &[N]) -> Target {
        match self {
            Self::Add => ns.iter().sum(),
            Self::Multiply => ns.iter().product(),
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
