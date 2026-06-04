use crate::mdk::fill::Fill;
use crate::mdk::operation::{Commutative, NonCommutative};
use crate::mdk::{Error, Target};

pub trait Memo: Sized {
    /// # Errors
    /// Returns [`Error::EmptyFills`] if no tuples satisfy the constraint.
    fn commutative(
        n: usize,
        k: usize,
        operator: Commutative,
        target: Target,
    ) -> Result<Self, Error>;
    /// # Errors
    /// Returns [`Error::EmptyFills`] if no tuples satisfy the constraint.
    fn non_commutative(n: usize, operator: NonCommutative, target: Target) -> Result<Self, Error>;
    /// # Errors
    /// Returns [`Error::IndexOutOfBounds`] if `index` exceeds the number of positions.
    fn fill(&self, index: usize) -> Result<Fill, Error>;
}

pub trait Narrow: Sized {
    /// # Errors
    /// Returns [`Error::EmptyFills`] if filtering leaves no valid tuples.
    fn remove(&self, fills: Vec<Fill>) -> Result<Self, Error>;
}
