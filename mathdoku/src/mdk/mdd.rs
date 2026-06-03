//! MDD-based (multivalued decision diagram) implementation of [`Memo`].
use crate::mdk::Error;
use crate::mdk::Target;
use crate::mdk::cage::Commutative;
use crate::mdk::fill::{Fill, Memo};
use crate::mdk::grid::{Cell, Polyomino};
use std::collections::HashMap;

/// Monotonic cage-fill memo backed by an MDD.
///
/// Suitable for cages whose constraint has monotonic structure (e.g. addition, multiplication).
pub struct Mdd {}

impl Mdd {
    /// Creates an MDD memo for `polyomino` with the monotonic `op` and `target` on a grid of size `n`.
    #[allow(clippy::todo)]
    pub fn new(_n: usize, _polyomino: &Polyomino, _op: Commutative, _target: Target) -> Self {
        todo!()
    }
}

#[allow(clippy::todo)]
impl Memo for Mdd {
    fn fill(&self, _cell: &Cell) -> Result<Fill, Error> {
        todo!()
    }

    fn remove(&self, _fills: HashMap<Cell, Fill>) -> Result<Self, Error> {
        todo!()
    }
}
