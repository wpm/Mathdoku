//! [`Puzzle`] and the cage types needed to build one.
use crate::mdk::Error;
use crate::mdk::cage::Cage;
use crate::mdk::fill::Fill;
use crate::mdk::grid::{Cell, Grid};
use std::collections::{BTreeSet, HashMap};

/// An n×n Mathdoku puzzle: a grid partitioned into cages, each with an arithmetic constraint.
pub struct Puzzle {
    grid: Grid,
    cages: BTreeSet<Cage>,
}

impl Puzzle {
    /// Returns the candidate fill for `cell`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCell`] if `cell` is not in the puzzle.
    pub fn get(&self, cell: &Cell) -> Result<Fill, Error> {
        let cage = self
            .cages
            .iter()
            .find(|c| c.polyomino.contains(cell))
            .ok_or(Error::InvalidCell(*cell))?;
        cage.memo
            .as_ref()
            .map_or_else(|| self.grid.get(cell), |memo| memo.fill(cell))
    }

    /// Applies `fills` as assignments and returns the updated candidate fills for all cells.
    ///
    /// # Errors
    ///
    /// Returns an error if any cell in `fills` is not in the puzzle.
    #[allow(clippy::todo)]
    pub fn set(&self, _fills: HashMap<Cell, Fill>) -> Result<HashMap<Cell, Fill>, Error> {
        todo!()
    }

    /// Adds `cage` to the puzzle.
    ///
    /// # Errors
    ///
    /// Returns an error if `cage` overlaps with an existing cage.
    #[allow(clippy::todo)]
    pub fn insert(&self, _cage: Cage) -> Result<(), Error> {
        todo!()
    }

    /// Removes `cage` from the puzzle.
    ///
    /// # Errors
    ///
    /// Returns an error if `cage` is not in the puzzle.
    #[allow(clippy::todo)]
    pub fn remove(&self, _cage: &Cage) -> Result<(), Error> {
        todo!()
    }
}
