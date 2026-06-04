//! Iterator over value tuples satisfying a cage arithmetic constraint.
use crate::mdk::operation::{Commutative, NonCommutative};
use crate::mdk::{N, Target};
use std::collections::VecDeque;

/// Iterator over all `k`-tuples of values in `1..=n` that satisfy an arithmetic constraint.
///
/// Tuples are yielded in lexicographic order via BFS. Commutative operations
/// use the ring identity to prune the search; non-commutative operations
/// enumerate all pairs without pruning.
struct Tuples {
    n: usize,
    k: usize,
    constraint: ArithmeticOperation,
    queue: VecDeque<Vec<N>>,
}

impl Tuples {
    /// Creates a `Tuples` iterator for a commutative (monotonic) operation.
    fn commutative(n: usize, k: usize, operator: Commutative, target: Target) -> Self {
        Tuples {
            n,
            k,
            constraint: ArithmeticOperation::Commutative(operator, target),
            queue: VecDeque::from([vec![]]),
        }
    }

    /// Creates a `Tuples` iterator for a non-commutative operation over pairs (`k = 2`).
    fn non_commutative(n: usize, op: NonCommutative, target: Target) -> Self {
        Tuples {
            n,
            k: 2,
            constraint: ArithmeticOperation::NonCommutative(op, target),
            queue: VecDeque::from([vec![]]),
        }
    }

    /// Advances one step for a commutative operation.
    ///
    /// Prunes partial tuples whose result plus the minimum possible completion
    /// already exceeds the target, using the dual operation's identity element
    /// as the minimum-per-remaining-slot bound.
    fn monotonic(&mut self, operator: Commutative, target: Target) -> Step {
        let Some(tuple) = self.queue.pop_front() else {
            return Step::Exhausted;
        };
        match tuple.len() == self.k {
            true => {
                if operator.apply(&tuple) == target {
                    Step::Yield(tuple)
                } else {
                    Step::Continue
                }
            }
            false => {
                for i in 1..=self.n {
                    let mut new_tuple = tuple.clone();
                    new_tuple.push(i as N);
                    let s = operator.apply(&new_tuple);
                    let remaining = (self.k - new_tuple.len()) as N;
                    let residual = operator.dual().identity() * remaining;
                    if s + residual <= target {
                        self.queue.push_back(new_tuple);
                    }
                }
                Step::Continue
            }
        }
    }

    /// Advances one step for a non-commutative operation.
    ///
    /// No pruning is possible since the operation is not monotonic.
    fn non_monotonic(&mut self, operator: NonCommutative, target: Target) -> Step {
        let Some(tuple) = self.queue.pop_front() else {
            return Step::Exhausted;
        };
        match tuple.len() == self.k {
            true => {
                if operator.apply(tuple[0], tuple[1]) == target {
                    Step::Yield(tuple)
                } else {
                    Step::Continue
                }
            }
            false => {
                for i in 1..=self.n {
                    let mut new_tuple = tuple.clone();
                    new_tuple.push(i as N);
                    self.queue.push_back(new_tuple);
                }
                Step::Continue
            }
        }
    }
}

/// Result of one BFS step.
enum Step {
    /// A complete tuple that satisfies the target — yield it.
    Yield(Vec<N>),
    /// Partial tuple extended or complete tuple rejected — keep going.
    Continue,
    /// Queue is empty — iteration is finished.
    Exhausted,
}

impl Iterator for Tuples {
    type Item = Vec<N>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let step = match self.constraint {
                ArithmeticOperation::Commutative(operator, target) => {
                    self.monotonic(operator, target)
                }
                ArithmeticOperation::NonCommutative(operator, target) => {
                    self.non_monotonic(operator, target)
                }
            };
            match step {
                Step::Yield(tuple) => return Some(tuple),
                Step::Continue => continue,
                Step::Exhausted => return None,
            }
        }
    }
}

/// An arithmetic operation paired with a target value.
#[derive(Clone, Copy)]
pub enum ArithmeticOperation {
    /// A commutative (monotonic) operation: add or multiply.
    Commutative(Commutative, Target),
    /// A non-commutative (non-monotonic) operation: subtract or divide.
    NonCommutative(NonCommutative, Target),
}

#[cfg(test)]
mod tests {
    use crate::mdk::N;
    use crate::mdk::operation::Commutative::{Add, Multiply};
    use crate::mdk::operation::NonCommutative::{Divide, Subtract};
    use crate::mdk::tuples::Tuples;

    #[test]
    fn sum_to_6() {
        let tuples = Tuples::commutative(7, 3, Add, 6);
        let actual: Vec<Vec<N>> = tuples.collect();
        assert_eq!(
            actual,
            vec![
                vec![1, 1, 4],
                vec![1, 2, 3],
                vec![1, 3, 2],
                vec![1, 4, 1],
                vec![2, 1, 3],
                vec![2, 2, 2],
                vec![2, 3, 1],
                vec![3, 1, 2],
                vec![3, 2, 1],
                vec![4, 1, 1],
            ]
        );
    }

    #[test]
    fn multiply_to_24() {
        let tuples = Tuples::commutative(7, 3, Multiply, 24);
        let actual: Vec<Vec<N>> = tuples.collect();
        // n=7 excludes e.g. [1, 3, 8] and [1, 2, 12]
        assert_eq!(
            actual,
            vec![
                vec![1, 4, 6],
                vec![1, 6, 4],
                vec![2, 2, 6],
                vec![2, 3, 4],
                vec![2, 4, 3],
                vec![2, 6, 2],
                vec![3, 2, 4],
                vec![3, 4, 2],
                vec![4, 1, 6],
                vec![4, 2, 3],
                vec![4, 3, 2],
                vec![4, 6, 1],
                vec![6, 1, 4],
                vec![6, 2, 2],
                vec![6, 4, 1],
            ]
        );
    }

    #[test]
    fn subtract_to_2() {
        let tuples = Tuples::non_commutative(4, Subtract, 2);
        let actual: Vec<Vec<N>> = tuples.collect();
        assert_eq!(
            actual,
            vec![vec![1, 3], vec![2, 4], vec![3, 1], vec![4, 2],]
        );
    }

    #[test]
    fn divide_to_2() {
        let tuples = Tuples::non_commutative(6, Divide, 2);
        let actual: Vec<Vec<N>> = tuples.collect();
        // includes integer-division pairs e.g. [2, 5] since max(2,5)/min(2,5) = 5/2 = 2
        assert_eq!(
            actual,
            vec![
                vec![1, 2],
                vec![2, 1],
                vec![2, 4],
                vec![2, 5],
                vec![3, 6],
                vec![4, 2],
                vec![5, 2],
                vec![6, 3],
            ]
        );
    }
}
