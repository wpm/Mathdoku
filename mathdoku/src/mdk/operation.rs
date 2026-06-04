//! Arithmetic operators and cage operations for Mathdoku constraints.
use crate::mdk::cage::Memo;
use crate::mdk::shape::Polyomino;
use crate::mdk::{N, Target};
use std::cmp::{max, min};
use std::ops::Div;

/// The arithmetic constraint applied to a cage's cell values.
#[derive(Clone, Copy)]
pub enum CageOperation {
    /// A commutative operation (add or multiply) with a target value.
    Commutative(Commutative, Target),
    /// A non-commutative operation (subtract or divide) with a target value.
    NonCommutative(NonCommutative, Target),
    /// A single cell whose value is given directly.
    Given(N),
}

impl CageOperation {
    fn memo(&self, _n: usize, _polyomino: &Polyomino) -> Option<Memo> {
        todo!()
    }
}

/// An arithmetic operation paired with a target value.
#[derive(Clone, Copy)]
pub enum ArithmeticOperation {
    /// A commutative (monotonic) operation: add or multiply.
    Commutative(Commutative, Target),
    /// A non-commutative (non-monotonic) operation: subtract or divide.
    NonCommutative(NonCommutative, Target),
}

/// A commutative, monotonically non-decreasing cage operator.
///
/// Because applying the operator to a longer tuple can only increase the result,
/// partial results can be used to prune the search for valid tuples.
#[derive(Clone, Copy)]
pub enum Commutative {
    Add,
    Multiply,
}

impl Commutative {
    /// Applies this operator to `ns`, returning the result.
    pub fn apply(&self, ns: &[N]) -> Target {
        match self {
            Self::Add => ns.iter().sum(),
            Self::Multiply => ns.iter().product(),
        }
    }

    /// Returns the identity element for this operator (`0` for add, `1` for multiply).
    ///
    /// Used as the per-slot minimum bound when pruning tuple search: a partial
    /// result extended by `remaining` copies of the dual identity gives the
    /// tightest reachable lower bound on the final result.
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
#[derive(Clone, Copy)]
pub enum NonCommutative {
    Subtract,
    Divide,
}

impl NonCommutative {
    /// Applies this operator to `(a, b)`, returning the result.
    ///
    /// Subtract returns `|a - b|`. Divide returns `max(a, b) / min(a, b)`
    /// using integer division.
    pub fn apply(&self, a: N, b: N) -> Target {
        match self {
            Self::Subtract => a.abs_diff(b),
            Self::Divide => max(a, b).div(min(a, b)),
        }
    }
}
