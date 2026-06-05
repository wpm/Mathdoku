//! The [`Solutions`] iterator: MAC search over a [`Puzzle`]'s constraint graph.

use crate::Fill;
use crate::grid::Grid;
use crate::puzzle::Puzzle;
use crate::{Cell, Error};

/// An iterator over all solutions for a [`Puzzle`].
///
/// Each item is a solved [`Grid`] in which every cell's values are a singleton.
/// Solutions are produced by interleaved propagation and backtracking search (MAC):
/// branching on the most-constrained cell calls [`Puzzle::set_value`], which propagates
/// all constraints to a fixpoint before the next branch is chosen.
///
/// Obtained via [`Puzzle::solutions`].
#[must_use]
#[allow(clippy::redundant_pub_crate)]
pub(crate) struct Solutions {
    stack: Vec<Puzzle>,
}

impl Solutions {
    pub(crate) fn new(puzzle: &Puzzle) -> Self {
        Self {
            stack: vec![puzzle.clone()],
        }
    }

    /// Finds the cell with the fewest values of size ≥ 2 (the most constrained variable).
    fn branch_cell(grid: &Grid) -> Option<(Cell, Fill)> {
        let n = grid.n();
        let mut best: Option<(Cell, Fill)> = None;
        for r in 0..n {
            for c in 0..n {
                let cell = Cell::new(r, c);
                if let Ok(values) = grid.get_values(cell)
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

impl Iterator for Solutions {
    type Item = Result<Grid, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(puzzle) = self.stack.pop() {
            // Every Puzzle on the stack is already at a fixpoint (maintained by set_value).
            // set_value returns None for infeasible branches, so they never enter the stack.
            let grid = puzzle.grid();
            let n = grid.n();

            // Check for success: all cells' values are singletons.
            let solved = (0..n)
                .flat_map(|r| (0..n).map(move |c| Cell::new(r, c)))
                .all(|cell| grid.get_values(cell).is_ok_and(Fill::is_singleton));
            if solved {
                return Some(Ok(grid));
            }

            // Branch on the most constrained unassigned cell.
            if let Some((cell, values)) = Self::branch_cell(&grid) {
                for v in values.values() {
                    if let Some(child) = puzzle.set_value(cell, v) {
                        self.stack.push(child);
                    }
                }
            }
        }
        None
    }
}
