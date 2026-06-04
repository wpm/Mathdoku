//! Traits for building and narrowing cage constraint representations.
//!
//! [`Memo`] constructs a representation of all value tuples satisfying a cage's
//! arithmetic constraint. [`Narrow`] filters that representation when external
//! information (e.g. from grid-level constraints) rules out certain values.
//!
//! Both traits are implemented by [`Table`](crate::mdk::table::Table), which
//! stores tuples explicitly, and will be implemented by `Mdd`, which stores
//! them as a multivalued decision diagram.
use crate::mdk::fill::Fill;
use crate::mdk::operation::{Commutative, NonCommutative};
use crate::mdk::{Error, Target};

/// A cage constraint representation that can be constructed from an arithmetic operation.
///
/// Implementors store the set of value tuples satisfying the constraint and
/// expose per-position candidate sets via [`fill`](Memo::fill).
pub trait Memo: Sized {
    /// Constructs a representation of all `k`-tuples of values in `1..=n`
    /// satisfying a commutative (add or multiply) constraint.
    ///
    /// # Errors
    /// Returns [`Error::EmptyFills`] if no tuples satisfy the constraint.
    fn commutative(
        n: usize,
        k: usize,
        operator: Commutative,
        target: Target,
    ) -> Result<Self, Error>;

    /// Constructs a representation of all pairs of values in `1..=n`
    /// satisfying a non-commutative (subtract or divide) constraint.
    ///
    /// # Errors
    /// Returns [`Error::EmptyFills`] if no tuples satisfy the constraint.
    fn non_commutative(n: usize, operator: NonCommutative, target: Target) -> Result<Self, Error>;

    /// Returns the candidate value set for position `index`.
    ///
    /// The candidate set is the union of values that appear at `index`
    /// across all tuples in the representation.
    ///
    /// # Errors
    /// Returns [`Error::IndexOutOfBounds`] if `index` is out of range.
    fn fill(&self, index: usize) -> Result<Fill, Error>;
}

/// A cage constraint representation that can be narrowed by restricting candidate values.
pub trait Narrow: Sized {
    /// Returns a new representation containing only the tuples where every
    /// position's value is present in the corresponding [`Fill`].
    ///
    /// # Errors
    /// Returns [`Error::EmptyFills`] if no tuples survive the filter.
    fn remove(&self, fills: Vec<Fill>) -> Result<Self, Error>;
}
