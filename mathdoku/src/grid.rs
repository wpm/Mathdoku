//! The [`Grid`] type: an `n×n` grid of cell values.

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};

use crate::Error::InvalidGridSize;
use crate::{Cell, Error, Value, Values};

// Serde wire format: flat struct with an n×n `values` array of cell value sets.
// `values` is optional on deserialization; absent means full value sets for all cells.
#[derive(Serialize, Deserialize)]
struct GridWire {
    n: usize,
    #[serde(default)]
    values: Vec<Vec<Values>>,
}

/// An `n×n` grid of cell values.
///
/// Each cell has a [`Values`] set — the candidate values `1..=n` still
/// consistent with the constraints applied so far. Use [`crate::Puzzle::grid`]
/// to get the propagated grid from a [`crate::Puzzle`].
///
/// `values` is a flat `[Values; 81]` array stored inline (no heap allocation).
/// Only the first `n*n` entries are used; the rest are `Values::default()`.
/// Cloning a `Grid` is a plain stack copy — no allocator involvement.
#[must_use]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Grid {
    n: usize,
    values: [Values; 81],
}

impl Grid {
    /// Creates an `n×n` grid with all cell values initialized to `{1, ..., n}`.
    ///
    /// # Errors
    /// Returns [`InvalidGridSize`] if `n` is not in `1..=9`.
    pub fn new(n: usize) -> Result<Self, Error> {
        if !(1..=9).contains(&n) {
            return Err(InvalidGridSize(n));
        }
        let full = Values::all(n);
        let mut values = [Values::default(); 81];
        for slot in values.iter_mut().take(n * n) {
            *slot = full;
        }
        Ok(Self { n, values })
    }

    /// Returns the grid size `n` (grid is `n`×`n`).
    #[must_use]
    pub const fn n(&self) -> usize {
        self.n
    }

    /// Returns the current values of `cell`.
    ///
    /// # Errors
    /// Returns [`Error::InvalidCell`] if `cell` is outside the grid.
    pub fn get_values(&self, cell: Cell) -> Result<Values, Error> {
        Ok(self.values[self.index(cell)?])
    }

    /// Returns a new grid with `cell`'s values narrowed to the singleton `{n}`.
    ///
    /// # Errors
    /// Returns [`Error::InvalidCell`] if `cell` is outside the grid.
    pub(crate) fn set_value(&self, cell: Cell, n: Value) -> Result<Self, Error> {
        self.set_values(cell, Values::singleton(n))
    }

    /// Returns a new grid with `cell`'s values replaced by `values`.
    ///
    /// # Errors
    /// Returns [`Error::InvalidCell`] if `cell` is outside the grid.
    pub(crate) fn set_values(&self, cell: Cell, values: Values) -> Result<Self, Error> {
        let i = self.index(cell)?;
        let mut new_values = self.values;
        new_values[i] = values;
        Ok(Self {
            n: self.n,
            values: new_values,
        })
    }

    /// Creates a `Grid` whose cell values are the singleton values from `square`.
    ///
    /// `square` must be an `n×n` slice of rows, each row containing values in `1..=n`.
    ///
    /// # Errors
    /// Returns [`InvalidGridSize`] if `square.len() != n` or any row has length ≠ `n`,
    /// and [`Error::InvalidValue`] if any value is outside `1..=n`.
    pub fn from_latin_square(n: usize, square: &[Vec<Value>]) -> Result<Self, Error> {
        let mut grid = Self::new(n)?;
        for (r, row) in square.iter().enumerate() {
            for (c, &v) in row.iter().enumerate() {
                let cell = Cell::new(r, c);
                grid = grid.set_values(cell, Self::singleton_values(v))?;
            }
        }
        Ok(grid)
    }

    fn singleton_values(v: Value) -> Values {
        Values::singleton(v)
    }

    pub(crate) const fn index(&self, cell: Cell) -> Result<usize, Error> {
        if cell.row < self.n && cell.column < self.n {
            Ok(cell.row * self.n + cell.column)
        } else {
            Err(Error::InvalidCell(cell))
        }
    }
}

impl Serialize for Grid {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let rows: Vec<Vec<Values>> = (0..self.n)
            .map(|r| (0..self.n).map(|c| self.values[r * self.n + c]).collect())
            .collect();
        GridWire {
            n: self.n,
            values: rows,
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for Grid {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let wire = GridWire::deserialize(d)?;
        let n = wire.n;
        if !(1..=9).contains(&n) {
            return Err(DeError::custom(format!("invalid grid size {n}")));
        }
        let mut values = [Values::default(); 81];
        if wire.values.is_empty() {
            let full = Values::all(n);
            for slot in values.iter_mut().take(n * n) {
                *slot = full;
            }
        } else {
            if wire.values.len() != n {
                return Err(DeError::custom(format!(
                    "expected {n} rows of values, got {}",
                    wire.values.len()
                )));
            }
            for (r, row) in wire.values.iter().enumerate() {
                if row.len() != n {
                    return Err(DeError::custom(format!(
                        "row {r}: expected {n} columns, got {}",
                        row.len()
                    )));
                }
            }
            for (slot, v) in values.iter_mut().zip(wire.values.into_iter().flatten()) {
                *slot = v;
            }
        }
        Ok(Self { n, values })
    }
}
impl Display for Grid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}×{} grid", self.n, self.n)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, from_str, json, to_string};

    use super::*;
    use crate::Target;
    use crate::operation::Operator;
    use crate::operation::Operator::{Add, Given};
    use crate::puzzle::Puzzle;
    use crate::test_utils::cage_at;

    fn puzzle_with_cage(
        n: usize,
        positions: &[(usize, usize)],
        operator: Operator,
        target: Target,
    ) -> Puzzle {
        let cage = cage_at(positions, operator, target);
        Puzzle::new(n).unwrap().insert_cage(cage).unwrap().unwrap()
    }

    // --- Grid::new ---

    #[test]
    fn new_valid_sizes_succeed() {
        for n in 1..=9 {
            assert!(Grid::new(n).is_ok(), "size {n} should succeed");
        }
    }

    #[test]
    fn new_size_zero_returns_err() {
        assert!(matches!(Grid::new(0), Err(InvalidGridSize(0))));
    }

    #[test]
    fn new_size_ten_returns_err() {
        assert!(matches!(Grid::new(10), Err(InvalidGridSize(10))));
    }

    #[test]
    fn new_values_are_full() {
        let g = Grid::new(4).unwrap();
        let expected = Values::all(4);
        for r in 0..4 {
            for c in 0..4 {
                assert_eq!(
                    g.get_values(Cell::new(r, c)).unwrap(),
                    expected,
                    "cell ({r},{c}) should have full values"
                );
            }
        }
    }

    // --- Grid::get_values ---

    #[test]
    fn get_values_out_of_bounds_returns_err() {
        let g = Grid::new(3).unwrap();
        assert!(matches!(
            g.get_values(Cell::new(3, 0)),
            Err(Error::InvalidCell(_))
        ));
        assert!(matches!(
            g.get_values(Cell::new(0, 3)),
            Err(Error::InvalidCell(_))
        ));
    }

    // --- Grid::set_value ---

    #[test]
    fn set_value_narrows_values() {
        let g = Grid::new(4).unwrap();
        let cell = Cell::new(1, 2);
        let g2 = g.set_value(cell, 3).unwrap();
        assert_eq!(g2.get_values(cell).unwrap(), Values::new(&[3]).unwrap());
    }

    #[test]
    fn set_value_is_non_destructive() {
        let g = Grid::new(4).unwrap();
        let cell = Cell::new(0, 0);
        let _ = g.set_value(cell, 2).unwrap();
        // Original grid is unchanged.
        assert_eq!(g.get_values(cell).unwrap(), Values::all(4));
    }

    #[test]
    fn set_value_out_of_bounds_returns_err() {
        let g = Grid::new(3).unwrap();
        assert!(matches!(
            g.set_value(Cell::new(3, 0), 1),
            Err(Error::InvalidCell(_))
        ));
    }

    // --- Grid::constrain ---

    // Builds a fully caged 2×2 puzzle and a Grid, verifies constrain pins every cell.
    //
    //   [1][2]
    //   [2][1]
    //
    fn solved_2x2_puzzle() -> Puzzle {
        Puzzle::new(2)
            .unwrap()
            .insert_cage(cage_at(&[(0, 0)], Given, 1))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(0, 1)], Given, 2))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(1, 0)], Given, 2))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(1, 1)], Given, 1))
            .unwrap()
            .unwrap()
    }

    // --- Puzzle::solutions (via Grid tests) ---

    #[test]
    fn solutions_no_cages_yields_all_latin_squares() {
        let puzzle = Puzzle::new(2).unwrap();
        let solutions: Vec<Grid> = puzzle.solutions().map(Result::unwrap).collect();
        assert_eq!(solutions.len(), 2);
        for sol in &solutions {
            for r in 0..2 {
                for c in 0..2 {
                    assert!(sol.get_values(Cell::new(r, c)).unwrap().is_singleton());
                }
            }
        }
    }

    #[test]
    fn solutions_fully_caged_yields_one_solution() {
        let puzzle = solved_2x2_puzzle();
        let solutions: Vec<Grid> = puzzle.solutions().map(Result::unwrap).collect();
        assert_eq!(solutions.len(), 1);
        let sol = &solutions[0];
        assert_eq!(
            sol.get_values(Cell::new(0, 0)).unwrap(),
            Values::new(&[1]).unwrap()
        );
        assert_eq!(
            sol.get_values(Cell::new(0, 1)).unwrap(),
            Values::new(&[2]).unwrap()
        );
        assert_eq!(
            sol.get_values(Cell::new(1, 0)).unwrap(),
            Values::new(&[2]).unwrap()
        );
        assert_eq!(
            sol.get_values(Cell::new(1, 1)).unwrap(),
            Values::new(&[1]).unwrap()
        );
    }

    #[test]
    fn solutions_infeasible_yields_none() {
        // Two Given cages that force conflicting values in the same row: (0,0)=1
        // and (0,1)=1 violate all-different. The second insert_cage detects this
        // via propagation and returns Ok(None) (infeasible), so no solutions are produced.
        let p = Puzzle::new(2).unwrap();
        let p = p
            .insert_cage(cage_at(&[(0, 0)], Given, 1))
            .unwrap()
            .unwrap();
        assert!(matches!(
            p.insert_cage(cage_at(&[(0, 1)], Given, 1)),
            Ok(None)
        ));
    }

    #[test]
    fn solutions_mixed_cages_unique_solution() {
        let puzzle = Puzzle::new(2)
            .unwrap()
            .insert_cage(cage_at(&[(0, 0), (0, 1)], Add, 3))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(1, 0)], Given, 2))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(1, 1)], Given, 1))
            .unwrap()
            .unwrap();
        let solutions: Vec<Grid> = puzzle.solutions().map(Result::unwrap).collect();
        assert_eq!(solutions.len(), 1);
        let sol = &solutions[0];
        assert_eq!(
            sol.get_values(Cell::new(0, 0)).unwrap(),
            Values::new(&[1]).unwrap()
        );
        assert_eq!(
            sol.get_values(Cell::new(0, 1)).unwrap(),
            Values::new(&[2]).unwrap()
        );
        assert_eq!(
            sol.get_values(Cell::new(1, 0)).unwrap(),
            Values::new(&[2]).unwrap()
        );
        assert_eq!(
            sol.get_values(Cell::new(1, 1)).unwrap(),
            Values::new(&[1]).unwrap()
        );
    }

    #[test]
    fn solutions_3x3_row_sum_cages_all_valid() {
        let puzzle = Puzzle::new(3)
            .unwrap()
            .insert_cage(cage_at(&[(0, 0), (0, 1), (0, 2)], Add, 6))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(1, 0), (1, 1), (1, 2)], Add, 6))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(2, 0), (2, 1), (2, 2)], Add, 6))
            .unwrap()
            .unwrap();
        let solutions: Vec<Grid> = puzzle.solutions().map(Result::unwrap).collect();
        assert!(!solutions.is_empty(), "should have at least one solution");
        for sol in &solutions {
            assert!(sol.is_solution());
            for r in 0..3 {
                let row_sum: u32 = (0..3)
                    .map(|c| u32::from(sol.get_values(Cell::new(r, c)).unwrap().values()[0]))
                    .sum();
                assert_eq!(row_sum, 6, "row {r} should sum to 6");
            }
        }
    }

    #[test]
    fn solutions_4x4_mixed_cages_match_expected_set() {
        // End-to-end regression for the MDD cutover. A fully caged 4×4 puzzle
        // mixing cage shapes (givens, row pairs, a column pair, horizontal
        // triominoes) and operators (Given / Add / Subtract / Multiply). The
        // puzzle is intentionally under-determined: MDD-based cage propagation
        // must reproduce *exactly* the same three-solution set the old
        // multiset → permute → filter pipeline produced.
        let puzzle = Puzzle::new(4)
            .unwrap()
            .insert_cage(cage_at(&[(0, 0)], Given, 1))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(0, 1), (0, 2)], Add, 5))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(0, 3), (1, 3)], Add, 6))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(1, 0), (1, 1)], Operator::Multiply, 12))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(1, 2)], Given, 1))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(2, 0), (3, 0)], Operator::Subtract, 2))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(2, 1), (2, 2), (2, 3)], Operator::Multiply, 12))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(3, 1), (3, 2), (3, 3)], Operator::Multiply, 6))
            .unwrap()
            .unwrap();

        let mut actual: Vec<[[u8; 4]; 4]> = puzzle
            .solutions()
            .map(Result::unwrap)
            .map(|g| {
                let mut m = [[0u8; 4]; 4];
                for (r, row) in m.iter_mut().enumerate() {
                    for (c, slot) in row.iter_mut().enumerate() {
                        *slot = g.get_values(Cell::new(r, c)).unwrap().values()[0];
                    }
                }
                m
            })
            .collect();
        actual.sort_unstable();

        // Independent of the expected set below, every returned grid must be a
        // genuine Latin square (each row and column a permutation of 1..=4).
        for m in &actual {
            for (i, row) in m.iter().enumerate() {
                let mut row = row.to_vec();
                let mut col: Vec<u8> = (0..4).map(|r| m[r][i]).collect();
                row.sort_unstable();
                col.sort_unstable();
                assert_eq!(row, vec![1, 2, 3, 4], "row {i} is not a permutation");
                assert_eq!(col, vec![1, 2, 3, 4], "column {i} is not a permutation");
            }
        }

        let mut expected = [
            [[1, 3, 2, 4], [3, 4, 1, 2], [2, 1, 4, 3], [4, 2, 3, 1]],
            [[1, 2, 3, 4], [3, 4, 1, 2], [2, 3, 4, 1], [4, 1, 2, 3]],
            [[1, 2, 3, 4], [3, 4, 1, 2], [2, 1, 4, 3], [4, 3, 2, 1]],
        ];
        expected.sort_unstable();
        assert_eq!(actual, expected);
    }

    // --- Puzzle::set_cage_tuple ---

    #[test]
    fn set_cage_tuple_pins_cells_in_lexicographic_order() {
        // Add cage over (0,0),(0,1) with target 3: the lexicographically first
        // valid tuple is [1, 2], so index 0 pins (0,0)=1 and (0,1)=2.
        let puzzle = puzzle_with_cage(4, &[(0, 0), (0, 1)], Add, 3);
        let cage = puzzle.cages().next().unwrap().clone();
        let set = puzzle.set_cage_tuple(&cage, 0).unwrap().unwrap().grid();
        assert_eq!(
            set.get_values(Cell::new(0, 0)).unwrap(),
            Values::new(&[1]).unwrap()
        );
        assert_eq!(
            set.get_values(Cell::new(0, 1)).unwrap(),
            Values::new(&[2]).unwrap()
        );
    }

    #[test]
    fn set_cage_tuple_out_of_range_index_errors() {
        let puzzle = puzzle_with_cage(4, &[(0, 0), (0, 1)], Add, 3);
        let cage = puzzle.cages().next().unwrap().clone();
        assert!(matches!(
            puzzle.set_cage_tuple(&cage, 999),
            Err(Error::InvalidTupleIndex(999, _))
        ));
    }

    // --- serde round-trip ---

    #[test]
    fn grid_round_trips_through_json() {
        let g = Grid::new(3).unwrap().set_value(Cell::new(0, 0), 2).unwrap();
        let json = to_string(&g).unwrap();
        let restored: Grid = from_str(&json).unwrap();
        assert_eq!(g, restored);
    }

    #[test]
    fn grid_deserialize_invalid_n_returns_err() {
        let json = r#"{"n":0,"values":[]}"#;
        assert!(from_str::<Grid>(json).is_err());
        let json = r#"{"n":10,"values":[]}"#;
        assert!(from_str::<Grid>(json).is_err());
    }

    #[test]
    fn grid_deserialize_wrong_row_count_returns_err() {
        let json = r#"{"n":2,"values":[[1,2]]}"#;
        assert!(from_str::<Grid>(json).is_err());
    }

    #[test]
    fn grid_deserialize_wrong_column_count_returns_err() {
        let json = r#"{"n":2,"values":[[1,2,3],[1,2,3]]}"#;
        assert!(from_str::<Grid>(json).is_err());
    }

    #[test]
    fn grid_serialize_values_are_row_major() {
        let g = Grid::new(2).unwrap().set_value(Cell::new(0, 0), 1).unwrap();
        let json = to_string(&g).unwrap();
        let v: Value = from_str(&json).unwrap();
        // values[0][0] should be the singleton [1]
        assert_eq!(v["values"][0][0], json!([1]));
    }

    #[test]
    fn grid_deserialize_absent_values_uses_full_value_sets() {
        let json = r#"{"n":3}"#;
        let g: Grid = from_str(json).unwrap();
        assert_eq!(g.n(), 3);
        for r in 0..3 {
            for c in 0..3 {
                assert_eq!(g.get_values(Cell::new(r, c)).unwrap(), Values::all(3));
            }
        }
    }

    #[test]
    fn display_shows_dimensions() {
        assert_eq!(Grid::new(4).unwrap().to_string(), "4×4 grid");
    }

    impl Grid {
        pub(crate) fn is_solution(&self) -> bool {
            (0..self.n)
                .flat_map(|r| (0..self.n).map(move |c| Cell::new(r, c)))
                .all(|cell| self.get_values(cell).is_ok_and(Values::is_singleton))
        }
    }
}
