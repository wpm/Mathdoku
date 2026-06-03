//! Grid and cell types internal to the mdk implementation.
use crate::mdk::Error;
use crate::mdk::Error::InvalidCell;
use crate::mdk::fill::Fill;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

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
        let full = Fill::new(self.n);
        let is_full = self.fill.values().all(|f| f == &full);
        let fills = if is_full {
            vec![]
        } else {
            (1..=self.n)
                .map(|r| {
                    (1..=self.n)
                        .map(|c| self.fill[&Cell(r, c)].clone())
                        .collect()
                })
                .collect()
        };
        GridWire { n: self.n, fills }.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Grid {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let wire = GridWire::deserialize(d)?;
        let n = wire.n;
        if !(1..=9).contains(&n) {
            return Err(DeError::custom(format!("invalid grid size {n}")));
        }
        if wire.fills.is_empty() {
            return Ok(Self::new(n));
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
        let fill = wire
            .fills
            .into_iter()
            .enumerate()
            .flat_map(|(r, row)| {
                row.into_iter()
                    .enumerate()
                    .map(move |(c, f)| (Cell(r + 1, c + 1), f))
            })
            .collect();
        Ok(Self { n, fill })
    }
}

impl Display for Grid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}×{} grid", self.n, self.n)
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
#[derive(Ord, Eq, PartialEq, Hash, PartialOrd, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Cell(usize, usize);

impl Cell {
    /// Creates a cell at `(row, col)`, both 1-indexed.
    pub(crate) const fn new(row: usize, col: usize) -> Self {
        Self(row, col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdk::fill::Fill;
    use serde_json::{Value, from_str, json, to_string};

    #[test]
    fn new_valid_sizes_succeed() {
        for n in 1..=9 {
            let g = Grid::new(n);
            assert_eq!(g.n, n);
        }
    }

    #[test]
    fn new_values_are_full() {
        let g = Grid::new(4);
        for r in 1..=4 {
            for c in 1..=4 {
                assert_eq!(g.get(&Cell::new(r, c)).unwrap(), Fill::new(4));
            }
        }
    }

    #[test]
    fn get_values_out_of_bounds_returns_err() {
        let g = Grid::new(3);
        assert!(matches!(g.get(&Cell::new(4, 1)), Err(InvalidCell(_))));
        assert!(matches!(g.get(&Cell::new(1, 4)), Err(InvalidCell(_))));
    }

    #[test]
    fn display_shows_dimensions() {
        assert_eq!(Grid::new(4).to_string(), "4×4 grid");
    }

    #[test]
    fn grid_round_trips_through_json() {
        let mut g = Grid::new(3);
        drop(g.fill.insert(Cell::new(1, 1), Fill::from(&[2])));
        let json = to_string(&g).unwrap();
        let restored: Grid = from_str(&json).unwrap();
        assert_eq!(g.fill, restored.fill);
        assert_eq!(g.n, restored.n);
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
        let mut g = Grid::new(2);
        drop(g.fill.insert(Cell::new(1, 1), Fill::from(&[1])));
        let json = to_string(&g).unwrap();
        let v: Value = from_str(&json).unwrap();
        assert_eq!(v["fills"][0][0], json!([1]));
    }

    #[test]
    fn grid_deserialize_absent_values_uses_full_fill_sets() {
        let g: Grid = from_str(r#"{"n":3}"#).unwrap();
        assert_eq!(g.n, 3);
        for r in 1..=3 {
            for c in 1..=3 {
                assert_eq!(g.get(&Cell::new(r, c)).unwrap(), Fill::new(3));
            }
        }
    }

    #[test]
    fn grid_full_serializes_without_values() {
        let g = Grid::new(3);
        let v: Value = from_str(&to_string(&g).unwrap()).unwrap();
        assert!(v.get("fills").is_none() || v["fills"] == json!([]));
    }

    #[test]
    fn grid_full_round_trips_through_json() {
        let g = Grid::new(3);
        let restored: Grid = from_str(&to_string(&g).unwrap()).unwrap();
        for r in 1..=3 {
            for c in 1..=3 {
                assert_eq!(restored.get(&Cell::new(r, c)).unwrap(), Fill::new(3));
            }
        }
    }
}
