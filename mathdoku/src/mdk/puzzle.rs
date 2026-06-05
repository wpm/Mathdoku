//! [`Puzzle`]: the top-level constraint-solving interface.
use crate::Operation;
use crate::mdk::Error::MissingCell;
use crate::mdk::cage::Cage;
use crate::mdk::fill::Fill;
use crate::mdk::grid::Grid;
use crate::mdk::polyomino::{Cell, Polyomino};
use crate::mdk::{Error, N, Target};
use std::collections::HashMap;
use std::sync::Arc;

/// A Mathdoku puzzle: an n×n grid partitioned into cages, each with an arithmetic constraint.
#[derive(Clone)]
pub struct Puzzle {
    grid: Grid,
    cages: HashMap<Cell, Arc<Cage>>,
}

impl Puzzle {
    /// Returns the candidate fill for `cell`.
    ///
    /// # Errors
    ///
    /// Returns [`MissingCell`] if `cell` is not in the puzzle.
    pub fn get(&self, cell: Cell) -> Result<Fill, Error> {
        self.grid.get(cell)
    }

    /// # Errors
    ///
    /// Returns an error if `cell` is not in the puzzle or `n` is not a candidate value for it.
    #[allow(clippy::todo)]
    pub fn set(&self, cell: Cell, n: N) -> Result<Self, Error> {
        let fill = self.grid.get(cell)?;
        if !fill.contains(n) {
            return Err(Error::InvalidCellValue(cell, n));
        }
        Ok(Self {
            grid: self.grid.set(cell, fill),
            cages: self.cages.clone(),
        })
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
    pub fn remove(&self, cage: &Cage) -> Result<(), Error> {
        let mut cages = self.cages.clone();
        for cell in cage.polyomino.iter() {
            let _ = cages.remove(cell).ok_or(MissingCell(*cell));
        }
        Ok(())
    }

    /// Returns the operations that are feasible for `polyomino` given the current grid state.
    ///
    /// An operation is feasible if at least one target value exists that is consistent
    /// with the candidate fills of the polyomino's cells.
    ///
    /// # Errors
    ///
    /// Returns [`MissingCell`] if any cell of `polyomino` is not in the puzzle.
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
    /// Returns [`MissingCell`] if any cell of `polyomino` is not in the puzzle.
    #[allow(clippy::todo)]
    pub fn possible_targets(
        &self,
        _polyomino: &Polyomino,
        _operation: Operation,
    ) -> Result<Vec<Target>, Error> {
        todo!()
    }

    /// Propagates all cage and all-different constraints to a GAC fixpoint.
    ///
    /// Returns `None` if any cell's domain becomes empty (infeasible).
    #[must_use]
    #[allow(clippy::todo)]
    pub fn fixpoint(&self) -> Option<Self> {
        todo!()
    }
}
