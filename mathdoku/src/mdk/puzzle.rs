//! [`Puzzle`]: the top-level constraint-solving interface.
use crate::mdk::Error::MissingCell;
use crate::mdk::cage::Cage;
use crate::mdk::csp::{Constraint, generalized_arc_consistency};
use crate::mdk::fill::Fill;
use crate::mdk::grid::{AllDifferent, Grid};
use crate::mdk::polyomino::{Cell, Polyomino};
use crate::mdk::{Error, N, T};
use std::collections::HashMap;
use std::sync::Arc;

/// A Mathdoku puzzle: an n×n grid partitioned into cages, each with an arithmetic constraint.
#[derive(Clone)]
pub struct Puzzle {
    grid: Grid,
    cages: HashMap<Cell, Arc<Cage>>,
}

/// A constraint that applies to a [`Puzzle`]'s grid: either a cage or an all-different.
#[derive(Clone)]
enum PuzzleConstraint {
    Cage(Arc<Cage>),
    AllDifferent(AllDifferent),
}

impl Constraint<Grid, Cell, Fill, Error> for PuzzleConstraint {
    fn propagate(&self, state: &Grid) -> Result<(Grid, Vec<Cell>), Error> {
        match self {
            Self::Cage(cage) => cage.propagate(state),
            Self::AllDifferent(ad) => ad.propagate(state),
        }
    }

    fn in_scope(&self, variable: Cell) -> bool {
        match self {
            Self::Cage(cage) => cage.in_scope(variable),
            Self::AllDifferent(ad) => ad.in_scope(variable),
        }
    }
}

impl Puzzle {
    /// Returns the candidate fill for `cell`.
    ///
    /// # Errors
    ///
    /// Returns [`MissingCell`] if `cell` is not in the puzzle.
    pub fn get(&self, cell: Cell) -> Result<Fill, Error> {
        self.grid.get(cell)
    }

    /// # Errors
    ///
    /// Returns an error if `cell` is not in the puzzle or `n` is not a candidate value for it.
    #[allow(clippy::todo)]
    pub fn set(&self, cell: Cell, n: N) -> Result<Self, Error> {
        let fill = self.grid.get(cell)?;
        if !fill.contains(n) {
            return Err(Error::InvalidCellValue(cell, n));
        }
        Ok(Self {
            grid: self.grid.set(cell, Fill::from(&[n])),
            cages: self.cages.clone(),
        })
    }

    /// Returns a copy of the puzzle with a new cage added, propagated to a fixpoint.
    ///
    /// Returns `None` if the new cage makes the puzzle infeasible.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MissingPolyomino`] if any cell of `polyomino` is not in the grid.
    /// Returns [`Error::NonDisjointPolyominoes`] if `polyomino` overlaps an existing cage.
    pub fn insert(
        &self,
        polyomino: &Polyomino,
        operation: CageOperator,
        target: T,
    ) -> Result<Option<Self>, Error> {
        let n = self.grid.size();

        // Check all cells are within the grid bounds.
        if polyomino
            .iter()
            .any(|&Cell(r, c)| r < 1 || r > n || c < 1 || c > n)
        {
            return Err(Error::MissingPolyomino(polyomino.clone()));
        }

        // Check disjoint with every existing cage.
        let mut seen: std::collections::HashSet<*const Cage> = std::collections::HashSet::new();
        for arc in self.cages.values() {
            if seen.insert(Arc::as_ptr(arc)) && !arc.polyomino.is_disjoint(polyomino) {
                return Err(Error::NonDisjointPolyominoes(
                    arc.polyomino.clone(),
                    polyomino.clone(),
                ));
            }
        }

        // Build the cage.
        let cage = match operation {
            CageOperator::Add => Cage::commutative(
                n,
                polyomino.clone(),
                crate::mdk::operator::CommutativeOperator::Add,
                target,
            )?,
            CageOperator::Multiply => Cage::commutative(
                n,
                polyomino.clone(),
                crate::mdk::operator::CommutativeOperator::Multiply,
                target,
            )?,
            CageOperator::Subtract => Cage::non_commutative(
                n,
                polyomino.clone(),
                crate::mdk::operator::NonCommutativeOperator::Subtract,
                target,
            )?,
            CageOperator::Divide => Cage::non_commutative(
                n,
                polyomino.clone(),
                crate::mdk::operator::NonCommutativeOperator::Divide,
                target,
            )?,
            CageOperator::Given => {
                let &cell = polyomino
                    .iter()
                    .next()
                    .ok_or_else(|| Error::MissingPolyomino(polyomino.clone()))?;
                #[allow(clippy::cast_possible_truncation)]
                Cage::given(cell, target as N)?
            }
        };

        // Insert into a cloned cage map.
        let mut cages = self.cages.clone();
        let arc = Arc::new(cage);
        for &cell in polyomino.iter() {
            let _ = cages.insert(cell, Arc::clone(&arc));
        }

        Ok(Self {
            grid: self.grid.clone(),
            cages,
        }
        .fixpoint())
    }

    /// Returns a copy of the puzzle with `cage` removed.
    ///
    /// # Errors
    ///
    /// Returns an error if `cage` is not in the puzzle.
    pub fn remove(&self, cage: &Cage) -> Result<Option<Self>, Error> {
        let mut cages = self.cages.clone();
        for cell in cage.polyomino.iter() {
            let _ = cages.remove(cell).ok_or(MissingCell(*cell));
        }
        Ok(self.fixpoint())
    }

    /// Returns the operators that are feasible for `polyomino` given the current grid state.
    ///
    /// An operation is feasible if at least one target value exists that is consistent
    /// with the candidate fills of the polyomino's cells.
    ///
    /// # Errors
    ///
    /// Returns [`MissingCell`] if any cell of `polyomino` is not in the puzzle.
    #[allow(clippy::todo)]
    pub fn possible_operations(&self, _polyomino: &Polyomino) -> Result<Vec<CageOperator>, Error> {
        todo!()
    }

    /// Returns the target values that are feasible for `polyomino` under `operation`
    /// given the current grid state.
    ///
    /// A target is feasible if some assignment of values from the cells' candidate fills
    /// satisfies `operation` with that target.
    ///
    /// # Errors
    ///
    /// Returns [`MissingCell`] if any cell of `polyomino` is not in the puzzle.
    #[allow(clippy::todo)]
    pub fn possible_targets(
        &self,
        _polyomino: &Polyomino,
        _operation: CageOperator,
    ) -> Result<Vec<T>, Error> {
        todo!()
    }

    /// Builds a [`Puzzle`] from a grid and a list of cages.
    ///
    /// Each cage's cells are mapped to a shared [`Arc`] in the cell→cage index.
    pub(crate) fn from_parts(grid: Grid, cage_list: Vec<Cage>) -> Self {
        let mut cages: HashMap<Cell, Arc<Cage>> = HashMap::new();
        for cage in cage_list {
            let arc = Arc::new(cage);
            for &cell in arc.polyomino.iter() {
                let _ = cages.insert(cell, Arc::clone(&arc));
            }
        }
        Self { grid, cages }
    }

    /// Propagates all cage and all-different constraints to a GAC fixpoint.
    ///
    /// Returns `None` if any cell's domain becomes empty (infeasible).
    #[must_use]
    pub fn fixpoint(&self) -> Option<Self> {
        let n = self.grid.size();
        // Deduplicate cages by pointer: each cage Arc is shared across all its cells.
        let mut seen: std::collections::HashSet<*const Cage> = std::collections::HashSet::new();
        let mut constraints: Vec<PuzzleConstraint> = self
            .cages
            .values()
            .filter(|c| seen.insert(Arc::as_ptr(c)))
            .map(|c| PuzzleConstraint::Cage(Arc::clone(c)))
            .collect();
        for i in 1..=n {
            constraints.push(PuzzleConstraint::AllDifferent(AllDifferent::row(n, i)));
            constraints.push(PuzzleConstraint::AllDifferent(AllDifferent::column(n, i)));
        }
        let grid = generalized_arc_consistency(self.grid.clone(), &constraints)?;
        Some(Self {
            grid,
            cages: self.cages.clone(),
        })
    }
}

#[derive(Clone, Copy)]
pub enum CageOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Given,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdk::operator::CommutativeOperator::Add;
    use crate::mdk::operator::NonCommutativeOperator::Subtract;
    use crate::mdk::polyomino::Polyomino;

    fn domino(r0: usize, c0: usize, r1: usize, c1: usize) -> Polyomino {
        Polyomino::from([Cell(r0, c0), Cell(r1, c1)]).unwrap()
    }

    #[test]
    fn insert_cage_pins_cell() {
        let p = Puzzle::from_parts(Grid::new(4).unwrap(), vec![]);
        let poly = Polyomino::from([Cell(1, 1)]).unwrap();
        let fp = p.insert(&poly, CageOperator::Given, 3).unwrap().unwrap();
        assert_eq!(fp.get(Cell(1, 1)).unwrap(), Fill::from(&[3]));
    }

    #[test]
    fn insert_missing_polyomino_returns_error() {
        let p = Puzzle::from_parts(Grid::new(2).unwrap(), vec![]);
        let poly = Polyomino::from([Cell(9, 9)]).unwrap();
        assert!(matches!(
            p.insert(&poly, CageOperator::Given, 1),
            Err(Error::MissingPolyomino(_))
        ));
    }

    #[test]
    fn insert_overlapping_cage_returns_error() {
        let cage = Cage::given(Cell(1, 1), 1).unwrap();
        let p = Puzzle::from_parts(Grid::new(2).unwrap(), vec![cage]);
        let poly = Polyomino::from([Cell(1, 1)]).unwrap();
        assert!(matches!(
            p.insert(&poly, CageOperator::Given, 2),
            Err(Error::NonDisjointPolyominoes(_, _))
        ));
    }

    #[test]
    fn insert_infeasible_cage_returns_none() {
        // pin (1,1)=1 and (1,2)=1 in a 2×2 — AllDifferent makes it infeasible
        let p = Puzzle::from_parts(Grid::new(2).unwrap(), vec![]);
        let p = p
            .insert(
                &Polyomino::from([Cell(1, 1)]).unwrap(),
                CageOperator::Given,
                1,
            )
            .unwrap()
            .unwrap();
        let poly = Polyomino::from([Cell(1, 2)]).unwrap();
        assert!(p.insert(&poly, CageOperator::Given, 1).unwrap().is_none());
    }

    #[test]
    fn insert_add_cage_prunes_cells() {
        let p = Puzzle::from_parts(Grid::new(4).unwrap(), vec![]);
        let poly = domino(1, 1, 1, 2);
        let fp = p.insert(&poly, CageOperator::Add, 3).unwrap().unwrap();
        assert_eq!(fp.get(Cell(1, 1)).unwrap(), Fill::from(&[1, 2]));
        assert_eq!(fp.get(Cell(1, 2)).unwrap(), Fill::from(&[1, 2]));
    }

    #[test]
    fn fixpoint_no_cages_full_grid_unchanged() {
        // With no cages and a full grid, AllDifferent has nothing to prune.
        let p = Puzzle::from_parts(Grid::new(2).unwrap(), vec![]);
        let fp = p.fixpoint().unwrap();
        assert_eq!(fp.get(Cell(1, 1)).unwrap(), Fill::all(2));
        assert_eq!(fp.get(Cell(1, 2)).unwrap(), Fill::all(2));
    }

    #[test]
    fn fixpoint_given_cage_pins_cell() {
        // A given cage for value 3 must narrow cell(1,1) to {3}.
        let cage = Cage::given(Cell(1, 1), 3).unwrap();
        let p = Puzzle::from_parts(Grid::new(4).unwrap(), vec![cage]);
        let fp = p.fixpoint().unwrap();
        assert_eq!(fp.get(Cell(1, 1)).unwrap(), Fill::from(&[3]));
    }

    #[test]
    fn fixpoint_given_cage_propagates_through_all_different() {
        // Given cage pins cell(1,1)={2}; AllDifferent for row 1 must then remove
        // 2 from every other cell in that row.
        let cage = Cage::given(Cell(1, 1), 2).unwrap();
        let p = Puzzle::from_parts(Grid::new(3).unwrap(), vec![cage]);
        let fp = p.fixpoint().unwrap();
        assert_eq!(fp.get(Cell(1, 1)).unwrap(), Fill::from(&[2]));
        assert!(!fp.get(Cell(1, 2)).unwrap().contains(2));
        assert!(!fp.get(Cell(1, 3)).unwrap().contains(2));
        // Column 1 also loses 2 from all other cells.
        assert!(!fp.get(Cell(2, 1)).unwrap().contains(2));
        assert!(!fp.get(Cell(3, 1)).unwrap().contains(2));
    }

    #[test]
    fn fixpoint_add_cage_prunes_both_cells() {
        // Add 3 in a 4×4: only pairs (1,2),(2,1) satisfy it, so both cells narrow to {1,2}.
        let cage = Cage::commutative(4, domino(1, 1, 1, 2), Add, 3).unwrap();
        let p = Puzzle::from_parts(Grid::new(4).unwrap(), vec![cage]);
        let fp = p.fixpoint().unwrap();
        assert_eq!(fp.get(Cell(1, 1)).unwrap(), Fill::from(&[1, 2]));
        assert_eq!(fp.get(Cell(1, 2)).unwrap(), Fill::from(&[1, 2]));
    }

    #[test]
    fn fixpoint_cage_and_all_different_chain() {
        // 2×2 grid: subtract cage on column 1 with target 1 allows (1,2),(2,1).
        // Both cells can be 1 or 2. AllDifferent on each row then pins the partner cells.
        let cage = Cage::non_commutative(2, domino(1, 1, 2, 1), Subtract, 1).unwrap();
        let p = Puzzle::from_parts(Grid::new(2).unwrap(), vec![cage]);
        // Should be feasible and not panic.
        assert!(p.fixpoint().is_some());
    }

    #[test]
    fn fixpoint_infeasible_returns_none() {
        // Two given cages both claiming value 1 in the same row: infeasible.
        let c1 = Cage::given(Cell(1, 1), 1).unwrap();
        let c2 = Cage::given(Cell(1, 2), 1).unwrap();
        let p = Puzzle::from_parts(Grid::new(2).unwrap(), vec![c1, c2]);
        assert!(p.fixpoint().is_none());
    }
}
