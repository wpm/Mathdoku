//! Candidate value sets.
use crate::mdk::N;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::ops::{BitAnd, BitOr};

/// The set of candidate values for a cell, stored as a u16 bitmap.
///
/// Bit `v` (1 ≤ v ≤ 9) is set iff value `v` is a candidate. Bit 0 is unused.
/// The representation is identical to [`crate::Values`]; it exists separately
/// because this module is a clean-room reimplementation and shares no public API.
#[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Debug, Default, Hash)]
pub struct Fill(u16);

impl Fill {
    /// Creates a full candidate set `{1..=n}`.
    pub(crate) const fn new(n: usize) -> Self {
        // Set bits 1..=n: ((1 << (n+1)) - 1) & !1
        Self(((1u16 << (n + 1)).wrapping_sub(1)) & !1)
    }

    /// Creates a candidate set from an explicit slice of values.
    pub(crate) fn from(ns: &[N]) -> Self {
        Self(ns.iter().fold(0u16, |acc, &v| acc | (1u16 << v)))
    }

    /// Returns `true` if `value` is in this candidate set.
    pub(crate) const fn contains(self, value: N) -> bool {
        self.0 & (1u16 << value) != 0
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns the values in ascending order.
    pub(crate) fn values(self) -> impl Iterator<Item = N> {
        (1u32..=9).filter(move |&v| self.0 & (1u16 << v) != 0)
    }
}

impl BitOr for Fill {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for Fill {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl crate::mdk::csp::Domain for Fill {
    fn is_empty(&self) -> bool {
        Self::is_empty(*self)
    }
}

impl Display for Fill {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        let mut first = true;
        for v in self.values() {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{v}")?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl Serialize for Fill {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(self.values())
    }
}

impl<'de> Deserialize<'de> for Fill {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let ns = Vec::<N>::deserialize(d)?;
        for &v in &ns {
            if v == 0 || v > 9 {
                return Err(DeError::custom(format!("value {v} is outside 1..=9")));
            }
        }
        Ok(Self::from(&ns))
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
