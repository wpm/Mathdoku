use crate::mdk::operation::{Commutative, NonCommutative};
use crate::mdk::{N, Target};
use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub enum ArithmeticOperation {
    Commutative(Commutative, Target),
    NonCommutative(NonCommutative, Target),
}

struct Tuples {
    n: usize,
    k: usize,
    operation: ArithmeticOperation,
    queue: VecDeque<Vec<N>>,
}

impl Tuples {
    fn commutative(n: usize, k: usize, operator: Commutative, target: Target) -> Self {
        Tuples {
            n,
            k,
            operation: ArithmeticOperation::Commutative(operator, target),
            queue: VecDeque::from([vec![]]),
        }
    }

    fn non_commutative(n: usize, op: NonCommutative, target: Target) -> Self {
        Tuples {
            n,
            k: 2, // A non-commutative operation requires exactly 2 elements.
            operation: ArithmeticOperation::NonCommutative(op, target),
            queue: VecDeque::from([vec![]]),
        }
    }

    fn monotonic(&mut self, operator: Commutative, target: Target) -> Option<Option<Vec<N>>> {
        let tuple = self.queue.pop_front()?;
        Some(match tuple.len() == self.k {
            true => {
                if operator.apply(&tuple) == target {
                    Some(tuple)
                } else {
                    self.next()
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
                self.next()
            }
        })
    }
    fn non_monotonic(
        &mut self,
        operator: NonCommutative,
        target: Target,
    ) -> Option<Option<Vec<N>>> {
        let tuple = self.queue.pop_front()?;
        Some(match tuple.len() == self.k {
            true => {
                if operator.apply(tuple[0], tuple[1]) == target {
                    Some(tuple)
                } else {
                    self.next()
                }
            }
            false => {
                for i in 1..=self.n {
                    let mut new_tuple = tuple.clone();
                    new_tuple.push(i as N);
                    self.queue.push_back(new_tuple);
                }
                self.next()
            }
        })
    }
}

impl Iterator for Tuples {
    type Item = Vec<N>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.operation {
            ArithmeticOperation::Commutative(operator, target) => {
                self.monotonic(operator, target)?
            }
            ArithmeticOperation::NonCommutative(operator, target) => {
                self.non_monotonic(operator, target)?
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mdk::N;
    use crate::mdk::operation::Commutative::{Add, Multiply};
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
}
