use crate::mdk::Error;
use crate::mdk::domino_table::DominoTable;
use crate::mdk::fill::Fill;
use crate::mdk::grid::{Cell, Polyomino};
use crate::mdk::mdd::Mdd;
use crate::mdk::operator::{Arithmetic, NonCommutative};
use std::collections::HashMap;

#[derive(Clone)]
pub enum CageMemo {
    DominoTable(DominoTable),
    Mdd(Mdd),
}

impl CageMemo {
    fn new(n: usize, polyomino: &Polyomino, operation: Arithmetic) -> Self {
        match operation.0 {
            NonCommutative => DominoTable::new(n, polyomino, operation),
            Commutative => CageMemo::DominoTable(DominoTable::new(n)),
        }
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
