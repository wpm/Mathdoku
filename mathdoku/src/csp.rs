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
pub fn generalized_arc_consistency<S, V, C, D, E>(state: S, constraints: &[C]) -> Result<S, E>
where
    S: State<V, D, E>,
    C: Constraint<S, V, D, E>,
{
    let state = state;
    let mut q = VecDeque::from(constraints);
    while let Some(constraint) = q.pop_front() {
        let narrowed_variables;
        (state, narrowed_variables) = constraint.propagate(&state)?;
        q.extend(
            constraints.filter(|constraint| {
                narrowed_variables.any(|variable| constraint.in_scope(variable))
            }),
        );
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::{Constraint, State, generalized_arc_consistency};
    use crate::csp::tests::Constraints::{Equal, Sum};
    use std::collections::{HashMap, HashSet};

    fn run(state: IntegerSets, constraints: &[Constraints]) -> IntegerSets {
        generalized_arc_consistency(state, constraints).unwrap()
    }

    fn sorted(result: &IntegerSets, var: &str) -> Vec<u8> {
        let mut v: Vec<u8> = result.get(var.to_string()).unwrap().into_iter().collect();
        v.sort_unstable();
        v
    }

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
            Sum(vars.iter().map(|s| s.to_string()).collect(), target)
        }
    }

    impl Constraint<IntegerSets, String, Domain, InvalidVariable> for Constraints {
        fn propagate(
            &self,
            state: &IntegerSets,
        ) -> Result<(IntegerSets, Vec<String>), InvalidVariable> {
            // Create a new state and determine which variables were changed.
            let r = |a, b| -> (IntegerSets, Vec<String>) {
                let (a, old_a, narrowed_a) = a;
                let (b, old_b, narrowed_b) = b;
                let mut changed_variables: Vec<String> = vec![];
                let new_state = IntegerSets::new(&[(a, narrowed_a), (b, narrowed_b)]);
                if old_a != narrowed_a {
                    changed_variables.push(a.into());
                }
                if old_b != narrowed_b {
                    changed_variables.push(b.into());
                }
                (new_state, changed_variables)
            };

            match self {
                Equal(a, b) => {
                    let current_a = state.get(*a)?;
                    let current_b = state.get(*b)?;
                    let common = current_a.intersection(*current_b).collect();
                    Ok(r((a, current_a, common), (b, current_b, common)))
                }
                Sum(vars, target) => {
                    let current_a = state.get(vars[0])?;
                    let current_b = state.get(vars[1])?;
                    let (narrowed_a, narrowed_b) = current_a.fold(
                        (HashSet::new(), HashSet::new()),
                        |(mut acc_a, mut acc_b), x| {
                            current_b
                                .map(|y| (x, y))
                                .filter(|(x, y)| x + y == target)
                                .for_each(|(x, y)| {
                                    acc_a.insert(x);
                                    acc_b.insert(y);
                                });
                            (acc_a, acc_b)
                        },
                    );
                    Ok(r(
                        (vars[0], current_a, narrowed_a),
                        (vars[1], current_b, narrowed_b),
                    ))
                }
            }
        }

        fn in_scope(&self, variable: String) -> bool {
            match self {
                Equal(a, b) => [*a, *b].contains(&variable),
                Sum(vars, _) => vars.contains(&variable),
            }
        }
    }
}
