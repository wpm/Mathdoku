//! Concrete CSP constraint types for Mathdoku: [`AllDifferent`] and [`Cage`] propagators,
//! wired to the generic [`crate::csp`] framework.

use std::sync::Arc;

use crate::cage::Cage;
use crate::csp::{Constraint, State};
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

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_with_values(values: &[(&(usize, usize), &[u8])]) -> Grid {
        let n = values.iter().map(|((r, c), _)| r.max(c) + 1).max().unwrap();
        let mut g = Grid::new(n).unwrap();
        for ((r, c), vals) in values {
            g = g
                .set_values(Cell::new(*r, *c), Values::new(vals).unwrap())
                .unwrap();
        }
        g
    }

    fn row0_forced_grid() -> Grid {
        grid_with_values(&[(&(0, 0), &[1, 2]), (&(0, 1), &[2]), (&(0, 2), &[1, 3])])
    }

    fn empty_puzzle(n: usize) -> Arc<Puzzle> {
        Arc::new(Puzzle::new(n).unwrap())
    }

    fn all_different_row(n: usize, row: usize) -> AllDifferent {
        AllDifferent::row(n, row, empty_puzzle(n))
    }

    fn all_different_column(n: usize, col: usize) -> AllDifferent {
        AllDifferent::column(n, col, empty_puzzle(n))
    }

    fn cage_fixture(
        positions: &[(usize, usize)],
        operator: crate::Operator,
        target: crate::Target,
    ) -> Cage {
        use crate::operation::Operation;
        use crate::polyomino::Polyomino;
        let cells: Vec<Cell> = positions.iter().map(|&(r, c)| Cell::new(r, c)).collect();
        Cage::new(
            Polyomino::from_cells(&cells).unwrap(),
            Operation::new(operator, target),
        )
        .unwrap()
    }

    fn puzzle_with(n: usize, c: &Cage) -> Arc<Puzzle> {
        Arc::new(Puzzle::new(n).unwrap().insert_cage(c.clone()).unwrap())
    }

    // --- AllDifferent::propagate ---

    #[test]
    fn propagate_full_values_unchanged() {
        let g = Grid::new(3).unwrap();
        let (new_g, changed) = all_different_row(3, 0).propagate(&g).unwrap();
        assert_eq!(new_g, g);
        assert!(changed.is_empty());
    }

    #[test]
    fn propagate_prunes_forced_value() {
        let (new_g, changed) = all_different_row(3, 0)
            .propagate(&row0_forced_grid())
            .unwrap();
        assert_eq!(
            new_g.cell_values(Cell::new(0, 0)).unwrap(),
            Values::new(&[1]).unwrap()
        );
        assert_eq!(
            new_g.cell_values(Cell::new(0, 1)).unwrap(),
            Values::new(&[2]).unwrap()
        );
        assert_eq!(
            new_g.cell_values(Cell::new(0, 2)).unwrap(),
            Values::new(&[3]).unwrap()
        );
        assert_eq!(changed.len(), 2);
        assert!(changed.contains(&Cell::new(0, 0)));
        assert!(changed.contains(&Cell::new(0, 2)));
    }

    #[test]
    fn propagate_infeasible_empties_values() {
        let g = grid_with_values(&[(&(0, 0), &[1]), (&(1, 0), &[1])]);
        let (new_g, changed) = all_different_column(2, 0).propagate(&g).unwrap();
        assert!(new_g.cell_values(Cell::new(0, 0)).unwrap().is_empty());
        assert!(new_g.cell_values(Cell::new(1, 0)).unwrap().is_empty());
        assert_eq!(changed.len(), 2);
    }

    #[test]
    fn propagate_unchanged_cells_not_in_changed() {
        let (_, changed) = all_different_row(3, 0)
            .propagate(&row0_forced_grid())
            .unwrap();
        assert!(!changed.contains(&Cell::new(0, 1)));
    }

    #[test]
    fn propagate_column_constraint() {
        let g = grid_with_values(&[(&(0, 1), &[1]), (&(1, 1), &[1, 2]), (&(2, 1), &[2, 3])]);
        let (new_g, _) = all_different_column(3, 1).propagate(&g).unwrap();
        assert_eq!(
            new_g.cell_values(Cell::new(1, 1)).unwrap(),
            Values::new(&[2]).unwrap()
        );
        assert_eq!(
            new_g.cell_values(Cell::new(2, 1)).unwrap(),
            Values::new(&[3]).unwrap()
        );
    }

    // --- propagate_cage ---

    #[test]
    fn cage_propagate_given_pins_cell() {
        let g = Grid::new(4).unwrap();
        let c = cage_fixture(&[(0, 0)], crate::Operator::Given, 3);
        let (new_g, changed) = propagate_cage(&c, &puzzle_with(4, &c), &g).unwrap();
        assert_eq!(
            new_g.cell_values(Cell::new(0, 0)).unwrap(),
            Values::new(&[3]).unwrap()
        );
        assert_eq!(changed, vec![Cell::new(0, 0)]);
    }

    #[test]
    fn cage_propagate_add_pair_prunes_impossible_values() {
        let g = Grid::new(4).unwrap();
        let c = cage_fixture(&[(0, 0), (0, 1)], crate::Operator::Add, 3);
        let (new_g, _) = propagate_cage(&c, &puzzle_with(4, &c), &g).unwrap();
        assert_eq!(
            new_g.cell_values(Cell::new(0, 0)).unwrap(),
            Values::new(&[1, 2]).unwrap()
        );
        assert_eq!(
            new_g.cell_values(Cell::new(0, 1)).unwrap(),
            Values::new(&[1, 2]).unwrap()
        );
    }

    #[test]
    fn cage_propagate_no_valid_tuple_empties_values() {
        let g = grid_with_values(&[(&(0, 0), &[4]), (&(0, 1), &[4])]);
        let c = cage_fixture(&[(0, 0), (0, 1)], crate::Operator::Add, 3);
        let (new_g, changed) = propagate_cage(&c, &puzzle_with(4, &c), &g).unwrap();
        assert!(new_g.cell_values(Cell::new(0, 0)).unwrap().is_empty());
        assert!(new_g.cell_values(Cell::new(0, 1)).unwrap().is_empty());
        assert_eq!(changed.len(), 2);
    }

    #[test]
    fn cage_propagate_values_constrain_tuples() {
        let g = Grid::new(4)
            .unwrap()
            .set_values(Cell::new(0, 1), Values::new(&[1, 2]).unwrap())
            .unwrap();
        let c = cage_fixture(&[(0, 0), (0, 1)], crate::Operator::Add, 5);
        let (new_g, _) = propagate_cage(&c, &puzzle_with(4, &c), &g).unwrap();
        assert_eq!(
            new_g.cell_values(Cell::new(0, 0)).unwrap(),
            Values::new(&[3, 4]).unwrap()
        );
        assert_eq!(
            new_g.cell_values(Cell::new(0, 1)).unwrap(),
            Values::new(&[1, 2]).unwrap()
        );
    }

    // --- new_valid_cell / new_out_of_bounds (formerly PuzzleCell tests) ---

    #[test]
    fn new_valid_cell_succeeds() {
        assert!(Grid::new(3).unwrap().cell_values(Cell::new(2, 2)).is_ok());
    }

    #[test]
    fn new_out_of_bounds_returns_invalid_cell() {
        assert!(matches!(
            Grid::new(3).unwrap().cell_values(Cell::new(3, 0)),
            Err(Error::InvalidCell(_))
        ));
    }
}
