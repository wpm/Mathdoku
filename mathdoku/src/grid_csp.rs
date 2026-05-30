use crate::csp::{Constraint, Domain, State, Variable};
use crate::regin::regin_gac;
use crate::{Cage, Cell, Grid, Value, Values};

impl Domain<Value> for Values {
    fn remove(&self, z: Value) -> Self {
        (*self).remove(z)
    }

    fn is_empty(&self) -> bool {
        (*self).is_empty()
    }
}

impl Variable for Cell {}

impl State<Cell, Values, Value> for Grid {
    fn domain(&self, cell: &Cell) -> Values {
        self.cell_values(*cell)
            .unwrap_or_else(|e| invalid_cell(*cell, &e))
    }
}

impl Extend<(Cell, Values)> for Grid {
    fn extend<T: IntoIterator<Item = (Cell, Values)>>(&mut self, iter: T) {
        for (cell, values) in iter {
            *self = self
                .set_values(cell, values)
                .unwrap_or_else(|e| invalid_cell(cell, &e));
        }
    }
}

impl Constraint<Grid, Cell, Values, Value> for Cage {
    fn in_scope(&self, cell: &Cell) -> bool {
        self.polyomino().contains(cell)
    }

    fn propagate(&self, grid: Grid) -> impl Iterator<Item = (Cell, Values)> {
        let cells = self.cells();
        let domains: Vec<Values> = cells.iter().map(|cell| grid.domain(cell)).collect();
        let narrowed = self.mdd(grid.n()).support(&domains);
        cells.into_iter().zip(narrowed)
    }
}
#[derive(Clone)]
struct AllDifferent {
    cells: Vec<Cell>,
}
impl Constraint<Grid, Cell, Values, Value> for AllDifferent {
    fn in_scope(&self, cell: &Cell) -> bool {
        self.cells.contains(cell)
    }

    fn propagate(&self, grid: Grid) -> impl Iterator<Item = (Cell, Values)> {
        let values: Vec<Values> = self
            .cells
            .iter()
            .map(|cell| {
                grid.cell_values(*cell)
                    .unwrap_or_else(|e| invalid_cell(*cell, &e))
            })
            .collect();
        let unique_values = regin_gac(&values);
        self.cells.clone().into_iter().zip(unique_values)
    }
}

#[allow(clippy::panic)]
fn invalid_cell(cell: Cell, e: &crate::Error) -> ! {
    panic!("Invalid cell {cell} in constraint: {e}")
}
