use crate::mdk::operation::Commutative;
use crate::mdk::{N, Target};
use std::collections::VecDeque;

struct Tuples {
    n: usize,
    k: usize,
    op: Commutative,
    target: Target,
    queue: VecDeque<Vec<N>>,
}

impl Tuples {
    fn new(n: usize, k: usize, op: Commutative, target: Target) -> Self {
        Tuples {
            n,
            k,
            op,
            target,
            queue: VecDeque::from([vec![]]),
        }
    }
}

impl Iterator for Tuples {
    type Item = Vec<N>;
    fn next(&mut self) -> Option<Self::Item> {
        let tuple = self.queue.pop_front()?;
        match tuple.len() == self.k {
            true => {
                if self.op.apply(tuple.clone()) == self.target {
                    Some(tuple)
                } else {
                    self.next()
                }
            }
            false => {
                for i in 1..=self.n {
                    let mut new_tuple = tuple.clone();
                    new_tuple.push(i as N);
                    let remaining = (self.k - new_tuple.len()) as N;
                    let s = self.op.apply(new_tuple.clone());
                    let residual = self.op.dual().identity() * remaining;
                    if s + residual <= self.target {
                        self.queue.push_back(new_tuple);
                    }
                }
                self.next()
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
        let tuples = Tuples::new(7, 3, Add, 6);
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
        let tuples = Tuples::new(7, 3, Multiply, 24);
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
