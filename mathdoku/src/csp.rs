#![allow(dead_code)]
//! Generic constraint satisfaction problem (CSP) abstractions.
//!
//! This module defines the core traits of a CSP — [`Variable`] and [`Constraint`] —
//! and the [`generalized_arc_consistency`] algorithm that ties them together. The
//! concrete solver in [`crate::puzzle_csp`] implements these abstractions for the
//! Mathdoku grid.
//!
//! ## Structure
//!
//! A CSP consists of:
//!
//! - **Variables** — decision points, each participating in a set of constraints.
//! - **Constraints** — relations over subsets of variables (the constraint's *scope*) that rule out
//!   inconsistent value combinations.
//!
//! The relationship between variables and constraints forms a bipartite *constraint graph*:
//! variables on one side, constraints on the other, with edges connecting each variable to
//! the constraints whose scope includes it.
//!
//! ## Propagation
//!
//! [`generalized_arc_consistency`] enforces GAC via the AC-3 worklist algorithm: it
//! propagates each constraint in turn, and whenever a variable's domain shrinks, it
//! re-queues all constraints adjacent to that variable. The algorithm terminates at a
//! fixpoint where no constraint can narrow any domain further.

use std::collections::VecDeque;

/// A state that maps variables of type `V` to domains of type `D`.
pub trait State<V, D, E> {
    /// Get the domain of the specified variable.
    fn get(&self, variable: V) -> Result<D, E>;
}

/// A relation over a set of variables (the constraint's scope) in a constraint satisfaction
/// problem.
///
/// A constraint is satisfied when the values assigned to its scope variables are jointly
/// consistent. Propagation enforces generalized arc consistency (GAC): it removes from each
/// variable's domain any value not supported by some consistent tuple over the scope.
pub trait Constraint<S, V, D, E>: Sized + Clone
where
    S: State<V, D, E>,
{
    /// Applies this constraint to `state`, returning the updated state and the variables
    /// whose domains were narrowed.
    ///
    /// # Errors
    /// Returns an error if propagation fails (e.g. a cell is out of bounds).
    fn propagate(&self, state: &S) -> Result<(S, Vec<V>), E>;

    /// Is `variable` in the scope of this constraint?
    fn in_scope(&self, variable: V) -> bool;
}

/// Enforces generalized arc consistency (GAC) via the AC-3 worklist algorithm.
///
/// AC-3 operates on the constraint graph, a bipartite graph with variables on one side and
/// constraints on the other. It maintains a queue of constraints to process. When a constraint
/// is propagated and reduces a variable's domain, all constraints adjacent to that variable are
/// re-added to the queue. The algorithm terminates when the queue is empty, at which point the
/// state is arc-consistent: no constraint can reduce any variable's domain further.
///
/// # Errors
/// Returns the first error from any constraint's [`Constraint::propagate`] call.
pub fn generalized_arc_consistency<S, V, C, D, E>(mut state: S, constraints: &[C]) -> Result<S, E>
where
    S: State<V, D, E>,
    V: Clone,
    C: Constraint<S, V, D, E>,
{
    let mut q: VecDeque<C> = constraints.iter().cloned().collect();
    while let Some(constraint) = q.pop_front() {
        let narrowed_variables;
        (state, narrowed_variables) = constraint.propagate(&state)?;
        q.extend(
            constraints
                .iter()
                .filter(|c| narrowed_variables.iter().any(|v| c.in_scope(v.clone())))
                .cloned(),
        );
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::{Constraint, State, generalized_arc_consistency};
    use crate::csp::tests::Constraints::{Equal, Sum};
    use std::collections::{HashMap, HashSet};

    #[test]
    fn equal_overlapping_domains_intersects_both() {
        // x ∈ {1,2,3}, y ∈ {2,3,4}, x=y  →  both {2,3}
        let state = IntegerSets::new(&[("x", &[1, 2, 3]), ("y", &[2, 3, 4])]);
        let result = run(state, &[Constraints::equal("x", "y")]);
        assert_eq!(sorted(&result, "x"), [2, 3]);
        assert_eq!(sorted(&result, "y"), [2, 3]);
    }

    #[test]
    fn equal_singleton_pins_other_variable() {
        // x ∈ {5}, y ∈ {1,2,3,4,5}, x=y  →  both {5}
        let state = IntegerSets::new(&[("x", &[5]), ("y", &[1, 2, 3, 4, 5])]);
        let result = run(state, &[Constraints::equal("x", "y")]);
        assert_eq!(sorted(&result, "x"), [5]);
        assert_eq!(sorted(&result, "y"), [5]);
    }

    #[test]
    fn equal_disjoint_domains_empties_both() {
        // x ∈ {1,2}, y ∈ {3,4}, x=y  →  both empty (infeasible)
        let state = IntegerSets::new(&[("x", &[1, 2]), ("y", &[3, 4])]);
        let result = run(state, &[Constraints::equal("x", "y")]);
        assert!(result.get("x".to_string()).unwrap().is_empty());
        assert!(result.get("y".to_string()).unwrap().is_empty());
    }

    #[test]
    fn sum_two_vars_prunes_unsupported_values() {
        // x,y ∈ {1,2,3}, x+y=5  →  only (2,3),(3,2) work, so x,y ∈ {2,3}
        let state = IntegerSets::new(&[("x", &[1, 2, 3]), ("y", &[1, 2, 3])]);
        let result = run(state, &[Constraints::sum(&["x", "y"], 5)]);
        assert_eq!(sorted(&result, "x"), [2, 3]);
        assert_eq!(sorted(&result, "y"), [2, 3]);
    }

    #[test]
    fn sum_three_vars_all_values_survive() {
        // x,y,z ∈ {1,2,3}, x+y+z=6  →  permutations of (1,2,3) use every value
        let state = IntegerSets::new(&[("x", &[1, 2, 3]), ("y", &[1, 2, 3]), ("z", &[1, 2, 3])]);
        let result = run(state, &[Constraints::sum(&["x", "y", "z"], 6)]);
        assert_eq!(sorted(&result, "x"), [1, 2, 3]);
        assert_eq!(sorted(&result, "y"), [1, 2, 3]);
        assert_eq!(sorted(&result, "z"), [1, 2, 3]);
    }

    #[test]
    fn sum_infeasible_target_empties_domains() {
        // x,y ∈ {1,2}, x+y=10 — impossible
        let state = IntegerSets::new(&[("x", &[1, 2]), ("y", &[1, 2])]);
        let result = run(state, &[Constraints::sum(&["x", "y"], 10)]);
        assert!(result.get("x".to_string()).unwrap().is_empty());
        assert!(result.get("y".to_string()).unwrap().is_empty());
    }

    #[test]
    fn propagation_chains_across_constraints() {
        // x,y,z ∈ {1,2,3}; x+y=5 pins x,y ∈ {2,3}; then x=z chains to pin z ∈ {2,3}
        let state = IntegerSets::new(&[("x", &[1, 2, 3]), ("y", &[1, 2, 3]), ("z", &[1, 2, 3])]);
        let result = run(
            state,
            &[
                Constraints::sum(&["x", "y"], 5),
                Constraints::equal("x", "z"),
            ],
        );
        assert_eq!(sorted(&result, "x"), [2, 3]);
        assert_eq!(sorted(&result, "y"), [2, 3]);
        assert_eq!(sorted(&result, "z"), [2, 3]);
    }

    fn run(state: IntegerSets, constraints: &[Constraints]) -> IntegerSets {
        generalized_arc_consistency(state, constraints).unwrap()
    }

    fn sorted(result: &IntegerSets, var: &str) -> Vec<u8> {
        let mut v: Vec<u8> = result.get(var.to_string()).unwrap().into_iter().collect();
        v.sort_unstable();
        v
    }

    type Domain = HashSet<u8>;

    struct IntegerSets(HashMap<String, Domain>);

    impl IntegerSets {
        fn new(init: &[(&str, &[u8])]) -> Self {
            Self(
                init.iter()
                    .map(|(k, v)| (k.to_string(), v.iter().copied().collect()))
                    .collect(),
            )
        }
    }
    impl State<String, Domain, InvalidVariable> for IntegerSets {
        fn get(&self, variable: String) -> Result<Domain, InvalidVariable> {
            self.0
                .get(&variable)
                .cloned()
                .ok_or(InvalidVariable(variable))
        }
    }
    #[derive(Debug)]
    struct InvalidVariable(String);

    #[derive(Clone)]
    enum Constraints {
        Equal(String, String),
        Sum(Vec<String>, u8),
    }

    impl Constraints {
        fn equal(a: &str, b: &str) -> Self {
            Equal(a.to_string(), b.to_string())
        }
        fn sum(vars: &[&str], target: u8) -> Self {
            Sum(vars.iter().map(ToString::to_string).collect(), target)
        }
    }

    impl Constraint<IntegerSets, String, Domain, InvalidVariable> for Constraints {
        fn propagate(
            &self,
            state: &IntegerSets,
        ) -> Result<(IntegerSets, Vec<String>), InvalidVariable> {
            // Returns updated state and names of variables whose domains shrank.
            let update = |name_a: &str,
                          old_a: &Domain,
                          new_a: Domain,
                          name_b: &str,
                          old_b: &Domain,
                          new_b: Domain|
             -> (IntegerSets, Vec<String>) {
                let mut changed = vec![];
                if &new_a != old_a {
                    changed.push(name_a.to_string());
                }
                if &new_b != old_b {
                    changed.push(name_b.to_string());
                }
                let new_state = IntegerSets(HashMap::from([
                    (name_a.to_string(), new_a),
                    (name_b.to_string(), new_b),
                ]));
                (new_state, changed)
            };

            match self {
                Equal(a, b) => {
                    let da = state.get(a.clone())?;
                    let db = state.get(b.clone())?;
                    let common: Domain = da.intersection(&db).copied().collect();
                    Ok(update(a, &da, common.clone(), b, &db, common))
                }
                Sum(vars, target) => {
                    let da = state.get(vars[0].clone())?;
                    let db = state.get(vars[1].clone())?;
                    let mut new_a: Domain = HashSet::new();
                    let mut new_b: Domain = HashSet::new();
                    for &x in &da {
                        for &y in &db {
                            if x + y == *target {
                                let _ = new_a.insert(x);
                                let _ = new_b.insert(y);
                            }
                        }
                    }
                    Ok(update(&vars[0], &da, new_a, &vars[1], &db, new_b))
                }
            }
        }

        fn in_scope(&self, variable: String) -> bool {
            match self {
                Equal(a, b) => a == &variable || b == &variable,
                Sum(vars, _) => vars.contains(&variable),
            }
        }
    }
}
