use crate::mdk::Target;
use crate::mdk::fill::Memo;
use crate::mdk::grid::Polyomino;
use crate::mdk::mdd::Mdd;
use crate::mdk::trie::Trie;
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter};

/// A polyomino paired with an arithmetic operation and its memoized candidate fills.
pub struct Cage {
    pub polyomino: Polyomino,
    operation: Operation,
    /// `None` for `Given` cages, which have no arithmetic constraint to memoize.
    pub memo: Option<Box<dyn Memo>>,
}

impl Cage {
    /// Creates a new cage for `polyomino` with `operation` on a grid of size `n`.
    #[must_use]
    pub fn new(n: usize, polyomino: Polyomino, operation: Operation) -> Self {
        let memo: Option<Box<dyn Memo>> = match operation.0 {
            Operator::Add | Operator::Multiply => {
                Some(Box::new(Mdd::new(n, &polyomino, &operation)))
            }
            Operator::Subtract | Operator::Divide => {
                Some(Box::new(Trie::new(n, &polyomino, &operation)))
            }
            Operator::Given => None,
        };
        Self {
            polyomino,
            operation,
            memo,
        }
    }
}

impl PartialEq for Cage {
    fn eq(&self, other: &Self) -> bool {
        self.polyomino == other.polyomino && self.operation == other.operation
    }
}

impl Eq for Cage {}

impl PartialOrd for Cage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Cage {
    fn cmp(&self, other: &Self) -> Ordering {
        self.polyomino
            .cmp(&other.polyomino)
            .then_with(|| self.operation.cmp(&other.operation))
    }
}

/// The arithmetic operator applied to a cage's cell values.
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Operator {
    Add,
    Multiply,
    Subtract,
    Divide,
    /// Cell value is given directly; no arithmetic constraint.
    Given,
}

impl Display for Operator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Add => "+",
            Self::Subtract => "−",
            Self::Multiply => "×",
            Self::Divide => "÷",
            Self::Given => "",
        };
        write!(f, "{s}")
    }
}

/// An operator paired with a target value.
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Operation(pub Operator, pub Target);

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0, self.1)
    }
}
