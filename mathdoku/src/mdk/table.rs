use crate::mdk::Error::EmptyFills;
use crate::mdk::Error::IndexOutOfBounds;
use crate::mdk::fill::Fill;
use crate::mdk::memo::{Memo, Narrow};
use crate::mdk::operation::{Commutative, NonCommutative};
use crate::mdk::tuples::Tuples;
use crate::mdk::{Error, N, Target};

pub struct Table {
    n: usize,
    tuples: Vec<Vec<N>>,
    fills: Vec<Fill>,
}

impl Table {
    fn new(n: usize, tuples: Vec<Vec<N>>) -> Result<Self, Error> {
        if tuples.is_empty() {
            return Err(EmptyFills);
        }
        let k = tuples[0].len();
        let fills: Vec<Fill> = (0..k)
            .map(|i| Fill::from(&tuples.iter().map(|t| t[i]).collect::<Vec<N>>()))
            .collect();
        if fills.iter().any(Fill::is_empty) {
            return Err(EmptyFills);
        }
        Ok(Self { n, tuples, fills })
    }
}

impl Memo for Table {
    fn commutative(
        n: usize,
        k: usize,
        operator: Commutative,
        target: Target,
    ) -> Result<Self, Error> {
        Self::new(n, Tuples::commutative(n, k, operator, target).collect())
    }

    fn non_commutative(n: usize, operator: NonCommutative, target: Target) -> Result<Self, Error> {
        Self::new(n, Tuples::non_commutative(n, operator, target).collect())
    }

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
        Self::new(self.n, tuples)
    }
}
