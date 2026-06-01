//! The [`Puzzle`] type: an `n×n` grid with cage constraints (no cell values).

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Error::CageConflict;
use crate::Error::InfeasibleCage;
use crate::Error::InvalidGridSize;
use crate::cage::Cage;
use crate::{Error, Grid, Polyomino, Values};

// Serde wire format. Two variants are accepted on deserialization:
// - `{"grid": {...}, "cages": [...]}` — full grid state
// - `{"n": 2, "cages": [...]}` — grid size only; deserializes as a maximally unconstrained grid
#[derive(Serialize)]
struct PuzzleWire {
    grid: Grid,
    #[serde(default)]
    cages: BTreeSet<Cage>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PuzzleWireIn {
    WithGrid {
        grid: Grid,
        #[serde(default)]
        cages: BTreeSet<Cage>,
    },
    WithN {
        n: usize,
        #[serde(default)]
        cages: BTreeSet<Cage>,
    },
}

/// An `n×n` Mathdoku puzzle defined by its cage constraints.
///
/// Each [`Cage`] carries its own pre-built MDD (skipped by serde). Two puzzles
/// are equal when their `n` and `cages` match.
///
/// Cell values live in [`Grid`].
#[must_use]
#[derive(Debug, Clone)]
pub struct Puzzle {
    grid: Grid,
    cages: BTreeSet<Cage>,
}

impl PartialEq for Puzzle {
    fn eq(&self, other: &Self) -> bool {
        self.n() == other.n() && self.cages == other.cages
    }
}

impl Eq for Puzzle {}

impl Hash for Puzzle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.grid.hash(state);
        self.cages.hash(state);
    }
}

impl Puzzle {
    /// Creates an empty `n×n` puzzle with no cages.
    ///
    /// # Errors
    /// Returns [`InvalidGridSize`] if `n` is not in `1..=9`.
    pub fn new(n: usize) -> Result<Self, Error> {
        if !(1..=9).contains(&n) {
            return Err(InvalidGridSize(n));
        }
        Ok(Self {
            grid: Grid::new(n)?,
            cages: BTreeSet::new(),
        })
    }

    /// Returns the grid size `n` (puzzle is `n`×`n`).
    #[must_use]
    pub const fn n(&self) -> usize {
        self.grid.n()
    }

    /// Returns an iterator over all cages in this puzzle in polyomino order.
    pub fn cages(&self) -> impl Iterator<Item = &Cage> {
        self.cages.iter()
    }

    /// Returns the cached MDD for `cage`, or `None` if `cage` is not in this
    /// puzzle or uses a non-monotonic operator.
    #[must_use]
    pub fn mdd(&self, cage: &Cage) -> Option<&crate::mdd::MonotonicMDD> {
        self.cages.get(cage)?.mdd()
    }

    /// Returns the current grid state for this puzzle.
    pub const fn grid(&self) -> Grid {
        self.grid
    }

    /// Returns a new puzzle with `cage` added and all constraints propagated
    /// to a fixpoint.
    ///
    /// Returns `Ok(self.clone())` if `cage` is already present.
    ///
    /// # Errors
    /// Returns [`CageConflict`] if `cage`'s polyomino overlaps an existing cage.
    /// Returns [`InfeasibleCage`] if the cage's constraint admits no valid
    /// assignment, or if propagation empties any cage cell's domain.
    pub fn insert_cage(&self, cage: Cage) -> Result<Self, Error> {
        if self.cages.contains(&cage) {
            return Ok(self.clone());
        }
        if self.intersects_cage(cage.polyomino()) {
            return Err(CageConflict(cage));
        }
        let mut cage = cage;
        if let Some(mdd) = cage.build_mdd(self.n()) {
            if mdd.is_empty() {
                return Err(InfeasibleCage(cage.polyomino().clone(), cage.operation()));
            }
        }
        let mut cages = self.cages.clone();
        let _ = cages.insert(cage.clone());
        let candidate = Self {
            grid: self.grid,
            cages,
        };
        let constrained = candidate.constrain()?;
        if cage.cells().iter().any(|&cell| {
            constrained
                .grid
                .cell_values(cell)
                .is_ok_and(Values::is_empty)
        }) {
            return Err(InfeasibleCage(cage.polyomino().clone(), cage.operation()));
        }
        Ok(constrained)
    }

    /// Returns a new puzzle with `cage` removed and all constraints propagated
    /// to a fixpoint.
    ///
    /// Returns `self` unchanged if `cage` is not present.
    ///
    /// # Errors
    /// Returns an error if propagation fails (e.g. the puzzle is ill-formed).
    pub fn remove_cage(&self, cage: &Cage) -> Result<Self, Error> {
        if !self.cages.contains(cage) {
            return Ok(self.clone());
        }
        let mut cages = self.cages.clone();
        let _ = cages.remove(cage);
        // Widen the removed cage's cells back to full domain before propagating.
        let n = self.n();
        let mut grid = self.grid;
        for cell in cage.cells() {
            grid = grid.set_values(cell, Values::all(n))?;
        }
        let candidate = Self { grid, cages };
        candidate.constrain()
    }

    /// Returns the cage covering exactly the cells of `polyomino`, or `None`.
    #[must_use]
    pub fn get_cage_at(&self, polyomino: &Polyomino) -> Option<&Cage> {
        self.cages.iter().find(|cage| cage.polyomino() == polyomino)
    }

    fn intersects_cage(&self, polyomino: &Polyomino) -> bool {
        self.cages
            .iter()
            .any(|cage| cage.polyomino().intersects(polyomino))
    }

    /// Propagates all cage and all-different constraints from this puzzle
    /// onto `grid` and returns the constrained result.
    ///
    /// Useful when the caller has a starting grid (e.g. a fixed Latin-square
    /// solution) that should be narrowed by the current cage structure.
    ///
    /// # Errors
    /// Returns [`InvalidGridSize`] if `grid.n() != self.n()`, or an error if
    /// propagation empties any cell's domain.
    pub fn constrain_grid(&self, grid: &Grid) -> Result<Grid, Error> {
        grid.constrain(self)
    }

    /// Runs all row, column, and cage constraints to a GAC fixpoint on `grid`.
    ///
    /// Assembles the constraint list via [`crate::grid_csp::puzzle_constraints`] and
    /// delegates to [`crate::grid_csp::run_to_fixpoint`].
    ///
    /// # Errors
    /// Returns an error if propagation fails (e.g. a cell is out of bounds).
    pub(crate) fn fixpoint(&self, grid: &Grid) -> Result<Grid, Error> {
        let puzzle = Arc::new(self.clone());
        let constraints = crate::grid_csp::puzzle_constraints(&puzzle);
        crate::grid_csp::run_to_fixpoint(*grid, &constraints)
    }

    /// Returns an iterator over all solutions for this puzzle.
    ///
    /// Uses the puzzle's propagated grid as the starting state. Each item is a
    /// solved [`Grid`] where every cell's values are a singleton.
    pub fn solutions(&self) -> impl Iterator<Item = Result<Grid, Error>> + '_ {
        crate::solutions::Solutions::new(&self.grid, self)
    }

    /// Propagate all constraints to a fixpoint and return the updated puzzle.
    fn constrain(&self) -> Result<Self, Error> {
        let grid = self.fixpoint(&self.grid)?;
        Ok(Self {
            grid,
            ..self.clone()
        })
    }
}

/// Compact wire form used when the grid is maximally unconstrained.
#[derive(Serialize)]
struct PuzzleWireN {
    n: usize,
    #[serde(default)]
    cages: BTreeSet<Cage>,
}

impl Serialize for Puzzle {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let n = self.grid.n();
        if Grid::new(n).is_ok_and(|full| full == self.grid) {
            PuzzleWireN {
                n,
                cages: self.cages.clone(),
            }
            .serialize(s)
        } else {
            PuzzleWire {
                grid: self.grid,
                cages: self.cages.clone(),
            }
            .serialize(s)
        }
    }
}

impl<'de> Deserialize<'de> for Puzzle {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let (grid, cages) = match PuzzleWireIn::deserialize(d)? {
            PuzzleWireIn::WithGrid { grid, cages } => (grid, cages),
            PuzzleWireIn::WithN { n, cages } => {
                if !(1..=9).contains(&n) {
                    return Err(serde::de::Error::custom(format!("invalid grid size {n}")));
                }
                (Grid::new(n).map_err(serde::de::Error::custom)?, cages)
            }
        };
        let base = Self {
            grid,
            cages: BTreeSet::new(),
        };
        cages.into_iter().try_fold(base, |puzzle, cage| {
            puzzle.insert_cage(cage).map_err(serde::de::Error::custom)
        })
    }
}

impl Display for Puzzle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let n = self.n();
        write!(f, "{}×{} puzzle, {} cages", n, n, self.cages.len())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{from_str, to_string};

    use super::*;
    use crate::Cell;
    use crate::operation::Operator::{Add, Given};
    use crate::polyomino::Polyomino;
    use crate::test_utils::cage_at;

    // --- Puzzle::new ---

    #[test]
    fn new_valid_sizes_succeed() {
        for n in 1..=9 {
            assert!(Puzzle::new(n).is_ok(), "size {n} should succeed");
        }
    }

    #[test]
    fn new_size_zero_returns_err() {
        assert!(matches!(Puzzle::new(0), Err(InvalidGridSize(0))));
    }

    #[test]
    fn new_size_ten_returns_err() {
        assert!(matches!(Puzzle::new(10), Err(InvalidGridSize(10))));
    }

    #[test]
    fn new_has_no_cages() {
        let p = Puzzle::new(4).unwrap();
        assert_eq!(p.cages().count(), 0);
    }

    // --- Puzzle::insert_cage ---

    #[test]
    fn insert_cage_returns_puzzle() {
        let p = Puzzle::new(4).unwrap();
        let cage = cage_at(&[(0, 0)], Given, 3);
        let p2 = p.insert_cage(cage).unwrap();
        assert_eq!(p2.n(), 4);
    }

    #[test]
    fn insert_cage_is_non_destructive() {
        let p = Puzzle::new(4).unwrap();
        let cage = cage_at(&[(0, 0)], Given, 3);
        let _ = p.insert_cage(cage);
        // Original puzzle unchanged — still has no cages.
        assert_eq!(p.cages().count(), 0);
    }

    #[test]
    fn insert_cage_duplicate_returns_self() {
        let p = Puzzle::new(4).unwrap();
        let cage = cage_at(&[(0, 0)], Given, 3);
        let p2 = p.insert_cage(cage.clone()).unwrap();
        let p3 = p2.insert_cage(cage).unwrap();
        assert_eq!(p2, p3);
    }

    #[test]
    fn insert_cage_overlap_returns_cage_conflict() {
        // A cage at (0,0)+(0,1) is already present; inserting a cage that
        // shares cell (0,0) with a *different* polyomino is a cage conflict.
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(cage_at(&[(0, 0), (0, 1)], Add, 3))
            .unwrap();
        // This cage shares cell (0,0) with the existing cage but has a different polyomino.
        let overlapping = cage_at(&[(0, 0)], Given, 1);
        assert!(matches!(p.insert_cage(overlapping), Err(CageConflict(_))));
    }

    #[test]
    fn insert_cage_infeasible_target_returns_infeasible_operation() {
        // On a 3×3 grid, two cells cannot sum to 7 (max is 3+3=6).
        let p = Puzzle::new(3).unwrap();
        let cage = cage_at(&[(0, 0), (0, 1)], Add, 7);
        assert!(matches!(p.insert_cage(cage), Err(InfeasibleCage(_, _))));
    }

    #[test]
    fn insert_cage_accumulates_cages() {
        let p = Puzzle::new(4).unwrap();
        let c1 = cage_at(&[(0, 0)], Given, 1);
        let c2 = cage_at(&[(0, 1)], Given, 2);
        let p3 = p.insert_cage(c1).unwrap().insert_cage(c2).unwrap();
        assert_eq!(p3.cages().count(), 2);
    }

    // --- Puzzle::remove_cage ---

    #[test]
    fn remove_cage_removes_present_cage() {
        let cage = cage_at(&[(0, 0)], Given, 1);
        let p = Puzzle::new(4).unwrap().insert_cage(cage.clone()).unwrap();
        let p2 = p.remove_cage(&cage).unwrap();
        assert_eq!(p2.cages().count(), 0);
    }

    #[test]
    fn remove_cage_absent_returns_self() {
        let cage = cage_at(&[(0, 0)], Given, 1);
        let p = Puzzle::new(4).unwrap();
        let p2 = p.remove_cage(&cage).unwrap();
        assert_eq!(p, p2);
    }

    #[test]
    fn remove_cage_is_non_destructive() {
        let cage = cage_at(&[(0, 0)], Given, 1);
        let p = Puzzle::new(4).unwrap().insert_cage(cage.clone()).unwrap();
        let _ = p.remove_cage(&cage);
        assert_eq!(p.cages().count(), 1);
    }

    // --- Puzzle::get_cage_at ---

    #[test]
    fn get_cage_at_returns_cage_for_present_polyomino() {
        let cage = cage_at(&[(0, 0), (0, 1)], Add, 3);
        let p = Puzzle::new(4).unwrap().insert_cage(cage.clone()).unwrap();
        let poly = Polyomino::from_cells(&[Cell::new(0, 0), Cell::new(0, 1)]).unwrap();
        assert_eq!(p.get_cage_at(&poly), Some(&cage));
    }

    #[test]
    fn get_cage_at_returns_none_for_absent_polyomino() {
        let p = Puzzle::new(4).unwrap();
        let poly = Polyomino::from_cells(&[Cell::new(0, 0)]).unwrap();
        assert!(p.get_cage_at(&poly).is_none());
    }

    // --- Puzzle::cages ---

    #[test]
    fn cages_returns_all_inserted_cages() {
        let c1 = cage_at(&[(0, 0)], Given, 1);
        let c2 = cage_at(&[(0, 1)], Given, 2);
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(c1.clone())
            .unwrap()
            .insert_cage(c2.clone())
            .unwrap();
        let cages: Vec<_> = p.cages().cloned().collect();
        assert!(cages.contains(&c1));
        assert!(cages.contains(&c2));
    }

    // --- serde round-trip ---

    #[test]
    fn puzzle_round_trips_through_json() {
        let p = Puzzle::new(3)
            .unwrap()
            .insert_cage(cage_at(&[(0, 0), (0, 1)], Add, 3))
            .unwrap()
            .insert_cage(cage_at(&[(0, 2)], Given, 3))
            .unwrap();
        let json = to_string(&p).unwrap();
        let restored: Puzzle = from_str(&json).unwrap();
        assert_eq!(p, restored);
    }

    #[test]
    fn display_shows_dimensions_and_cage_count() {
        let p = Puzzle::new(4).unwrap();
        assert_eq!(p.to_string(), "4×4 puzzle, 0 cages");
        let cage = cage_at(&[(0, 0)], Given, 1);
        let p = p.insert_cage(cage).unwrap();
        assert_eq!(p.to_string(), "4×4 puzzle, 1 cages");
    }

    #[test]
    fn puzzle_deserialize_invalid_n_returns_err() {
        let json = r#"{"n":0,"cages":[]}"#;
        assert!(from_str::<Puzzle>(json).is_err());
        let json = r#"{"n":10,"cages":[]}"#;
        assert!(from_str::<Puzzle>(json).is_err());
    }
}
