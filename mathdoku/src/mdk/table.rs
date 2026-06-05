//! Explicit-tuple implementation of [`Lookup`] and [`Narrow`].
use crate::mdk::Error::IndexOutOfBounds;
use crate::mdk::fill::Fill;
use crate::mdk::memo::{Lookup, Narrow, fills_from_tuples};
use crate::mdk::operation::{CommutativeOperation, NonCommutativeOperation};
use crate::mdk::tuples::Tuples;
use crate::mdk::{Error, N, Target};

/// A cage constraint stored as an explicit list of valid value tuples.
///
/// Each tuple is a `k`-vector of values in `1..=n` satisfying the cage's
/// arithmetic constraint. Per-position candidate sets ([`Fill`]s) are derived
/// as the union of values appearing at each position across all tuples, and
/// are guaranteed non-empty — construction fails with [`EmptyFills`]
/// if no valid tuples exist.
pub(crate) struct Table {
    n: usize,
    tuples: Vec<Vec<N>>,
    fills: Vec<Fill>,
}

impl Table {
    /// Constructs a representation of all `k`-tuples of values in `1..=n`
    /// satisfying a commutative (add or multiply) constraint.
    ///
    /// # Errors
    /// Returns [`EmptyFills`] if no tuples satisfy the constraint.
    pub fn commutative(
        n: usize,
        k: usize,
        operator: CommutativeOperation,
        target: Target,
    ) -> Result<Self, Error> {
        Self::build(n, Tuples::commutative(n, k, operator, target).collect())
    }

    /// Constructs a representation of all pairs of values in `1..=n`
    /// satisfying a non-commutative (subtract or divide) constraint.
    ///
    /// # Errors
    /// Returns [`EmptyFills`] if no tuples satisfy the constraint.
    pub fn non_commutative(
        n: usize,
        operator: NonCommutativeOperation,
        target: Target,
    ) -> Result<Self, Error> {
        Self::build(n, Tuples::non_commutative(n, operator, target).collect())
    }

    /// Constructs a `Table` from a pre-computed list of tuples, deriving fills.
    ///
    /// # Errors
    /// Returns [`EmptyFills`] if `tuples` is empty or any position's
    /// fill would be empty.
    fn build(n: usize, tuples: Vec<Vec<N>>) -> Result<Self, Error> {
        let fills = fills_from_tuples(&tuples)?;
        Ok(Self { n, tuples, fills })
    }
}

impl Lookup for Table {
    fn fill(&self, index: usize) -> Result<Fill, Error> {
        self.fills
            .get(index)
            .cloned()
            .ok_or(IndexOutOfBounds(index))
    }
}

impl Narrow for Table {
    fn remove(&self, fills: Vec<Fill>) -> Result<Self, Error> {
        let tuples = self
            .tuples
            .iter()
            .filter(|tuple| tuple.iter().enumerate().all(|(i, &v)| fills[i].contains(v)))
            .cloned()
            .collect::<Vec<_>>();
        Self::build(self.n, tuples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdk::Error::EmptyFills;
    use crate::mdk::operation::CommutativeOperation::{Add, Multiply};
    use crate::mdk::operation::NonCommutativeOperation::{Divide, Subtract};

    #[test]
    fn add_fills_are_union_of_column_values() {
        // 3+3=6, 2+4=6, 4+2=6 — position 0 is {2,3,4}, position 1 is {2,3,4}
        let t = Table::commutative(4, 2, Add, 6).unwrap();
        assert_eq!(t.fill(0).unwrap(), Fill::from(&[2, 3, 4]));
        assert_eq!(t.fill(1).unwrap(), Fill::from(&[2, 3, 4]));
    }

    #[test]
    fn multiply_fills_contain_expected_values() {
        // 2*3=6, 3*2=6, 1*6=6, 6*1=6 within n=6
        let t = Table::commutative(6, 2, Multiply, 6).unwrap();
        assert_eq!(t.fill(0).unwrap(), Fill::from(&[1, 2, 3, 6]));
        assert_eq!(t.fill(1).unwrap(), Fill::from(&[1, 2, 3, 6]));
    }

    #[test]
    fn subtract_fills_contain_expected_values() {
        // pairs with |a-b|=1 in n=4: (1,2),(2,1),(2,3),(3,2),(3,4),(4,3)
        let t = Table::non_commutative(4, Subtract, 1).unwrap();
        assert_eq!(t.fill(0).unwrap(), Fill::from(&[1, 2, 3, 4]));
        assert_eq!(t.fill(1).unwrap(), Fill::from(&[1, 2, 3, 4]));
    }

    #[test]
    fn divide_fills_contain_expected_values() {
        // pairs with max/min=2 in n=4: (1,2),(2,1),(2,4),(4,2)
        let t = Table::non_commutative(4, Divide, 2).unwrap();
        assert_eq!(t.fill(0).unwrap(), Fill::from(&[1, 2, 4]));
        assert_eq!(t.fill(1).unwrap(), Fill::from(&[1, 2, 4]));
    }

    #[test]
    fn commutative_no_solutions_returns_empty_fills_error() {
        // no 2-tuple in 1..=4 sums to 9
        assert!(matches!(Table::commutative(4, 2, Add, 9), Err(EmptyFills)));
    }

    #[test]
    fn fill_out_of_bounds_returns_index_error() {
        let t = Table::commutative(4, 2, Add, 5).unwrap();
        assert!(matches!(t.fill(2), Err(IndexOutOfBounds(2))));
    }

    #[test]
    fn remove_filters_tuples_and_updates_fills() {
        // add to 5 in n=4: (1,4),(2,3),(3,2),(4,1)
        let t = Table::commutative(4, 2, Add, 5).unwrap();
        // restrict position 0 to {1,2}, position 1 to {1,2,3,4}
        let narrowed = t
            .remove(vec![Fill::from(&[1, 2]), Fill::from(&[1, 2, 3, 4])])
            .unwrap();
        assert_eq!(narrowed.fill(0).unwrap(), Fill::from(&[1, 2]));
        assert_eq!(narrowed.fill(1).unwrap(), Fill::from(&[3, 4]));
    }

    #[test]
    fn remove_eliminating_all_tuples_returns_empty_fills_error() {
        let t = Table::commutative(4, 2, Add, 5).unwrap();
        // restrict both positions to {1} — no tuple (1,1) sums to 5
        assert!(matches!(
            t.remove(vec![Fill::from(&[1]), Fill::from(&[1])]),
            Err(EmptyFills)
        ));
    }
}
