use crate::mdk::{N, Target};
use std::collections::VecDeque;

struct Tuples {
    n: usize,
    k: usize,
    f: Box<dyn Fn(&Vec<N>) -> Target>,
    target: Target,
    queue: VecDeque<Vec<N>>,
}

impl Tuples {
    fn new(n: usize, k: usize, f: impl Fn(&Vec<N>) -> Target + 'static, target: Target) -> Self {
        Tuples {
            n,
            k,
            f: Box::new(f),
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
                if (self.f)(&tuple) == self.target {
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
                    if (self.f)(&new_tuple) + remaining <= self.target {
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
    use crate::mdk::operation::CageOperation::Monotonic;
    use crate::mdk::operation::Commutative::Add;
    use crate::mdk::table::Tuples;

    #[test]
    fn sum_to_6() {
        let Monotonic(op, target) = Monotonic(Add, 6) else { panic!() };
        let tuples = Tuples::new(7, 3, move |tuple| op.apply(tuple.clone()), target);
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
