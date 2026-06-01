//! The [`Solutions`] iterator: MAC search over a [`Puzzle`]'s constraint graph.

use crate::grid::Grid;
use crate::puzzle::Puzzle;
use crate::{Cell, Error, Values};

/// An iterator over all solutions for a [`Grid`] under a [`Puzzle`]'s constraints.
///
/// Each item is a solved [`Grid`] in which every cell's values are a singleton.
/// Solutions are produced by interleaved propagation and backtracking search (MAC):
/// after each branch, [`Puzzle::fixpoint`] is called to prune as far as possible before
/// the next branch.
///
/// Obtained via [`Grid::solutions`] or [`Puzzle::solutions`].
// Explicit `pub(crate)` marks the crate-internal API surface.
#[must_use]
#[allow(clippy::redundant_pub_crate)]
pub(crate) struct Solutions<'a> {
    stack: Vec<Grid>,
    puzzle: &'a Puzzle,
}

impl<'a> Solutions<'a> {
    pub(crate) fn new(grid: &Grid, puzzle: &'a Puzzle) -> Self {
        Self {
            stack: vec![*grid],
            puzzle,
        }
    }

    /// Finds the cell with the fewest values of size ≥ 2 (the most constrained variable).
    fn branch_cell(grid: &Grid) -> Option<(Cell, Values)> {
        let n = grid.n();
        let mut best: Option<(Cell, Values)> = None;
        for r in 0..n {
            for c in 0..n {
                let cell = Cell::new(r, c);
                if let Ok(values) = grid.cell_values(cell)
                    && values.len() >= 2
                    && best.is_none_or(|(_, d)| values.len() < d.len())
                {
                    best = Some((cell, values));
                }
            }
        }
        best
    }
}

impl Iterator for Solutions<'_> {
    type Item = Result<Grid, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(grid) = self.stack.pop() {
            // Propagate to fixpoint.
            let grid = match self.puzzle.propagate_grid(&grid) {
                Ok(g) => g,
                Err(e) => return Some(Err(e)),
            };

            let n = grid.n();

            // Check for failure: any empty value set means this branch is dead.
            let failed = (0..n)
                .flat_map(|r| (0..n).map(move |c| Cell::new(r, c)))
                .any(|cell| grid.cell_values(cell).is_ok_and(Values::is_empty));
            if failed {
                continue;
            }

            // Check for success: all cells' values are singletons.
            let solved = (0..n)
                .flat_map(|r| (0..n).map(move |c| Cell::new(r, c)))
                .all(|cell| grid.cell_values(cell).is_ok_and(Values::is_singleton));
            if solved {
                return Some(Ok(grid));
            }

            // Branch on the most constrained unassigned cell.
            if let Some((cell, values)) = Self::branch_cell(&grid) {
                for v in values.values() {
                    if let Ok(child) = grid.set_cell_value(cell, v) {
                        self.stack.push(child);
                    }
                }
            }
        }
        None
    }
}
