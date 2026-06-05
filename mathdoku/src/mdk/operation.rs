//! Arithmetic operators and cage operations for Mathdoku constraints.
use crate::mdk::{N, Target};
use std::cmp::{max, min};
use std::ops::Div;

/// An arithmetic operation paired with a target value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticConstraint {
    /// A commutative operation and a target.
    CommutativeConstraint(CommutativeOperator, Target),
    /// A non-commutative operation and a target.
    NonCommutativeConstraint(NonCommutativeOperator, Target),
}

/// A commutative, monotonically non-decreasing cage operation.
///
/// Because applying the operator to a longer tuple can only increase the result,
/// partial results can be used to prune the search for valid tuples.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum CommutativeOperator {
    Add,
    Multiply,
}
impl CommutativeOperator {
    /// Applies this operator to a tuple of values, returning the result.
    #[must_use]
    pub fn apply_to_tuple(&self, ns: &[N]) -> Target {
        match self {
            Self::Add => ns.iter().map(|&v| Target::from(v)).sum(),
            Self::Multiply => ns.iter().map(|&v| Target::from(v)).product(),
        }
    }

    // TODO Why isn't apply_to_pair a method in NonCommutativeOperator?
    /// Applies this operator to a single pair `(x, y)`.
    #[must_use]
    pub const fn apply_to_pair(self, x: Target, y: Target) -> Target {
        match self {
            Self::Add => x + y,
            Self::Multiply => x * y,
        }
    }

    /// Returns the identity element for this operator (`0` for add, `1` for multiply).
    ///
    /// Used as the per-slot minimum bound when pruning tuple search: a partial
    /// result extended by `remaining` copies of the dual identity gives the
    /// tightest reachable lower bound on the final result.
    #[must_use]
    pub const fn identity(&self) -> Target {
        match self {
            Self::Add => 0,
            Self::Multiply => 1,
        }
    }

    /// Returns the dual operator (`Multiply` for `Add`, `Add` for `Multiply`).
    ///
    /// The dual's identity is the minimum value each remaining slot can contribute,
    /// forming the ring relationship used in tuple pruning.
    #[must_use]
    pub const fn dual(&self) -> Self {
        match self {
            Self::Add => Self::Multiply,
            Self::Multiply => Self::Add,
        }
    }
}

/// A non-commutative cage operator whose result depends on operand order.
///
/// Applied to a pair `(a, b)` without regard to order — subtract uses absolute
/// difference and divide uses `max / min` — so the result is order-independent
/// even though the operator is not commutative in the algebraic sense.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum NonCommutativeOperator {
    Subtract,
    Divide,
}

impl NonCommutativeOperator {
    /// Applies this operator to `(a, b)`, returning the result.
    ///
    /// Subtract returns `|a - b|`. Divide returns `max(a, b) / min(a, b)`
    /// using integer division.
    #[must_use]
    pub fn apply(&self, a: N, b: N) -> Target {
        match self {
            Self::Subtract => Target::from(a.abs_diff(b)),
            Self::Divide => Target::from(max(a, b).div(min(a, b))),
        }
    }
}
