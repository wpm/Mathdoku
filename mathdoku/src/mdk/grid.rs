//! Grid and cell types internal to the mdk implementation.
use crate::mdk::Error;
use crate::mdk::Error::InvalidCell;
use crate::mdk::fill::Fill;
use std::collections::{BTreeMap, BTreeSet};

/// An n×n grid mapping each cell to its current candidate fill.
#[derive(Clone)]
pub struct Grid {
    n: usize,
    fill: BTreeMap<Cell, Fill>,
}

impl Grid {
    /// Creates a new grid of size `n` with every cell initialised to the full
    /// candidate set `{1..=n}`.
    pub fn new(n: usize) -> Self {
        let fill = (1..=n)
            .flat_map(|i| (1..=n).map(move |j| Cell(i, j)))
            .map(|cell| (cell, Fill::new(n)))
            .collect();
        Self { n, fill }
    }

    /// Returns the candidate fill for `cell`, or an error if the cell is not in this
    /// grid.
    pub fn get(&self, cell: &Cell) -> Result<Fill, Error> {
        self.fill.get(cell).cloned().ok_or(InvalidCell(*cell))
    }
}

/// A set of cells forming a polyomino (connected region of the grid).
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Hash)]
pub struct Polyomino(BTreeSet<Cell>);

impl Polyomino {
    pub(crate) fn from_cells(cells: impl IntoIterator<Item = Cell>) -> Self {
        Self(cells.into_iter().collect())
    }

    /// Returns the number of cells in this polyomino.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if `cell` is part of this polyomino.
    pub fn contains(&self, cell: &Cell) -> bool {
        self.0.contains(cell)
    }

    /// Returns an iterator over the cells of this polyomino in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = &Cell> {
        self.0.iter()
    }

    /// Returns the cells of this polyomino in sorted order.
    pub fn cells(&self) -> Vec<Cell> {
        self.0.iter().copied().collect()
    }
}

/// A grid position identified by `(row, column)`, both 1-indexed.
#[derive(Ord, Eq, PartialEq, Hash, PartialOrd, Copy, Clone, Debug)]
pub struct Cell(usize, usize);

impl Cell {
    /// Creates a cell at `(row, col)`, both 1-indexed.
    pub(crate) const fn new(row: usize, col: usize) -> Self {
        Self(row, col)
    }
}
