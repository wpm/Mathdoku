//! Concrete CSP constraint types for Mathdoku: [`AllDifferent`] and [`Cage`] propagators,
//! wired to the generic [`crate::csp`] framework.

use std::sync::Arc;

use crate::cage::Cage;
use crate::csp::{Constraint, State, generalized_arc_consistency};
use crate::grid::Grid;
use crate::puzzle::Puzzle;
use crate::regin::regin_gac;
use crate::{Cell, Error, Values};

// ---- State impl ----

impl State<Cell, Values, Error> for Grid {
    fn get(&self, cell: Cell) -> Result<Values, Error> {
        self.cell_values(cell)
    }
}

// ---- AllDifferent ----

/// The constraint that all cells in a row or column must contain distinct values.
#[derive(Clone)]
pub(crate) struct AllDifferent {
    cells: Vec<Cell>,
    puzzle: Arc<Puzzle>,
}

impl AllDifferent {
    pub(crate) fn row(n: usize, row: usize, puzzle: Arc<Puzzle>) -> Self {
        Self {
            cells: (0..n).map(|column| Cell::new(row, column)).collect(),
            puzzle,
        }
    }

    pub(crate) fn column(n: usize, column: usize, puzzle: Arc<Puzzle>) -> Self {
        Self {
            cells: (0..n).map(|row| Cell::new(row, column)).collect(),
            puzzle,
        }
    }
}

impl Constraint<Grid, Cell, Values, Error> for AllDifferent {
    fn propagate(&self, state: &Grid) -> Result<(Grid, Vec<Cell>), Error> {
        let cells = &self.cells;
        let old_values: Vec<Values> = cells
            .iter()
            .map(|&c| state.cell_values(c))
            .collect::<Result<_, _>>()?;
        let new_values = regin_gac(&old_values);
        apply_values(state, &self.puzzle, cells, &old_values, &new_values)
    }

    fn in_scope(&self, cell: Cell) -> bool {
        self.cells.contains(&cell)
    }
}

// ---- Cage constraint ----

/// The arithmetic constraint imposed on a portion of the [`Grid`] by a [`Cage`].
#[derive(Clone)]
pub(crate) struct CageConstraint {
    cage: Cage,
    puzzle: Arc<Puzzle>,
}

impl Constraint<Grid, Cell, Values, Error> for CageConstraint {
    fn propagate(&self, state: &Grid) -> Result<(Grid, Vec<Cell>), Error> {
        propagate_cage(&self.cage, &self.puzzle, state)
    }

    fn in_scope(&self, cell: Cell) -> bool {
        self.cage.contains(cell)
    }
}

// ---- PuzzleConstraint ----

/// A constraint on the grid, either an [`AllDifferent`] or a cage arithmetic constraint.
#[derive(Clone)]
pub(crate) enum PuzzleConstraint {
    AllDifferent(AllDifferent),
    Cage(CageConstraint),
}

impl Constraint<Grid, Cell, Values, Error> for PuzzleConstraint {
    fn propagate(&self, state: &Grid) -> Result<(Grid, Vec<Cell>), Error> {
        match self {
            Self::AllDifferent(c) => c.propagate(state),
            Self::Cage(c) => c.propagate(state),
        }
    }

    fn in_scope(&self, cell: Cell) -> bool {
        match self {
            Self::AllDifferent(c) => c.in_scope(cell),
            Self::Cage(c) => c.in_scope(cell),
        }
    }
}

// ---- Constraint assembly ----

/// Builds the full constraint list for `puzzle`: one [`AllDifferent`] per row and column,
/// plus one cage constraint per cage.
pub(crate) fn puzzle_constraints(puzzle: &Arc<Puzzle>) -> Vec<PuzzleConstraint> {
    let n = puzzle.n();
    let rows =
        (0..n).map(|r| PuzzleConstraint::AllDifferent(AllDifferent::row(n, r, Arc::clone(puzzle))));
    let cols = (0..n)
        .map(|c| PuzzleConstraint::AllDifferent(AllDifferent::column(n, c, Arc::clone(puzzle))));
    let cages = puzzle.cages().cloned().map(|cage| {
        PuzzleConstraint::Cage(CageConstraint {
            cage,
            puzzle: Arc::clone(puzzle),
        })
    });
    rows.chain(cols).chain(cages).collect()
}

/// Runs `generalized_arc_consistency` over all puzzle constraints, returning the fixpoint grid.
///
/// Called by [`Puzzle::fixpoint`].
///
/// # Errors
/// Returns an error if propagation fails.
pub(crate) fn run_to_fixpoint(grid: Grid, constraints: &[PuzzleConstraint]) -> Result<Grid, Error> {
    generalized_arc_consistency(grid, constraints)
}

// ---- Helpers ----

fn apply_values(
    state: &Grid,
    puzzle: &Arc<Puzzle>,
    cells: &[Cell],
    old_values: &[Values],
    new_values: &[Values],
) -> Result<(Grid, Vec<Cell>), Error> {
    let _ = puzzle; // puzzle carried for future use (e.g. cage MDD invalidation)
    let mut new_state = *state;
    let mut changed = vec![];
    for ((&cell, old), new) in cells.iter().zip(old_values).zip(new_values) {
        if new != old {
            new_state = new_state.set_values(cell, *new)?;
            changed.push(cell);
        }
    }
    Ok((new_state, changed))
}

fn propagate_cage(
    cage: &Cage,
    puzzle: &Arc<Puzzle>,
    state: &Grid,
) -> Result<(Grid, Vec<Cell>), Error> {
    let cells = cage.cells();
    let old_values: Vec<Values> = cells
        .iter()
        .map(|&c| state.cell_values(c))
        .collect::<Result<_, _>>()?;
    let new_values = puzzle.mdd(cage).map_or_else(
        || brute_force_support(cage, puzzle.n(), &old_values),
        |mdd| mdd.support(&old_values),
    );
    apply_values(state, puzzle, &cells, &old_values, &new_values)
}

fn brute_force_support(cage: &Cage, n: usize, values: &[Values]) -> Vec<Values> {
    use crate::Target;
    use crate::operation::Operator;

    let arity = cage.polyomino().len();
    let op = cage.operation();
    let target = op.target;
    let n_val = u8::try_from(n).unwrap_or(u8::MAX);
    let mut support = vec![Values::default(); arity];

    let mut tuple = vec![1u8; arity];
    loop {
        let satisfies = match op.operator() {
            Operator::Given => arity == 1 && Target::from(tuple[0]) == target,
            Operator::Subtract => {
                arity == 2 && Target::from(tuple[0]).abs_diff(Target::from(tuple[1])) == target
            }
            Operator::Divide => {
                arity == 2 && {
                    let (va, vb) = (Target::from(tuple[0]), Target::from(tuple[1]));
                    va == vb * target || vb == va * target
                }
            }
            _ => false,
        };
        if satisfies && tuple.iter().zip(values).all(|(&v, d)| d.contains(v)) {
            for (pos, &v) in tuple.iter().enumerate() {
                support[pos] = support[pos] | Values::singleton(v);
            }
        }
        let mut pos = 0;
        while pos < arity && tuple[pos] == n_val {
            tuple[pos] = 1;
            pos += 1;
        }
        if pos == arity {
            break;
        }
        tuple[pos] += 1;
    }
    support
}
