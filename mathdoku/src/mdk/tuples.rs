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
                    if self.op.apply(new_tuple.clone()) + remaining <= self.target {
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
    use crate::mdk::operation::Commutative::Add;
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
}
