use crate::mdk::Error;
use crate::mdk::fill::Fill;
use crate::mdk::old_cage::Operation;
use crate::mdk::old_mdd::Mdd;
use crate::mdk::polyomino::{Cell, Polyomino};
use std::collections::HashMap;

/// Memo used to store intermediate results for cage operations.
/// Commutative operations use an `Mdd` while non-commutative operations use a domino table.
#[derive(Clone)]
pub enum CageMemo {
    Mdd(Mdd),
}

impl CageMemo {
    #[allow(clippy::todo)]
    fn new(_n: usize, _polyomino: &Polyomino, _operation: Operation) -> Self {
        todo!()
    }
}

/// Memoizes the candidate fills for each cell of a cage given its size and arithmetic operation.
pub trait Memo {
    /// Returns the candidate fill for `cell` within the cage.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingCell`] if `cell` is not part of the cage.
    fn fill(&self, cell: &Cell) -> Result<Fill, Error>;

    /// Returns a new memo with `fills` removed as candidates, propagating the constraint.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingCell`] if any cell in `fills` is not part of the cage.
    fn remove(&self, fills: HashMap<Cell, Fill>) -> Result<Self, Error>
    where
        Self: Sized;
}
