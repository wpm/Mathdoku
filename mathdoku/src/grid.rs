//! Grid and cell types internal to the mdk implementation.
use crate::Error;
use crate::Error::{InvalidGridSize, MissingCell};
use crate::csp::{Constraint, State};
use crate::fill::Fill;
use crate::polyomino::Cell;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

/// An n×n grid mapping each cell to its current candidate fill.
#[derive(Clone, Debug)]
pub struct Grid(usize, BTreeMap<Cell, Fill>);

impl Grid {
    /// Creates a new grid of size `n` with every cell initialised to the full
    /// candidate set `{1..=n}`.
    pub fn new(n: usize) -> Result<Self, Error> {
        if !(1..=9).contains(&n) {
            return Err(InvalidGridSize(n));
        }
        let fills = (1..=n)
            .flat_map(|i| (1..=n).map(move |j| Cell(i, j)))
            .map(|cell| (cell, Fill::all(n)))
            .collect();
        Ok(Self(n, fills))
    }

    /// Returns the [`Fill`] for `cell`.
    ///
    /// # Errors
    ///
    /// Returns [`MissingCell`] if `cell` is not in this grid.
    pub fn get(&self, cell: Cell) -> Result<Fill, Error> {
        self.1.get(&cell).copied().ok_or(MissingCell(cell))
    }

    /// Returns the grid size `n`.
    pub const fn size(&self) -> usize {
        self.0
    }

    /// Returns a new grid with `cell` updated to `fill`.
    pub fn set(&self, cell: Cell, fill: Fill) -> Self {
        let mut grid = self.clone();
        let _ = grid.1.insert(cell, fill);
        grid
    }

    /// Applies `new_fills` to `cells`, returning the updated grid and the cells whose fill changed.
    pub(crate) fn apply_fills(
        &self,
        cells: &[Cell],
        old_fills: &[Fill],
        new_fills: Vec<Fill>,
    ) -> (Self, Vec<Cell>) {
        let mut new_state = self.clone();
        let mut changed = vec![];
        for ((&cell, old), new) in cells.iter().zip(old_fills).zip(new_fills) {
            if new != *old {
                new_state = new_state.set(cell, new);
                changed.push(cell);
            }
        }
        (new_state, changed)
    }
}

impl State<Cell, Fill, Error> for Grid {
    fn get(&self, cell: Cell) -> Result<Fill, Error> {
        let fill = self.1.get(&cell).ok_or(MissingCell(cell))?;
        Ok(*fill)
    }
}

// ---- AllDifferent ----

/// The constraint that all cells in a row or column must contain distinct values.
#[derive(Clone)]
pub struct AllDifferent {
    cells: Vec<Cell>,
}

impl AllDifferent {
    /// Creates an all-different constraint over row `row` of an `n`×`n` grid.
    pub fn row(n: usize, row: usize) -> Self {
        Self {
            cells: (1..=n).map(|col| Cell(row, col)).collect(),
        }
    }

    /// Creates an all-different constraint over column `col` of an `n`×`n` grid.
    pub fn column(n: usize, col: usize) -> Self {
        Self {
            cells: (1..=n).map(|row| Cell(row, col)).collect(),
        }
    }
}

impl Constraint<Grid, Cell, Fill, Error> for AllDifferent {
    fn propagate(&self, state: &Grid) -> Result<(Grid, Vec<Cell>), Error> {
        let cells = &self.cells;
        let old_fills: Vec<Fill> = cells
            .iter()
            .map(|&c| state.get(c))
            .collect::<Result<_, _>>()?;
        let new_fills = crate::regin::regin_gac(&old_fills);
        Ok(state.apply_fills(cells, &old_fills, new_fills))
    }

    fn in_scope(&self, variable: Cell) -> bool {
        self.cells.contains(&variable)
    }
}

// Serde wire format: flat struct with an n×n `fills` array of cell fill sets.
// `fills` is optional on deserialization; absent means full fill sets for all cells.
#[derive(Serialize, Deserialize)]
struct GridWire {
    n: usize,
    #[serde(default)]
    fills: Vec<Vec<Fill>>,
}

impl Serialize for Grid {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let full = Fill::all(self.0);
        let is_full = self.1.values().all(|f| *f == full);
        let fills = if is_full {
            vec![]
        } else {
            (1..=self.0)
                .map(|r| (1..=self.0).map(|c| self.1[&Cell(r, c)]).collect())
                .collect()
        };
        GridWire { n: self.0, fills }.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Grid {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let wire = GridWire::deserialize(d)?;
        let n = wire.n;
        if wire.fills.is_empty() {
            return Self::new(n).map_err(|e| DeError::custom(e.to_string()));
        }
        if wire.fills.len() != n {
            return Err(DeError::custom(format!(
                "expected {n} rows of values, got {}",
                wire.fills.len()
            )));
        }
        for (r, row) in wire.fills.iter().enumerate() {
            if row.len() != n {
                return Err(DeError::custom(format!(
                    "row {r}: expected {n} columns, got {}",
                    row.len()
                )));
            }
        }
        let fills = wire
            .fills
            .into_iter()
            .enumerate()
            .flat_map(|(r, row)| {
                row.into_iter()
                    .enumerate()
                    .map(move |(c, f)| (Cell(r + 1, c + 1), f))
            })
            .collect();
        Ok(Self(n, fills))
    }
}

impl Display for Grid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}×{} grid", self.0, self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fill::Fill;
    use serde_json::{Value, from_str, json, to_string};
    fn assert_all_full(g: &Grid, n: usize) {
        for r in 1..=n {
            for c in 1..=n {
                assert_eq!(g.get(Cell(r, c)).unwrap(), Fill::all(n));
            }
        }
    }

    // Row 1 forced-chain: cell(1,1)={1,2}, cell(1,2)={2}, cell(1,3)={1,3}.
    // After AllDifferent: {1}, {2}, {3}.
    fn forced_chain_row1() -> Grid {
        Grid::new(3)
            .unwrap()
            .set(Cell(1, 1), Fill::from(&[1, 2]))
            .set(Cell(1, 2), Fill::from(&[2]))
            .set(Cell(1, 3), Fill::from(&[1, 3]))
    }

    fn grid_with_modified_cell(n: usize, cell: Cell, fill: Fill) -> Grid {
        let mut g = Grid::new(n).unwrap();
        let _ = g.1.insert(cell, fill);
        g
    }

    #[test]
    fn all_different_propagate_full_values_unchanged() {
        let g = Grid::new(3).unwrap();
        let (new_g, changed) = AllDifferent::row(3, 1).propagate(&g).unwrap();
        assert_eq!(new_g.1, g.1);
        assert!(changed.is_empty());
    }

    #[test]
    fn all_different_propagate_prunes_forced_value() {
        let (new_g, changed) = AllDifferent::row(3, 1)
            .propagate(&forced_chain_row1())
            .unwrap();
        assert_eq!(new_g.get(Cell(1, 1)).unwrap(), Fill::from(&[1]));
        assert_eq!(new_g.get(Cell(1, 2)).unwrap(), Fill::from(&[2]));
        assert_eq!(new_g.get(Cell(1, 3)).unwrap(), Fill::from(&[3]));
        assert_eq!(changed.len(), 2);
        assert!(changed.contains(&Cell(1, 1)));
        assert!(changed.contains(&Cell(1, 3)));
    }

    #[test]
    fn all_different_propagate_infeasible_empties_values() {
        // 2×2 grid: both column-1 cells pinned to {1} — infeasible.
        let g = Grid::new(2)
            .unwrap()
            .set(Cell(1, 1), Fill::from(&[1]))
            .set(Cell(2, 1), Fill::from(&[1]));
        let (new_g, changed) = AllDifferent::column(2, 1).propagate(&g).unwrap();
        assert!(new_g.get(Cell(1, 1)).unwrap().is_empty());
        assert!(new_g.get(Cell(2, 1)).unwrap().is_empty());
        assert_eq!(changed.len(), 2);
    }

    #[test]
    fn all_different_propagate_unchanged_cells_not_in_changed() {
        // cell(1,2)={2} is already a singleton — should not appear in changed.
        let (_, changed) = AllDifferent::row(3, 1)
            .propagate(&forced_chain_row1())
            .unwrap();
        assert!(!changed.contains(&Cell(1, 2)));
    }

    #[test]
    fn state_get_returns_fill_for_present_cell() {
        let fill = Fill::from(&[2, 3]);
        let g = grid_with_modified_cell(4, Cell(1, 1), fill);
        assert_eq!(
            <Grid as State<Cell, Fill, Error>>::get(&g, Cell(1, 1)).unwrap(),
            fill
        );
    }

    #[test]
    fn state_get_returns_missing_cell_for_absent_cell() {
        let g = Grid::new(3).unwrap();
        assert!(matches!(
            <Grid as State<Cell, Fill, Error>>::get(&g, Cell(4, 1)),
            Err(MissingCell(_))
        ));
    }

    #[test]
    fn new_valid_sizes_succeed() {
        for n in 1..=9 {
            let g = Grid::new(n).unwrap();
            assert_eq!(g.0, n);
        }
    }

    #[test]
    fn new_rejects_zero() {
        assert!(matches!(Grid::new(0), Err(InvalidGridSize(0))));
    }

    #[test]
    fn new_rejects_ten() {
        assert!(matches!(Grid::new(10), Err(InvalidGridSize(10))));
    }

    #[test]
    fn new_values_are_full() {
        assert_all_full(&Grid::new(4).unwrap(), 4);
    }

    #[test]
    fn get_values_out_of_bounds_returns_err() {
        let g = Grid::new(3).unwrap();
        assert!(matches!(g.get(Cell(4, 1)), Err(MissingCell(_))));
        assert!(matches!(g.get(Cell(1, 4)), Err(MissingCell(_))));
    }

    #[test]
    fn display_shows_dimensions() {
        assert_eq!(Grid::new(4).unwrap().to_string(), "4×4 grid");
    }

    #[test]
    fn grid_round_trips_through_json() {
        let g = grid_with_modified_cell(3, Cell(1, 1), Fill::from(&[2]));
        let restored: Grid = from_str(&to_string(&g).unwrap()).unwrap();
        assert_eq!(g.1, restored.1);
        assert_eq!(g.0, restored.0);
    }

    #[test]
    fn grid_deserialize_invalid_n_returns_err() {
        assert!(from_str::<Grid>(r#"{"n":0,"fills":[]}"#).is_err());
        assert!(from_str::<Grid>(r#"{"n":10,"fills":[]}"#).is_err());
    }

    #[test]
    fn grid_deserialize_wrong_row_count_returns_err() {
        assert!(from_str::<Grid>(r#"{"n":2,"fills":[[1,2]]}"#).is_err());
    }

    #[test]
    fn grid_deserialize_wrong_column_count_returns_err() {
        assert!(from_str::<Grid>(r#"{"n":2,"fills":[[1,2,3],[1,2,3]]}"#).is_err());
    }

    #[test]
    fn grid_serialize_values_are_row_major() {
        let g = grid_with_modified_cell(2, Cell(1, 1), Fill::from(&[1]));
        let v: Value = from_str(&to_string(&g).unwrap()).unwrap();
        assert_eq!(v["fills"][0][0], json!([1]));
    }

    #[test]
    fn grid_deserialize_absent_values_uses_full_fill_sets() {
        let g: Grid = from_str(r#"{"n":3}"#).unwrap();
        assert_eq!(g.0, 3);
        assert_all_full(&g, 3);
    }

    #[test]
    fn grid_full_serializes_without_values() {
        let v: Value = from_str(&to_string(&Grid::new(3).unwrap()).unwrap()).unwrap();
        assert!(v.get("fills").is_none() || v["fills"] == json!([]));
    }

    #[test]
    fn grid_full_round_trips_through_json() {
        let restored: Grid = from_str(&to_string(&Grid::new(3).unwrap()).unwrap()).unwrap();
        assert_all_full(&restored, 3);
    }
}
