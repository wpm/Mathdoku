//! [`Puzzle`] and the cage types needed to build one.
use crate::mdk::fill::{Fill, Memo};
use crate::mdk::grid::{Cell, Grid, Polyomino};
use crate::mdk::{Error, Target};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};

/// An n×n Mathdoku puzzle: a grid partitioned into cages, each with an arithmetic constraint.
pub struct Puzzle {
    grid: Grid,
    cage: HashMap<Cell, Cage>,
    memo: HashMap<Polyomino, Box<dyn Memo>>,
}

impl Puzzle {
    /// Returns the candidate fill for `cell`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCell`] if `cell` is not in the puzzle.
    pub fn get(&self, cell: &Cell) -> Result<Fill, Error> {
        let cage = self.cage.get(cell).ok_or(Error::InvalidCell(*cell))?;
        match cage.1.0 {
            Operator::Given => self.grid.get(cell),
            _ => self
                .memo
                .get(&cage.0)
                .ok_or(Error::InvalidCell(*cell))?
                .fill(cell),
        }
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
    pub fn insert(&self, _cage: &Cage) -> Result<(), Error> {
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

/// A polyomino paired with an arithmetic operation that its cell values must satisfy.
#[derive(PartialEq, Eq, Hash)]
pub struct Cage(Polyomino, Operation);

#[derive(PartialEq, Eq, Hash)]
enum Operator {
    Add,
    Multiply,
    Subtract,
    Divide,
    /// Cell value is given directly; no arithmetic constraint.
    Given,
}

impl Display for Operator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Add => "+",
            Self::Subtract => "−",
            Self::Multiply => "×",
            Self::Divide => "÷",
            Self::Given => "",
        };
        write!(f, "{s}")
    }
}

#[derive(PartialEq, Eq, Hash)]
struct Operation(Operator, Target);

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0, self.1)
    }
}
