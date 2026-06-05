//! Traits for building and narrowing cage constraint representations.
//!
//! [`Lookup`] constructs a representation of all value tuples satisfying a cage's
//! arithmetic constraint. [`Narrow`] filters that representation when external
//! information (e.g. from grid-level constraints) rules out certain values.
//!
//! Both traits are implemented by [`Table`](crate::mdk::table::Table), which
//! stores tuples explicitly, and will be implemented by `Mdd`, which stores
//! them as a multivalued decision diagram.
use crate::mdk::Error::{self, EmptyFills};
use crate::mdk::fill::Fill;
use crate::mdk::tuples::Tuple;

/// A cage constraint representation that can be constructed from an arithmetic operation.
///
/// Implementors store the set of value tuples satisfying the constraint and
/// expose per-position candidate sets via [`fill`](Lookup::fill).
pub trait Lookup {
    /// Returns the candidate value set for position `index`.
    ///
    /// The candidate set is the union of values that appear at `index`
    /// across all tuples in the representation.
    ///
    /// # Errors
    /// Returns [`Error::IndexOutOfBounds`] if `index` is out of range.
    fn fill(&self, index: usize) -> Result<Fill, Error>;
}

/// Derives per-position fills from a non-empty tuple list.
///
/// Returns `Err(EmptyFills)` if `tuples` is empty or any column's fill is empty.
pub(crate) fn fills_from_tuples(tuples: &[Tuple]) -> Result<Vec<Fill>, Error> {
    if tuples.is_empty() {
        return Err(EmptyFills);
    }
    let k = tuples[0].len();
    let fills: Vec<Fill> = (0..k)
        .map(|i| Fill::from(&tuples.iter().map(|t| t[i]).collect::<Tuple>()))
        .collect();
    if fills.iter().any(Fill::is_empty) {
        return Err(EmptyFills);
    }
    Ok(fills)
}

/// A cage constraint representation that can be narrowed by restricting candidate values.
pub trait Narrow: Sized {
    /// Returns a new representation containing only the tuples where every
    /// position's value is present in the corresponding [`Fill`].
    ///
    /// # Errors
    /// Returns [`EmptyFills`] if no tuples survive the filter.
    fn remove(&self, fills: Vec<Fill>) -> Result<Self, Error>;
}
