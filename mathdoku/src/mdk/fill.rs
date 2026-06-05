//! Candidate value sets.
use crate::mdk::N;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

/// The set of candidate values for a cell.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Fill(BTreeSet<N>);

impl Fill {
    /// Creates a full candidate set `{1..=n}`.
    pub(crate) fn new(n: usize) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        Self((1..=n as N).collect())
    }

    /// Creates a candidate set from an explicit slice of values.
    pub(crate) fn from(ns: &[N]) -> Self {
        Self(ns.iter().copied().collect())
    }

    /// Returns `true` if `value` is in this candidate set.
    pub(crate) fn contains(&self, value: N) -> bool {
        self.0.contains(&value)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for Fill {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}}}", self.0.iter().join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn new_contains_one_through_n() {
        let f = Fill::new(4);
        assert!(f.contains(1));
        assert!(f.contains(4));
        assert!(!f.contains(0));
        assert!(!f.contains(5));
    }

    #[test]
    fn from_empty_slice_is_empty() {
        assert!(Fill::from(&[]).is_empty());
    }

    #[test]
    fn from_deduplicates_values() {
        assert_eq!(Fill::from(&[2, 2, 3]), Fill::from(&[2, 3]));
    }

    #[test]
    fn contains_absent_value_is_false() {
        assert!(!Fill::from(&[1, 3]).contains(2));
    }

    #[test]
    fn is_empty_false_for_non_empty() {
        assert!(!Fill::new(3).is_empty());
    }

    #[test]
    fn default_is_empty() {
        assert!(Fill::default().is_empty());
    }

    #[test]
    fn display_empty() {
        assert_eq!(Fill::from(&[]).to_string(), "{}");
    }

    #[test]
    fn display_singleton() {
        assert_eq!(Fill::from(&[3]).to_string(), "{3}");
    }

    #[test]
    fn display_sorted() {
        assert_eq!(Fill::from(&[3, 1, 2]).to_string(), "{1, 2, 3}");
    }

    #[test]
    fn round_trips_through_json() {
        let f = Fill::from(&[1, 3]);
        assert_eq!(from_str::<Fill>(&to_string(&f).unwrap()).unwrap(), f);
    }

    #[test]
    fn serialize_is_sorted_array() {
        assert_eq!(to_string(&Fill::from(&[3, 1])).unwrap(), r"[1,3]");
    }
}
