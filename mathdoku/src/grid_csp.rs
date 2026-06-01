use crate::csp::{Constraint, State};
use crate::{Cage, Cell, Error, Grid, Values};

impl State<Cell, Values, Error> for Grid {
    fn get(&self, cell: Cell) -> Result<Values, Error> {
        self.cell_values(cell)
    }
}

/// The constraint that all cells in a row or column must contain distinct values.
#[derive(Clone)]
struct AllDifferent {
    cells: Vec<Cell>,
}

impl AllDifferent {
    pub fn row(n: usize, i: usize) -> Self {
        Self {
            cells: (0..n).map(|j| Cell::new(i, j)).collect(),
        }
    }
    pub fn column(n: usize, i: usize) -> Self {
        Self {
            cells: (0..n).map(|j| Cell::new(j, i)).collect(),
        }
    }
}

impl Constraint<Grid, Cell, Values, Error> for AllDifferent {
    fn propagate(&self, grid: &Grid) -> Result<(Grid, Vec<Cell>), Error> {
        todo!()
    }

    fn in_scope(&self, cell: Cell) -> bool {
        self.cells.contains(&cell)
    }
}

/// The arithmetic constraint imposed on a portion of the [`Grid`] by a [`Cage`].
impl Constraint<Grid, Cell, Values, Error> for Cage {
    fn propagate(&self, grid: &Grid) -> Result<(Grid, Vec<Cell>), Error> {
        todo!()
    }

    fn in_scope(&self, cell: Cell) -> bool {
        self.contains(cell)
    }
}
