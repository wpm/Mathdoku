//! The [`Puzzle`] type: an `n×n` grid with cage constraints (no cell values).

use std::collections::{BTreeSet, HashMap};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Error::CageConflict;
use crate::Error::InfeasibleOperation;
use crate::Error::InvalidGridSize;
use crate::cage::Cage;
use crate::mdd::{MonotonicMDD, build_mdd};
use crate::{Error, Polyomino};

// Serde wire format — only n and cages cross the wire; the MDD is rebuilt on load.
#[derive(Serialize, Deserialize)]
struct PuzzleWire {
    n: usize,
    #[serde(default)]
    cages: BTreeSet<Cage>,
}

/// An `n×n` Mathdoku puzzle defined by its cage constraints.
///
/// A `Puzzle` stores the grid size, the set of cages, and a pre-built MDD for
/// each cage. The MDD map participates in neither equality, ordering, hashing,
/// nor serialization: two puzzles are equal when their `n` and `cages` match.
/// The MDD is rebuilt from the cages on deserialization.
///
/// Cell values live in [`Grid`].
///
/// [`Grid`]: crate::Grid
#[derive(Debug, Clone)]
pub struct Puzzle {
    n: usize,
    cages: BTreeSet<Cage>,
    /// Per-cage MDD, keyed by cage. Not serialized; rebuilt from `cages` on load.
    mdd: HashMap<Cage, MonotonicMDD>,
}

impl PartialEq for Puzzle {
    fn eq(&self, other: &Self) -> bool {
        self.n == other.n && self.cages == other.cages
    }
}

impl Eq for Puzzle {}

impl Hash for Puzzle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.n.hash(state);
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
            n,
            cages: BTreeSet::new(),
            mdd: HashMap::new(),
        })
    }

    /// Returns the grid size `n` (puzzle is `n`×`n`).
    #[must_use]
    pub const fn n(&self) -> usize {
        self.n
    }

    /// Returns an iterator over all cages in this puzzle in polyomino order.
    pub fn cages(&self) -> impl Iterator<Item = &Cage> {
        self.cages.iter()
    }

    /// Returns the MDD for `cage`, or `None` if `cage` is not in this puzzle.
    #[must_use]
    pub fn mdd(&self, cage: &Cage) -> Option<&MonotonicMDD> {
        self.mdd.get(cage)
    }

    /// Returns a new puzzle with `cage` added.
    ///
    /// Returns `Ok(self.clone())` if `cage` is already present. Does not
    /// propagate constraints — call [`Grid::constrain`] separately to apply the
    /// new cage's constraints to a grid.
    ///
    /// [`Grid::constrain`]: crate::Grid::constrain
    ///
    /// # Errors
    /// Returns [`CageConflict`] if `cage`'s polyomino overlaps an existing cage's
    /// polyomino (but not if the cage is already present).
    /// Returns [`InfeasibleOperation`] if the cage's constraint admits no valid
    /// assignment at this grid size, which would collapse its cells to empty domains.
    pub fn insert_cage(&self, cage: Cage) -> Result<Self, Error> {
        if self.cages.contains(&cage) {
            return Ok(self.clone());
        }
        if self.intersects_cage(cage.polyomino()) {
            return Err(CageConflict(cage));
        }
        let mut cages = self.cages.clone();
        let mut mdd_map = self.mdd.clone();
        if let Some(mdd) = build_mdd(self.n, &cage) {
            if mdd.is_empty() {
                return Err(InfeasibleOperation(
                    cage.polyomino().clone(),
                    cage.operation(),
                ));
            }
            let _ = mdd_map.insert(cage.clone(), mdd);
        }
        let _ = cages.insert(cage);
        Ok(Self {
            n: self.n,
            cages,
            mdd: mdd_map,
        })
    }

    /// Returns a new puzzle with `cage` removed.
    ///
    /// Returns `self` unchanged if `cage` is not present.
    #[must_use]
    pub fn remove_cage(&self, cage: &Cage) -> Self {
        let mut cages = self.cages.clone();
        let mut mdd_map = self.mdd.clone();
        let _ = cages.remove(cage);
        let _ = mdd_map.remove(cage);
        Self {
            n: self.n,
            cages,
            mdd: mdd_map,
        }
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

    /// Build the MDD map for a set of cages at grid size `n`.
    fn build_mdd_map(n: usize, cages: &BTreeSet<Cage>) -> HashMap<Cage, MonotonicMDD> {
        cages
            .iter()
            .filter_map(|cage| build_mdd(n, cage).map(|mdd| (cage.clone(), mdd)))
            .collect()
    }
}

impl Serialize for Puzzle {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        PuzzleWire {
            n: self.n,
            cages: self.cages.clone(),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for Puzzle {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let wire = PuzzleWire::deserialize(d)?;
        let n = wire.n;
        if !(1..=9).contains(&n) {
            return Err(DeError::custom(format!("invalid grid size {n}")));
        }
        let mdd = Self::build_mdd_map(n, &wire.cages);
        Ok(Self {
            n,
            cages: wire.cages,
            mdd,
        })
    }
}

impl Display for Puzzle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}×{} puzzle, {} cages",
            self.n,
            self.n,
            self.cages.len()
        )
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
        assert!(matches!(
            p.insert_cage(cage),
            Err(InfeasibleOperation(_, _))
        ));
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
        let p2 = p.remove_cage(&cage);
        assert_eq!(p2.cages().count(), 0);
    }

    #[test]
    fn remove_cage_absent_returns_self() {
        let cage = cage_at(&[(0, 0)], Given, 1);
        let p = Puzzle::new(4).unwrap();
        let p2 = p.remove_cage(&cage);
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
            .insert_cage(cage_at(&[(0, 2)], Given, 2))
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
