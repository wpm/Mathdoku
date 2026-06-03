//! Candidate value sets and the [`Memo`] trait for cage-fill memoization.
use crate::mdk::grid::Cell;
use crate::mdk::{Error, N};
use itertools::Itertools;
use std::collections::{BTreeSet, HashMap};
use std::fmt::{Display, Formatter};

/// The set of candidate values for a cell.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct Fill(BTreeSet<N>);

impl Fill {
    /// Creates a full candidate set `{1..=n}`.
    pub(crate) fn new(n: usize) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        Self((1..=n as N).collect())
    }

    /// Creates a candidate set from an explicit slice of values.
    pub(crate) fn from(n: &[N]) -> Self {
        Self(n.iter().copied().collect())
    }
}

impl Display for Fill {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}}}", self.0.iter().join(", "))
    }
}

/// Memoizes the candidate fills for each cell of a cage given its size and arithmetic operation.
pub trait Memo {
    /// Returns the candidate fill for `cell` within the cage.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCell`] if `cell` is not part of the cage.
    fn fill(&self, cell: &Cell) -> Result<Fill, Error>;

    /// Returns a new memo with `fills` removed as candidates, propagating the constraint.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCell`] if any cell in `fills` is not part of the cage.
    fn remove(&self, fills: HashMap<Cell, Fill>) -> Result<Self, Error>
    where
        Self: Sized;
}
