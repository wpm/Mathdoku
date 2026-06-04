//! Grid and cell types internal to the mdk implementation.
use crate::mdk::Error;
use crate::mdk::Error::MissingCell;
use crate::mdk::fill::Fill;
use crate::mdk::shape::Cell;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
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
        self.fill.get(cell).cloned().ok_or(MissingCell(*cell))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdk::fill::Fill;
    use serde_json::{Value, from_str, json, to_string};
    fn assert_all_full(g: &Grid, n: usize) {
        for r in 1..=n {
            for c in 1..=n {
                assert_eq!(g.get(&Cell(r, c)).unwrap(), Fill::new(n));
            }
        }
    }

    fn grid_with_modified_cell(n: usize, cell: Cell, fill: Fill) -> Grid {
        let mut g = Grid::new(n);
        drop(g.fill.insert(cell, fill));
        g
    }

    #[test]
    fn new_valid_sizes_succeed() {
        for n in 1..=9 {
            let g = Grid::new(n);
            assert_eq!(g.n, n);
        }
    }

    #[test]
    fn new_values_are_full() {
        assert_all_full(&Grid::new(4), 4);
    }

    #[test]
    fn get_values_out_of_bounds_returns_err() {
        let g = Grid::new(3);
        assert!(matches!(g.get(&Cell(4, 1)), Err(MissingCell(_))));
        assert!(matches!(g.get(&Cell(1, 4)), Err(MissingCell(_))));
    }

    #[test]
    fn display_shows_dimensions() {
        assert_eq!(Grid::new(4).to_string(), "4×4 grid");
    }

    #[test]
    fn grid_round_trips_through_json() {
        let g = grid_with_modified_cell(3, Cell(1, 1), Fill::from(&[2]));
        let restored: Grid = from_str(&to_string(&g).unwrap()).unwrap();
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
        let g = grid_with_modified_cell(2, Cell(1, 1), Fill::from(&[1]));
        let v: Value = from_str(&to_string(&g).unwrap()).unwrap();
        assert_eq!(v["fills"][0][0], json!([1]));
    }

    #[test]
    fn grid_deserialize_absent_values_uses_full_fill_sets() {
        let g: Grid = from_str(r#"{"n":3}"#).unwrap();
        assert_eq!(g.n, 3);
        assert_all_full(&g, 3);
    }

    #[test]
    fn grid_full_serializes_without_values() {
        let v: Value = from_str(&to_string(&Grid::new(3)).unwrap()).unwrap();
        assert!(v.get("fills").is_none() || v["fills"] == json!([]));
    }

    #[test]
    fn grid_full_round_trips_through_json() {
        let restored: Grid = from_str(&to_string(&Grid::new(3)).unwrap()).unwrap();
        assert_all_full(&restored, 3);
    }
}
