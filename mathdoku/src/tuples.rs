use crate::arithmetic::{
    addition_multisets, division_multisets, multiplication_multisets, subtraction_multisets,
};
use crate::operation::{Operation, Operator};
use crate::{Cell, M, N, Polyomino, Tuple};
use itertools::Itertools;
use std::collections::HashMap;
use std::iter::{empty, once};

/// Returns all valid [`Tuple`]s for a polyomino with a given operation in an `n`×`n` grid.
///
/// Each [`Tuple`] assigns one value from `1..=n` to each cell of `polyomino`, in the
/// row-major order of [`Polyomino::cells`]. The algorithm:
///
/// 1. Generates all non-decreasing multisets of values satisfying the arithmetic operation (e.g.,
///    all `k`-tuples summing to `target`).
/// 2. Expands each multiset into all its permutations.
/// 3. Filters out permutations that repeat a value within any row or column shared by two or more
///    cells of the polyomino (the all-different constraint).
/// 4. Sorts and deduplicates (distinct permutations of the same multiset are all kept; only exact
///    duplicates from step 3 are removed).
#[allow(clippy::needless_pass_by_value)]
pub fn tuples(n: N, polyomino: &Polyomino, operation: Operation) -> impl Iterator<Item = Tuple> {
    let k = polyomino.len();
    let target = operation.target;
    let multisets: Box<dyn Iterator<Item = Tuple>> = match operation.operator {
        Operator::Add => Box::new(addition_multisets(n, k, target)),
        Operator::Subtract => Box::new(subtraction_multisets(n, target)),
        Operator::Multiply => Box::new(multiplication_multisets(n, k, target)),
        Operator::Divide => Box::new(division_multisets(n, target)),
        Operator::Given => {
            #[allow(clippy::cast_possible_truncation)] // guarded: target <= n <= 9
            if target >= 1 && target <= M::from(n) {
                Box::new(once(vec![target as N]))
            } else {
                Box::new(empty())
            }
        }
    };
    let filter = CollinearityFilter::new(polyomino);
    multisets
        .flat_map(move |t| t.into_iter().permutations(k))
        .filter(move |t| filter.filter(t))
        .sorted()
        .dedup()
}

/// Filters tuples that violate the all-different constraint within any row or
/// column of a cage's polyomino.
///
/// Precomputes the cell-index groups for each row and column once on
/// construction, then checks each candidate tuple against those groups.
struct CollinearityFilter {
    rows_and_columns: Vec<Vec<usize>>,
}

impl CollinearityFilter {
    /// Builds the filter for `polyomino`, grouping cell indices by shared row
    /// and column.
    fn new(polyomino: &Polyomino) -> Self {
        let cell_indexes: HashMap<Cell, usize> = polyomino
            .cells()
            .iter()
            .copied()
            .enumerate()
            .map(|(i, cell)| (cell, i))
            .collect();
        let to_indexes = |cells: Vec<Cell>| cells.iter().map(|cell| cell_indexes[cell]).collect();
        let rows = polyomino.rows().into_iter().map(&to_indexes);
        let columns = polyomino.columns().into_iter().map(&to_indexes);
        Self {
            rows_and_columns: rows.chain(columns).collect(),
        }
    }
    /// Returns `true` if `tuple` satisfies all-different within every row and
    /// column group of the polyomino.
    fn filter(&self, tuple: &Tuple) -> bool {
        self.rows_and_columns
            .iter()
            .all(|indexes| indexes.iter().map(|&i| tuple[i]).all_unique())
    }
}
