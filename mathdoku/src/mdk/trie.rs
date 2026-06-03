//! Trie-based implementation of [`Memo`].
use crate::mdk::Error;
use crate::mdk::cage::Operation;
use crate::mdk::fill::{Fill, Memo};
use crate::mdk::grid::{Cell, Polyomino};
use std::collections::HashMap;

/// Non-monotonic cage-fill memo backed by a trie.
///
/// Suitable for cages whose constraint is non-monotonic (e.g. subtraction, division).
pub struct Trie {}

#[allow(clippy::todo)]
impl Memo for Trie {
    fn new(_n: usize, _polyomino: &Polyomino, _operation: &Operation) -> Self {
        todo!()
    }

    fn fill(&self, _cell: &Cell) -> Result<Fill, Error> {
        todo!()
    }

    fn remove(&self, _fills: HashMap<Cell, Fill>) -> Result<Self, Error> {
        todo!()
    }
}
