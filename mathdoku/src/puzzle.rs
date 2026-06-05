//! The [`Puzzle`] type: an `n×n` grid with cage constraints (no cell values).

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Error::CageConflict;
use crate::Error::InfeasibleCage;
use crate::Error::InvalidGridSize;
use crate::Fill;
use crate::cage::Cage;
use crate::cage_fill::CageFill as _;
use crate::{Error, Grid, Polyomino};

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

    /// Returns the cached fill for `cage`, or `None` if `cage` is not in this puzzle.
    #[must_use]
    pub(crate) fn fill(&self, cage: &Cage) -> Option<&crate::cage_fill::CageFillKind> {
        self.cages.get(cage)?.fill()
    }

    /// Returns the propagated grid that reflects all cage and all-different constraints
    /// applied so far. Each cell holds the set of values still consistent with all constraints.
    pub const fn grid(&self) -> Grid {
        self.grid
    }

    /// Returns the current values of `cell` in this puzzle's propagated grid.
    ///
    /// # Errors
    /// Returns [`Error::InvalidCell`] if `cell` is outside the grid.
    pub fn get_values(&self, cell: crate::Cell) -> Result<Fill, Error> {
        self.grid.get_values(cell)
    }

    /// Returns all valid ordered value assignments for `cage`.
    ///
    /// Each tuple assigns one value from `1..=n` to each cell in the cage, in
    /// the cage's cell order, filtered by the current values of each cell.
    /// Tuples are in lexicographic order.
    ///
    /// # Errors
    /// Returns [`Error::InvalidCage`] if `cage` is not in this puzzle.
    pub fn cage_tuples(&self, cage: &Cage) -> Result<Vec<crate::Tuple>, Error> {
        use crate::operation::Operator;
        if !self.cages().any(|c| c == cage) {
            return Err(Error::InvalidCage(cage.clone()));
        }
        let cells = cage.cells();
        let n = self.n();

        let valid = |tuple: &crate::Tuple| {
            let fits_domain = tuple
                .iter()
                .zip(&cells)
                .all(|(&v, &cell)| self.grid.get_values(cell).is_ok_and(|d| d.contains(v)));
            let collinear_ok = (0..cells.len()).all(|i| {
                (0..i).all(|j| {
                    (cells[i].row != cells[j].row && cells[i].column != cells[j].column)
                        || tuple[i] != tuple[j]
                })
            });
            fits_domain && collinear_ok
        };

        if let Some(crate::cage_fill::CageFillKind::Mdd(mdd)) = self.fill(cage) {
            return Ok(mdd.tuples().into_iter().filter(|t| valid(t)).collect());
        }

        let arity = cells.len();
        let op = cage.operation();
        let target = op.target;
        #[allow(clippy::cast_possible_truncation)] // n ≤ 9
        let n_val: crate::N = n as crate::N;
        let mut result = Vec::new();
        let mut tuple: Vec<crate::N> = vec![1; arity];
        loop {
            let satisfies = match op.operator() {
                Operator::Given => arity == 1 && u64::from(tuple[0]) == target,
                Operator::Subtract => {
                    arity == 2 && u64::from(tuple[0]).abs_diff(u64::from(tuple[1])) == target
                }
                Operator::Divide => {
                    arity == 2 && {
                        let (a, b) = (u64::from(tuple[0]), u64::from(tuple[1]));
                        a == b * target || b == a * target
                    }
                }
                _ => false,
            };
            if satisfies && valid(&tuple) {
                result.push(tuple.clone());
            }
            let mut pos = arity - 1;
            loop {
                tuple[pos] += 1;
                if tuple[pos] <= n_val {
                    break;
                }
                tuple[pos] = 1;
                if pos == 0 {
                    return Ok(result);
                }
                pos -= 1;
            }
        }
    }

    /// Returns a new puzzle with `cage` added and all constraints propagated
    /// to a fixpoint.
    ///
    /// Returns `Ok(Some(self.clone()))` if `cage` is already present.
    /// Returns `Ok(None)` if the cage makes the puzzle infeasible.
    ///
    /// # Errors
    /// Returns [`CageConflict`] if `cage`'s polyomino overlaps an existing cage.
    /// Returns [`InfeasibleCage`] if the cage's constraint admits no valid assignment.
    pub fn insert_cage(&self, cage: Cage) -> Result<Option<Self>, Error> {
        if self.cages.contains(&cage) {
            return Ok(Some(self.clone()));
        }
        if self.intersects_cage(cage.polyomino()) {
            return Err(CageConflict(cage));
        }
        let mut cage = cage;
        if cage.build_fill(self.n()).is_empty() {
            return Err(InfeasibleCage(cage.polyomino().clone(), cage.operation()));
        }
        let mut cages = self.cages.clone();
        let _ = cages.insert(cage.clone());
        Ok(Self {
            grid: self.grid,
            cages,
        }
        .fixpoint())
    }

    /// Returns a new puzzle with `cage` removed and all constraints propagated
    /// to a fixpoint.
    ///
    /// Returns `Ok(Some(self.clone()))` if `cage` is not present.
    /// Returns `Ok(None)` if removal makes the puzzle infeasible (should not
    /// occur in practice).
    ///
    /// # Errors
    /// Returns an error if a cell is out of bounds.
    pub fn remove_cage(&self, cage: &Cage) -> Result<Option<Self>, Error> {
        if !self.cages.contains(cage) {
            return Ok(Some(self.clone()));
        }
        let mut cages = self.cages.clone();
        let _ = cages.remove(cage);
        // Widen the removed cage's cells back to full domain before propagating.
        let n = self.n();
        let mut grid = self.grid;
        for cell in cage.cells() {
            grid = grid.set_values(cell, Fill::all(n))?;
        }
        Ok(Self { grid, cages }.fixpoint())
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

    /// Returns an iterator over all solutions for this puzzle.
    ///
    /// Each item is a solved [`Grid`] where every cell's values are a singleton.
    pub fn solutions(&self) -> impl Iterator<Item = Result<Grid, Error>> + '_ {
        crate::solutions::Solutions::new(self)
    }

    /// Propagates all cage and all-different constraints to a GAC fixpoint.
    ///
    /// Returns `None` if any cell's domain becomes empty (infeasible).
    #[must_use]
    pub fn fixpoint(&self) -> Option<Self> {
        let puzzle = Arc::new(self.clone());
        let constraints = Self::constraints(&puzzle);
        let grid = crate::mdk::csp::generalized_arc_consistency(self.grid, &constraints)?;
        Some(Self {
            grid,
            ..self.clone()
        })
    }

    /// Returns a new puzzle with `cell` narrowed to the singleton `{value}` and
    /// all constraints propagated to a fixpoint.
    ///
    /// Returns `None` if the assignment makes the puzzle infeasible, or if `cell`
    /// is outside the grid.
    pub(crate) fn set_value(&self, cell: crate::Cell, value: crate::N) -> Option<Self> {
        let grid = self.grid.set_value(cell, value).ok()?;
        Self {
            grid,
            cages: self.cages.clone(),
        }
        .fixpoint()
    }

    /// Returns a new puzzle with the cells of `cage` set to the values in the
    /// tuple at `index` and all constraints propagated to a fixpoint.
    ///
    /// # Errors
    /// Returns [`Error::InvalidCage`] if `cage` is not in this puzzle, or
    /// [`Error::InvalidTupleIndex`] if `index` is out of range.
    /// Returns `Ok(None)` if the assignment makes the puzzle infeasible.
    pub fn set_cage_tuple(&self, cage: &Cage, index: usize) -> Result<Option<Self>, Error> {
        let tuples = self.cage_tuples(cage)?;
        let tuple = tuples
            .get(index)
            .ok_or(Error::InvalidTupleIndex(index, tuples.len()))?;
        let mut grid = self.grid;
        for (cell, &value) in cage.cells().iter().zip(tuple) {
            grid = grid.set_value(*cell, value)?;
        }
        Ok(Self {
            grid,
            cages: self.cages.clone(),
        }
        .fixpoint())
    }

    /// Builds the full constraint list: one `AllDifferent` per row and column,
    /// plus one cage constraint per cage.
    fn constraints(puzzle: &Arc<Self>) -> Vec<crate::grid_csp::PuzzleConstraint> {
        use crate::grid_csp::{AllDifferent, CageConstraint, PuzzleConstraint};
        let n = puzzle.n();
        let rows = (0..n)
            .map(|r| PuzzleConstraint::AllDifferent(AllDifferent::row(n, r, Arc::clone(puzzle))));
        let cols = (0..n).map(|c| {
            PuzzleConstraint::AllDifferent(AllDifferent::column(n, c, Arc::clone(puzzle)))
        });
        let cages = puzzle.cages().cloned().map(|cage| {
            PuzzleConstraint::Cage(CageConstraint {
                cage,
                puzzle: Arc::clone(puzzle),
            })
        });
        rows.chain(cols).chain(cages).collect()
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
        cages
            .into_iter()
            .try_fold(base, |puzzle, cage| match puzzle.insert_cage(cage) {
                Ok(Some(p)) => Ok(p),
                Ok(None) => Err(serde::de::Error::custom("infeasible cage")),
                Err(e) => Err(serde::de::Error::custom(e)),
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
    use crate::operation::Operator::{self, Add, Given};
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
        let p2 = p.insert_cage(cage).unwrap().unwrap();
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
        let p2 = p.insert_cage(cage.clone()).unwrap().unwrap();
        let p3 = p2.insert_cage(cage).unwrap().unwrap();
        assert_eq!(p2, p3);
    }

    #[test]
    fn insert_cage_overlap_returns_cage_conflict() {
        // A cage at (0,0)+(0,1) is already present; inserting a cage that
        // shares cell (0,0) with a *different* polyomino is a cage conflict.
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(cage_at(&[(0, 0), (0, 1)], Add, 3))
            .unwrap()
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
        let p3 = p
            .insert_cage(c1)
            .unwrap()
            .unwrap()
            .insert_cage(c2)
            .unwrap()
            .unwrap();
        assert_eq!(p3.cages().count(), 2);
    }

    #[test]
    fn insert_cage_subtract_succeeds() {
        let p = Puzzle::new(4).unwrap();
        let cage = cage_at(&[(0, 0), (0, 1)], Operator::Subtract, 1);
        assert!(p.insert_cage(cage).is_ok());
    }

    #[test]
    fn insert_cage_divide_succeeds() {
        let p = Puzzle::new(4).unwrap();
        let cage = cage_at(&[(0, 0), (0, 1)], Operator::Divide, 2);
        assert!(p.insert_cage(cage).is_ok());
    }

    #[test]
    fn insert_cage_multiply_succeeds() {
        let p = Puzzle::new(4).unwrap();
        let cage = cage_at(&[(0, 0), (0, 1)], Operator::Multiply, 6);
        assert!(p.insert_cage(cage).is_ok());
    }

    #[test]
    fn insert_cage_subtract_infeasible_returns_err() {
        // In a 4×4 the max difference is 3; target=9 is impossible.
        let p = Puzzle::new(4).unwrap();
        let cage = cage_at(&[(0, 0), (0, 1)], Operator::Subtract, 9);
        assert!(matches!(p.insert_cage(cage), Err(InfeasibleCage(_, _))));
    }

    #[test]
    fn insert_cage_divide_infeasible_returns_err() {
        // In a 3×3, no pair of distinct values has ratio 9.
        let p = Puzzle::new(3).unwrap();
        let cage = cage_at(&[(0, 0), (0, 1)], Operator::Divide, 9);
        assert!(matches!(p.insert_cage(cage), Err(InfeasibleCage(_, _))));
    }

    #[test]
    fn insert_cage_subtract_propagates_constraints() {
        // Subtract 3 in a 4×4: only (4,1) and (1,4) are valid.
        // After insert, (0,0) and (0,1) should be pruned to {1,4}.
        let p = Puzzle::new(4).unwrap();
        let cage = cage_at(&[(0, 0), (0, 1)], Operator::Subtract, 3);
        let p2 = p.insert_cage(cage).unwrap().unwrap();
        assert_eq!(
            p2.get_values(Cell::new(0, 0)).unwrap(),
            Fill::new(&[1, 4]).unwrap()
        );
        assert_eq!(
            p2.get_values(Cell::new(0, 1)).unwrap(),
            Fill::new(&[1, 4]).unwrap()
        );
    }

    #[test]
    fn insert_cage_divide_propagates_constraints() {
        // Divide 3 in a 3×3: only (3,1) and (1,3) are valid.
        let p = Puzzle::new(3).unwrap();
        let cage = cage_at(&[(0, 0), (0, 1)], Operator::Divide, 3);
        let p2 = p.insert_cage(cage).unwrap().unwrap();
        assert_eq!(
            p2.get_values(Cell::new(0, 0)).unwrap(),
            Fill::new(&[1, 3]).unwrap()
        );
    }

    // --- Puzzle::remove_cage ---

    #[test]
    fn remove_cage_removes_present_cage() {
        let cage = cage_at(&[(0, 0)], Given, 1);
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(cage.clone())
            .unwrap()
            .unwrap();
        let p2 = p.remove_cage(&cage).unwrap().unwrap();
        assert_eq!(p2.cages().count(), 0);
    }

    #[test]
    fn remove_cage_absent_returns_self() {
        let cage = cage_at(&[(0, 0)], Given, 1);
        let p = Puzzle::new(4).unwrap();
        let p2 = p.remove_cage(&cage).unwrap().unwrap();
        assert_eq!(p, p2);
    }

    #[test]
    fn remove_cage_is_non_destructive() {
        let cage = cage_at(&[(0, 0)], Given, 1);
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(cage.clone())
            .unwrap()
            .unwrap();
        let _ = p.remove_cage(&cage);
        assert_eq!(p.cages().count(), 1);
    }

    // --- Puzzle::get_cage_at ---

    #[test]
    fn get_cage_at_returns_cage_for_present_polyomino() {
        let cage = cage_at(&[(0, 0), (0, 1)], Add, 3);
        let p = Puzzle::new(4)
            .unwrap()
            .insert_cage(cage.clone())
            .unwrap()
            .unwrap();
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
            .unwrap()
            .insert_cage(c2.clone())
            .unwrap()
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
            .unwrap()
            .insert_cage(cage_at(&[(0, 2)], Given, 3))
            .unwrap()
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
        let p = p.insert_cage(cage).unwrap().unwrap();
        assert_eq!(p.to_string(), "4×4 puzzle, 1 cages");
    }

    #[test]
    fn puzzle_deserialize_invalid_n_returns_err() {
        let json = r#"{"n":0,"cages":[]}"#;
        assert!(from_str::<Puzzle>(json).is_err());
        let json = r#"{"n":10,"cages":[]}"#;
        assert!(from_str::<Puzzle>(json).is_err());
    }

    // --- Puzzle::cage_tuples ---

    #[test]
    fn cage_tuples_returns_valid_tuples() {
        let cage = cage_at(&[(0, 0), (0, 1)], Add, 3);
        let puzzle = Puzzle::new(4)
            .unwrap()
            .insert_cage(cage.clone())
            .unwrap()
            .unwrap();
        let tuples = puzzle.cage_tuples(&cage).unwrap();
        assert!(!tuples.is_empty());
        for t in &tuples {
            let sum: crate::Target = t.iter().map(|&v| crate::Target::from(v)).sum();
            assert_eq!(sum, 3);
        }
    }

    #[test]
    fn cage_tuples_invalid_cage_returns_err() {
        let puzzle = Puzzle::new(4).unwrap();
        let cage = cage_at(&[(0, 0)], Given, 1);
        assert!(matches!(
            puzzle.cage_tuples(&cage),
            Err(Error::InvalidCage(_))
        ));
    }

    // --- Puzzle::fixpoint ---

    #[test]
    fn fixpoint_given_cages_pin_cells() {
        // A 2×2 with Given cages fully determines every cell.
        let puzzle = Puzzle::new(2)
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
            .unwrap();
        let p = puzzle.fixpoint().unwrap();
        assert_eq!(
            p.get_values(Cell::new(0, 0)).unwrap(),
            Fill::new(&[1]).unwrap()
        );
        assert_eq!(
            p.get_values(Cell::new(0, 1)).unwrap(),
            Fill::new(&[2]).unwrap()
        );
        assert_eq!(
            p.get_values(Cell::new(1, 0)).unwrap(),
            Fill::new(&[2]).unwrap()
        );
        assert_eq!(
            p.get_values(Cell::new(1, 1)).unwrap(),
            Fill::new(&[1]).unwrap()
        );
    }

    #[test]
    fn fixpoint_is_idempotent() {
        let puzzle = Puzzle::new(2)
            .unwrap()
            .insert_cage(cage_at(&[(0, 0)], Given, 1))
            .unwrap()
            .unwrap();
        let p1 = puzzle.fixpoint().unwrap();
        let p2 = p1.fixpoint().unwrap();
        assert_eq!(p1.grid(), p2.grid());
    }

    #[test]
    fn fixpoint_no_cages_is_identity() {
        let puzzle = Puzzle::new(3).unwrap();
        let p = puzzle.fixpoint().unwrap();
        for r in 0..3 {
            for c in 0..3 {
                assert_eq!(p.get_values(Cell::new(r, c)).unwrap(), Fill::all(3));
            }
        }
    }

    #[test]
    fn fixpoint_arithmetic_cages_prune_values() {
        // 2×2 with Add=3 and Divide=2: all cells must hold {1,2}.
        let puzzle = Puzzle::new(2)
            .unwrap()
            .insert_cage(cage_at(&[(0, 0), (0, 1)], Add, 3))
            .unwrap()
            .unwrap()
            .insert_cage(cage_at(&[(1, 0), (1, 1)], Operator::Divide, 2))
            .unwrap()
            .unwrap();
        let p = puzzle.fixpoint().unwrap();
        let expected = Fill::new(&[1, 2]).unwrap();
        for r in 0..2 {
            for c in 0..2 {
                assert_eq!(
                    p.get_values(Cell::new(r, c)).unwrap(),
                    expected,
                    "cell ({r},{c}) should be pruned to {{1,2}}"
                );
            }
        }
    }
}
