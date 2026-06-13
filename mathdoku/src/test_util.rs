//! Helpers shared by the crate's unit tests.

use crate::polyomino::{Cell, Polyomino};
use crate::puzzle::{CageOperator, Puzzle};

/// A two-cell polyomino from 1-indexed `(row, column)` coordinates.
pub fn domino(r0: usize, c0: usize, r1: usize, c1: usize) -> Polyomino {
    Polyomino::from([Cell(r0, c0), Cell(r1, c1)]).unwrap()
}

/// A 2×2 puzzle with a single-cell `Given` cage pinning `(1,1)=1`.
pub fn pinned_2x2() -> Puzzle {
    Puzzle::new(2)
        .unwrap()
        .insert(
            &Polyomino::from([Cell(1, 1)]).unwrap(),
            CageOperator::Given,
            1,
        )
        .unwrap()
        .unwrap()
}

/// A three-cell polyomino from 1-indexed `(row, column)` coordinates.
pub fn triomino(r0: usize, c0: usize, r1: usize, c1: usize, r2: usize, c2: usize) -> Polyomino {
    Polyomino::from([Cell(r0, c0), Cell(r1, c1), Cell(r2, c2)]).unwrap()
}

/// Behavioural contract every [`Memo`](crate::memo::Memo) implementation must
/// satisfy. `Table` and `Mdd` unit tests both delegate here, passing a
/// constructor closure `(n, k, operator, target)` for the implementation
/// under test.
pub mod memo_contract {
    use crate::Error::{self, EmptyFills, InvalidCellCageIndex};
    use crate::fill::Fill;
    use crate::memo::Memo;
    use crate::operator::CommutativeOperator::{self, Add, Multiply};
    use crate::{N, Target};
    use std::fmt::Debug;

    /// Constructor for the implementation under test: `(n, k, operator, target)`.
    pub trait Make<M: Memo>: Fn(N, N, CommutativeOperator, Target) -> Result<M, Error> {}
    impl<M: Memo, F: Fn(N, N, CommutativeOperator, Target) -> Result<M, Error>> Make<M> for F {}

    pub fn add_fills_are_union_of_column_values<M: Memo>(make: impl Make<M>) {
        // 3+3=6, 2+4=6, 4+2=6 — position 0 is {2,3,4}, position 1 is {2,3,4}
        let m = make(4, 2, Add, 6).unwrap();
        assert_eq!(m.get(0).unwrap(), Fill::from(&[2, 3, 4]));
        assert_eq!(m.get(1).unwrap(), Fill::from(&[2, 3, 4]));
    }

    pub fn multiply_fills_contain_expected_values<M: Memo>(make: impl Make<M>) {
        // 2*3=6, 3*2=6, 1*6=6, 6*1=6 within n=6
        let m = make(6, 2, Multiply, 6).unwrap();
        assert_eq!(m.get(0).unwrap(), Fill::from(&[1, 2, 3, 6]));
        assert_eq!(m.get(1).unwrap(), Fill::from(&[1, 2, 3, 6]));
    }

    pub fn commutative_no_solutions_returns_empty_fills_error<M: Memo>(make: impl Make<M>) {
        // no 2-tuple in 1..=4 sums to 9
        assert!(matches!(make(4, 2, Add, 9), Err(EmptyFills)));
    }

    pub fn fill_out_of_bounds_returns_index_error<M: Memo>(make: impl Make<M>) {
        let m = make(4, 2, Add, 5).unwrap();
        assert!(matches!(m.get(2), Err(InvalidCellCageIndex(2))));
    }

    pub fn narrow_with_full_support_is_identity<M: Memo + PartialEq + Debug>(make: impl Make<M>) {
        // support that includes every value leaves all tuples intact
        let m = make(4, 2, Add, 5).unwrap();
        assert_eq!(m.narrow(&[Fill::all(4), Fill::all(4)]).unwrap(), m);
    }

    pub fn narrow_filters_tuples_and_updates_fills<M: Memo>(make: impl Make<M>) {
        // add to 5 in n=4: (1,4),(2,3),(3,2),(4,1)
        // restrict position 0 to {1,2} → surviving: (1,4),(2,3)
        let m = make(4, 2, Add, 5).unwrap();
        let narrowed = m
            .narrow(&[Fill::from(&[1, 2]), Fill::from(&[1, 2, 3, 4])])
            .unwrap();
        assert_eq!(narrowed.get(0).unwrap(), Fill::from(&[1, 2]));
        assert_eq!(narrowed.get(1).unwrap(), Fill::from(&[3, 4]));
    }

    pub fn narrow_eliminating_all_tuples_returns_empty_fills_error<M: Memo>(make: impl Make<M>) {
        let m = make(4, 2, Add, 5).unwrap();
        // restrict both positions to {1} — no tuple (1,1) sums to 5
        assert!(matches!(
            m.narrow(&[Fill::from(&[1]), Fill::from(&[1])]),
            Err(EmptyFills)
        ));
    }
}
