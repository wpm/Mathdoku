//! [`Puzzle`]: the top-level constraint-solving interface for an mdk grid.
use crate::mdk::cage::{Cage, Operation};
use crate::mdk::fill::Fill;
use crate::mdk::grid::{Cell, Grid, Polyomino};
use crate::mdk::{Error, Target};
use std::collections::HashMap;

/// An n×n Mathdoku puzzle: a grid partitioned into cages, each with an arithmetic constraint.
pub struct Puzzle {
    grid: Grid,
    cages: HashMap<Cell, Cage>,
}

impl Puzzle {
    /// Returns the candidate fill for `cell`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCell`] if `cell` is not in the puzzle.
    pub fn get(&self, cell: &Cell) -> Result<Fill, Error> {
        let memo = &self
            .cages
            .get(cell)
            .ok_or(Error::InvalidCell(*cell))?
            .memo
            .as_ref();
        memo.map_or_else(|| self.grid.get(cell), |memo| memo.fill(cell))
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

    /// Returns the operations that are feasible for `polyomino` given the current grid state.
    ///
    /// An operation is feasible if at least one target value exists that is consistent
    /// with the candidate fills of the polyomino's cells.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCell`] if any cell of `polyomino` is not in the puzzle.
    #[allow(clippy::todo)]
    pub fn possible_operations(&self, _polyomino: &Polyomino) -> Result<Vec<Operation>, Error> {
        todo!()
    }

    /// Returns the target values that are feasible for `polyomino` under `operation`
    /// given the current grid state.
    ///
    /// A target is feasible if some assignment of values from the cells' candidate fills
    /// satisfies `operation` with that target.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCell`] if any cell of `polyomino` is not in the puzzle.
    #[allow(clippy::todo)]
    pub fn possible_targets(
        &self,
        _polyomino: &Polyomino,
        _operation: Operation,
    ) -> Result<Vec<Target>, Error> {
        todo!()
    }
}
