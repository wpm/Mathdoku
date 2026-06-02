//! MDD-based (multivalued decision diagram) implementation of [`Memo`].
use crate::mdk::Error;
use crate::mdk::fill::{Fill, Memo};
use crate::mdk::grid::Cell;
use crate::mdk::puzzle::Cage;
use std::collections::HashMap;

/// Monotonic cage-fill memo backed by an MDD.
///
/// Suitable for cages whose constraint has monotonic structure (e.g. addition, multiplication).
pub struct Mdd {}

#[allow(clippy::todo)]
impl Memo for Mdd {
    fn new(_n: usize, _cage: Cage) -> Self {
        todo!()
    }

    fn fill(&self, _cell: &Cell) -> Result<Fill, Error> {
        todo!()
    }

    fn remove(&self, _fills: HashMap<Cell, Fill>) -> Result<Self, Error> {
        todo!()
    }
}
