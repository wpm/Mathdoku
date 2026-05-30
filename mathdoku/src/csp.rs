//! Generic constraint satisfaction problem (CSP) abstractions.
//!
//! This module defines `Domain`, `Constraint`, and the `ac3` worklist algorithm.
//! Concrete solvers implement these traits for a specific problem.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// A set of values of type `Z` for a variable.
#[allow(dead_code)]
pub trait Domain<Z>: Clone {
    /// Returns a copy of this domain with `z` removed.
    #[must_use]
    fn remove(&self, z: Z) -> Self;
    /// Returns `true` if this domain has no remaining values.
    fn is_empty(&self) -> bool;
}

/// A variable that is assigned a [`Domain`] and is subject to [`Constraint`]s.
pub trait Variable: Clone + Eq + Hash {}

/// A mapping of variables to their [`Domain`]s.
#[allow(dead_code)]
pub trait State<X, D, Z>: Clone + Extend<(X, D)>
where
    D: Domain<Z>,
    X: Variable,
{
    /// The [`Domain`] of the variable.
    fn domain(&self, x: &X) -> D;
}

/// A constraint on the [`Domain`]s of a set of variables of type `X`.
#[allow(dead_code)]
pub trait Constraint<S, X, D, Z>: Clone
where
    D: Domain<Z>,
    X: Variable,
    S: State<X, D, Z>,
{
    /// Is `x` in this constraint's scope?
    fn in_scope(&self, x: &X) -> bool;

    /// Enforces this constraint against the domain state `s` and returns the updated domains.
    ///
    /// For each variable in scope, removes values from its domain that are not supported
    /// by any consistent assignment of the other in-scope variables. Only entries whose
    /// domains changed are required in the returned map.
    fn propagate(&self, s: S) -> impl Iterator<Item = (X, D)>;
}

/// Enforces arc consistency via the AC-3 worklist algorithm.
///
/// Processes constraints one at a time from a queue. When propagating a constraint
/// narrows any variable's domain, all constraints that include that variable are
/// re-queued. Terminates when no constraint can narrow any domain further.
#[allow(dead_code)]
fn ac3<S, X, D, C, Z>(mut s: S, constraints: &[C]) -> S
where
    S: State<X, D, Z>,
    X: Variable,
    D: Domain<Z>,
    C: Constraint<S, X, D, Z>,
{
    let mut q: VecDeque<_> = constraints.iter().collect();
    while let Some(constraint) = q.pop_front() {
        let delta = constraint
            .propagate(s.clone())
            .fold(HashMap::new(), |mut r, (v, d)| {
                let _ = r.insert(v, d);
                r
            });
        let v: Vec<_> = constraints
            .iter()
            .filter(|constraint| delta.keys().any(|x| constraint.in_scope(x)))
            .collect();
        q.extend(v);
        s.extend(delta);
    }
    s
}

#[cfg(test)]
mod tests {
    use crate::csp::{Constraint, Domain, State, ac3};
    use itertools::Itertools;
    use std::collections::{HashMap, HashSet};

    /// A set of natural numbers backed by a `HashSet<u8>`.
    #[derive(Debug, Clone, PartialEq)]
    struct N {
        s: HashSet<u8>,
    }

    impl N {
        fn new(ns: &[u8]) -> Self {
            Self {
                s: ns.iter().copied().collect(),
            }
        }
        fn intersection(&self, other: &Self) -> Self {
            Self {
                s: self.s.intersection(&other.s).copied().collect(),
            }
        }
    }

    impl Domain<u8> for N {
        fn remove(&self, z: u8) -> Self {
            let mut s = self.s.clone();
            let _ = s.remove(&z);
            Self { s }
        }

        fn is_empty(&self) -> bool {
            self.s.is_empty()
        }
    }

    #[derive(Clone, PartialEq, Debug)]
    struct Values(HashMap<String, N>);

    impl Values {
        fn from(values: &[(&str, N)]) -> Self {
            Self(
                values
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect(),
            )
        }
    }

    impl State<String, N, u8> for Values {
        fn domain(&self, x: &String) -> N {
            self.0[x].clone()
        }
    }

    impl Extend<(String, N)> for Values {
        fn extend<T: IntoIterator<Item = (String, N)>>(&mut self, iter: T) {
            self.0.extend(iter);
        }
    }

    /// A constraint requiring that two named variables share the same value.
    #[derive(Clone)]
    struct Equals {
        a: String,
        b: String,
    }

    impl Equals {
        fn new(a: &str, b: &str) -> Self {
            Self {
                a: a.into(),
                b: b.into(),
            }
        }
    }

    impl crate::csp::Variable for String {}

    impl Constraint<Values, String, N, u8> for Equals {
        fn in_scope(&self, x: &String) -> bool {
            *x == self.a || *x == self.b
        }

        fn propagate(&self, s: Values) -> impl Iterator<Item = (String, N)> {
            use std::sync::Arc;
            let map = Arc::new(s.0);
            let keys: Vec<String> = map.keys().filter(|x| self.in_scope(x)).cloned().collect();
            let map2 = Arc::clone(&map);
            keys.clone()
                .into_iter()
                .cartesian_product(keys)
                .filter(move |(a, b)| a != b && map[a] != map[b])
                .flat_map(move |(a, b)| {
                    let i = map2[&a].intersection(&map2[&b]);
                    [(a, i.clone()), (b, i)]
                })
        }
    }

    /// A constraint requiring that named variables sum to a target value.
    #[derive(Clone)]
    struct Sum {
        vars: Vec<String>,
        target: u8,
    }

    impl Sum {
        fn new(vars: &[&str], target: u8) -> Self {
            Self {
                vars: vars.iter().map(|&v| v.to_string()).collect(),
                target,
            }
        }
    }

    fn sum_extend(
        pos: usize,
        current: &mut Vec<u8>,
        domains: &[Vec<u8>],
        target: u8,
        survivors: &mut Vec<HashSet<u8>>,
    ) {
        if pos == domains.len() {
            if current.iter().map(|&v| u32::from(v)).sum::<u32>() == u32::from(target) {
                for (i, &v) in current.iter().enumerate() {
                    let _ = survivors[i].insert(v);
                }
            }
            return;
        }
        for &v in &domains[pos] {
            current.push(v);
            sum_extend(pos + 1, current, domains, target, survivors);
            let _ = current.pop();
        }
    }

    impl Constraint<Values, String, N, u8> for Sum {
        fn in_scope(&self, x: &String) -> bool {
            self.vars.contains(x)
        }

        fn propagate(&self, s: Values) -> impl Iterator<Item = (String, N)> {
            use std::sync::Arc;
            let map = Arc::new(s.0);
            let vars = self.vars.clone();
            let target = self.target;
            let domains: Vec<Vec<u8>> = vars
                .iter()
                .map(|v| map[v].s.iter().copied().collect())
                .collect();
            let mut survivors: Vec<HashSet<u8>> = vars.iter().map(|_| HashSet::new()).collect();
            sum_extend(0, &mut vec![], &domains, target, &mut survivors);
            vars.into_iter().zip(survivors).filter_map(move |(v, sv)| {
                if sv == map[&v].s {
                    None
                } else {
                    Some((v, N { s: sv }))
                }
            })
        }
    }

    #[derive(Clone)]
    enum NumericConstraint {
        Equals(Equals),
        Sum(Sum),
    }

    impl Constraint<Values, String, N, u8> for NumericConstraint {
        fn in_scope(&self, x: &String) -> bool {
            match self {
                Self::Equals(c) => c.in_scope(x),
                Self::Sum(c) => c.in_scope(x),
            }
        }

        fn propagate(&self, s: Values) -> impl Iterator<Item = (String, N)> {
            match self {
                Self::Equals(c) => {
                    Box::new(c.propagate(s)) as Box<dyn Iterator<Item = (String, N)>>
                }
                Self::Sum(c) => Box::new(c.propagate(s)),
            }
        }
    }

    fn run<C: Constraint<Values, String, N, u8> + Clone>(
        domains: &[(&str, &[u8])],
        constraints: &[C],
    ) -> Values {
        let initial = Values::from(
            &domains
                .iter()
                .map(|(k, vs)| (*k, N::new(vs)))
                .collect::<Vec<_>>(),
        );
        ac3(initial, constraints)
    }

    fn expect(domains: &[(&str, &[u8])]) -> Values {
        Values::from(
            &domains
                .iter()
                .map(|(k, vs)| (*k, N::new(vs)))
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    // x ∈ {1,2,3}, y ∈ {2,3,4}, x=y  →  both {2,3}
    fn equal_overlapping_domains_intersects_both() {
        assert_eq!(
            run(
                &[("x", &[1, 2, 3]), ("y", &[2, 3, 4])],
                &[Equals::new("x", "y")]
            ),
            expect(&[("x", &[2, 3]), ("y", &[2, 3])])
        );
    }

    #[test]
    // x ∈ {5}, y ∈ {1,2,3,4,5}, x=y  →  both {5}
    fn equal_singleton_pins_other_variable() {
        assert_eq!(
            run(
                &[("x", &[5]), ("y", &[1, 2, 3, 4, 5])],
                &[Equals::new("x", "y")]
            ),
            expect(&[("x", &[5]), ("y", &[5])])
        );
    }

    #[test]
    // x ∈ {1,2}, y ∈ {3,4}, x=y  →  both empty (infeasible)
    fn equal_disjoint_domains_empties_both() {
        assert_eq!(
            run(&[("x", &[1, 2]), ("y", &[3, 4])], &[Equals::new("x", "y")]),
            expect(&[("x", &[]), ("y", &[])])
        );
    }

    #[test]
    // x,y ∈ {1,2,3}, x+y=5  →  only (2,3),(3,2) work, so x,y ∈ {2,3}
    fn sum_two_vars_prunes_unsupported_values() {
        assert_eq!(
            run(
                &[("x", &[1, 2, 3]), ("y", &[1, 2, 3])],
                &[Sum::new(&["x", "y"], 5)]
            ),
            expect(&[("x", &[2, 3]), ("y", &[2, 3])])
        );
    }

    #[test]
    // x,y,z ∈ {1,2,3}, x+y+z=6  →  permutations of (1,2,3) use every value
    fn sum_three_vars_all_values_survive() {
        assert_eq!(
            run(
                &[("x", &[1, 2, 3]), ("y", &[1, 2, 3]), ("z", &[1, 2, 3])],
                &[Sum::new(&["x", "y", "z"], 6)]
            ),
            expect(&[("x", &[1, 2, 3]), ("y", &[1, 2, 3]), ("z", &[1, 2, 3])])
        );
    }

    #[test]
    // x,y ∈ {1,2}, x+y=10 — impossible
    fn sum_infeasible_target_empties_domains() {
        assert_eq!(
            run(
                &[("x", &[1, 2]), ("y", &[1, 2])],
                &[Sum::new(&["x", "y"], 10)]
            ),
            expect(&[("x", &[]), ("y", &[])])
        );
    }

    #[test]
    // x,y,z ∈ {1,2,3}; x+y=5 pins x,y ∈ {2,3}; then x=z chains to pin z ∈ {2,3}
    fn propagation_chains_across_constraints() {
        assert_eq!(
            run(
                &[("x", &[1, 2, 3]), ("y", &[1, 2, 3]), ("z", &[1, 2, 3])],
                &[
                    NumericConstraint::Sum(Sum::new(&["x", "y"], 5)),
                    NumericConstraint::Equals(Equals::new("x", "z")),
                ]
            ),
            expect(&[("x", &[2, 3]), ("y", &[2, 3]), ("z", &[2, 3])])
        );
    }
}
