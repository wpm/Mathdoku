//! The [`Solutions`] iterator: MAC search over a [`Puzzle`]'s constraint graph.

use crate::Error;
use crate::fill::Fill;
use crate::polyomino::Cell;
use crate::puzzle::Puzzle;

/// An iterator over all solutions for a [`Puzzle`].
///
/// Each item is a solved [`Puzzle`] in which every cell's fill is a singleton.
/// Solutions are produced by interleaved propagation and backtracking search (MAC):
/// branching on the most-constrained cell calls [`Puzzle::set`], which propagates
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

    /// Finds the cell with the fewest candidate values of size ≥ 2 (most constrained).
    fn branch_cell(puzzle: &Puzzle) -> Option<(Cell, Fill)> {
        let n = puzzle.n();
        let mut best: Option<(Cell, Fill)> = None;
        for r in 1..=n {
            for c in 1..=n {
                let cell = Cell(r, c);
                if let Ok(fill) = puzzle.get(cell)
                    && fill.len() >= 2
                    && best.is_none_or(|(_, d)| fill.len() < d.len())
                {
                    best = Some((cell, fill));
                }
            }
        }
        best
    }
}

impl Iterator for Solutions {
    type Item = Result<Puzzle, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(puzzle) = self.stack.pop() {
            let n = puzzle.n();

            // Check for success: all cells' fills are singletons.
            let solved = (1..=n)
                .flat_map(|r| (1..=n).map(move |c| Cell(r, c)))
                .all(|cell| puzzle.get(cell).is_ok_and(Fill::is_singleton));
            if solved {
                return Some(Ok(puzzle));
            }

            // Branch on the most constrained unassigned cell.
            if let Some((cell, fill)) = Self::branch_cell(&puzzle) {
                for v in fill.values() {
                    if let Ok(child) = puzzle.set(cell, v)
                        && let Some(fp) = child.fixpoint()
                    {
                        self.stack.push(fp);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::fill::Fill;
    use crate::polyomino::Cell;
    use crate::puzzle::Puzzle;

    fn solved_puzzles(puzzle: &Puzzle) -> Vec<Puzzle> {
        puzzle.solutions().map(Result::unwrap).collect()
    }

    #[test]
    fn empty_2x2_has_two_solutions() {
        // The two 2×2 Latin squares: [[1,2],[2,1]] and [[2,1],[1,2]].
        let p = Puzzle::new(2).unwrap();
        let solutions = solved_puzzles(&p);
        assert_eq!(solutions.len(), 2);
        for s in &solutions {
            for r in 1..=2 {
                for c in 1..=2 {
                    assert!(s.get(Cell(r, c)).unwrap().is_singleton());
                }
            }
        }
    }

    #[test]
    fn empty_3x3_has_twelve_solutions() {
        // There are exactly 12 Latin squares of order 3.
        let p = Puzzle::new(3).unwrap();
        assert_eq!(p.solutions().count(), 12);
    }

    #[test]
    fn given_cage_pins_solution() {
        // Pinning (1,1)=1 in a 2×2 leaves exactly one Latin square.
        let p = crate::test_util::pinned_2x2();
        let solutions = solved_puzzles(&p);
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].get(Cell(1, 1)).unwrap(), Fill::from(&[1]));
        assert_eq!(solutions[0].get(Cell(1, 2)).unwrap(), Fill::from(&[2]));
        assert_eq!(solutions[0].get(Cell(2, 1)).unwrap(), Fill::from(&[2]));
        assert_eq!(solutions[0].get(Cell(2, 2)).unwrap(), Fill::from(&[1]));
    }

    #[test]
    fn contradictory_state_has_no_solutions() {
        // Pin two cells of row 1 to the same value via `set` (which does not
        // propagate): every branch's fixpoint is infeasible, so the search
        // exhausts without producing a solution.
        let p = Puzzle::new(2)
            .unwrap()
            .set(Cell(1, 1), 1)
            .unwrap()
            .set(Cell(1, 2), 1)
            .unwrap();
        assert_eq!(p.solutions().count(), 0);
    }

    #[test]
    fn fully_solved_puzzle_yields_itself() {
        // A puzzle whose every fill is already a singleton is returned as-is.
        let square: Vec<Vec<crate::N>> = vec![vec![1, 2], vec![2, 1]];
        let p = Puzzle::from_latin_square(2, &square).unwrap();
        let solutions = solved_puzzles(&p);
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].get(Cell(1, 1)).unwrap(), Fill::from(&[1]));
    }
}
