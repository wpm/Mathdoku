//! Candidate value sets.
use crate::mdk::N;
use itertools::Itertools;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

/// The set of candidate values for a cell.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Debug, Default)]
pub struct Fill(usize, BTreeSet<N>);

impl Fill {
    /// Creates a full candidate set `{1..=n}`.
    pub(crate) fn new(n: usize) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        Self(n, (1..=n as N).collect())
    }

    /// Creates a candidate set from an explicit slice of values.
    pub(crate) fn from(n: usize, ns: &[N]) -> Self {
        Self(n, ns.iter().copied().collect())
    }

    /// Returns `true` if `value` is in this candidate set.
    pub(crate) fn contains(&self, value: N) -> bool {
        self.1.contains(&value)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.1.is_empty()
    }

    pub(crate) fn compliment(&self) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        Self(
            self.0,
            (1..=self.0 as N)
                .filter(|&v| !self.1.contains(&v))
                .collect(),
        )
    }
}

use serde::ser::SerializeStruct;

impl Serialize for Fill {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Fill", 2)?;
        st.serialize_field("n", &self.0)?;
        st.serialize_field("values", &self.1.iter().collect::<Vec<_>>())?;
        st.end()
    }
}

impl<'de> Deserialize<'de> for Fill {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            n: usize,
            values: Vec<N>,
        }
        let wire = Wire::deserialize(d)?;
        if wire.values.contains(&0) {
            return Err(DeError::custom("fill value must be >= 1"));
        }
        Ok(Self::from(wire.n, &wire.values))
    }
}

impl Display for Fill {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{}}}", self.1.iter().join(", "))
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
        assert!(Fill::from(0, &[]).is_empty());
    }

    #[test]
    fn from_deduplicates_values() {
        assert_eq!(Fill::from(4, &[2, 2, 3]), Fill::from(4, &[2, 3]));
    }

    #[test]
    fn contains_absent_value_is_false() {
        assert!(!Fill::from(4, &[1, 3]).contains(2));
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
        assert_eq!(Fill::from(0, &[]).to_string(), "{}");
    }

    #[test]
    fn display_singleton() {
        assert_eq!(Fill::from(4, &[3]).to_string(), "{3}");
    }

    #[test]
    fn display_sorted() {
        assert_eq!(Fill::from(4, &[3, 1, 2]).to_string(), "{1, 2, 3}");
    }

    #[test]
    fn round_trips_through_json() {
        let f = Fill::from(4, &[1, 3]);
        assert_eq!(from_str::<Fill>(&to_string(&f).unwrap()).unwrap(), f);
    }

    #[test]
    fn deserialize_zero_returns_err() {
        assert!(from_str::<Fill>(r#"{"n":4,"values":[0,1]}"#).is_err());
    }

    #[test]
    fn serialize_includes_n_and_sorted_values() {
        let f = Fill::from(4, &[3, 1]);
        let json = to_string(&f).unwrap();
        assert_eq!(json, r#"{"n":4,"values":[1,3]}"#);
    }
}
