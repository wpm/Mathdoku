//! Trie-based implementation of [`Memo`].
use crate::mdk::Error;
use crate::mdk::Target;
use crate::mdk::cage::NonMonotonicOp;
use crate::mdk::fill::{Fill, Memo};
use crate::mdk::grid::{Cell, Polyomino};
use std::collections::HashMap;

/// Non-monotonic cage-fill memo backed by a trie.
///
/// Suitable for cages whose constraint is non-monotonic (e.g. subtraction, division).
pub struct Trie {}

impl Trie {
    /// Creates a trie memo for `polyomino` with the non-monotonic `op` and `target` on a grid of size `n`.
    #[allow(clippy::todo)]
    pub fn new(_n: usize, _polyomino: &Polyomino, _op: NonMonotonicOp, _target: Target) -> Self {
        todo!()
    }
}

#[allow(clippy::todo)]
impl Memo for Trie {
    fn fill(&self, _cell: &Cell) -> Result<Fill, Error> {
        todo!()
    }

    fn remove(&self, _fills: HashMap<Cell, Fill>) -> Result<Self, Error> {
        todo!()
    }
}
