use crate::{M, Polyomino};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::{Display, Formatter};

/// An [`Operator`] paired with a numeric target value imposed on a cage's cells.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Operation {
    pub operator: Operator,
    pub target: M,
}

impl Operation {
    #[must_use]
    pub const fn new(operator: Operator, target: M) -> Self {
        Self { operator, target }
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.operator, self.target)
    }
}

/// The arithmetic operation a cage imposes on its cells.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Operator {
    /// Cells sum to the target.
    Add,
    /// Two cells differ by the target.
    Subtract,
    /// Cells multiply to the target.
    Multiply,
    /// Two cells have a ratio equal to the target.
    Divide,
    /// A single cell is fixed to the target value.
    Given,
}

impl Display for Operator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "×",
            Self::Divide => "÷",
            Self::Given => "",
        };
        write!(f, "{s}")
    }
}

#[must_use]
pub fn operators(polynomial: &Polyomino) -> Vec<Operator> {
    match polynomial.len() {
        1 => vec![Operator::Given],
        2 => vec![
            Operator::Add,
            Operator::Subtract,
            Operator::Multiply,
            Operator::Divide,
            Operator::Given,
        ],
        _ => vec![Operator::Add, Operator::Multiply],
    }
}
