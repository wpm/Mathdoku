use crate::mdk::cage::Memo;
use crate::mdk::shape::Polyomino;
use crate::mdk::{N, Target};
use std::ops::Div;

// An operator and a [`Target`] value or a single cell with a specified value.
#[derive(Clone, Copy)]
pub enum CageOperation {
    Monotonic(Commutative, Target),
    NonMonotonic(NonCommutative, Target),
    Given(N),
}

impl CageOperation {
    fn memo(&self, _n: usize, _polyomino: &Polyomino) -> Option<Memo> {
        todo!()
    }
}

// The operator classes — these are enums of actual operators:
#[derive(Clone, Copy)]
pub enum Commutative {
    Add,
    Multiply,
}

impl Commutative {
    pub fn apply(&self, ns: Vec<N>) -> Target {
        match self {
            Self::Add => ns.iter().sum(),
            Self::Multiply => ns.iter().product(),
        }
    }
}

#[derive(Clone, Copy)]
pub enum NonCommutative {
    Subtract,
    Divide,
}

impl NonCommutative {
    pub fn apply(&self, a: N, b: N) -> Target {
        match self {
            Self::Subtract => a.abs_diff(b),
            Self::Divide => a.div(b),
        }
    }
}
