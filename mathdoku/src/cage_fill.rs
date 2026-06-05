//! Cage fills: cached constraint data structures for fast GAC support computation.
//!
//! Every [`Cage`] holds a cage fill once a grid size `n` is known. The fill
//! pre-computes the set of valid value assignments so that [`support`] does not
//! re-enumerate them on every propagation call.
//!
//! | Operator        | Fill type     | Construction             |
//! |-----------------|---------------|--------------------------|
//! | Add / Multiply  | [`MonotonicMDD`] | MDD-4R (incremental)  |
//! | Subtract / Divide / Given | [`Trie`] / [`GivenFill`] | Odometer enumeration |
//!
//! All fill types implement [`CageFill`]. [`CageFillKind`] is the enum that
//! `Cage` stores, dispatching to the appropriate implementation.

#![allow(clippy::redundant_pub_crate)] // pub(crate) items in a private module

use crate::Fill;
use crate::Target;
use crate::mdd::{Constraint, MonotonicConstraint, MonotonicMDD};

/// Operators whose constraint is non-monotonic — valid tuples are enumerated
/// by odometer and stored in a trie. Given is handled by [`GivenFill`] instead.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum NonMonotonicOp {
    Subtract,
    Divide,
}

// ---- CageFill trait ----

/// A pre-built constraint cache that returns per-cell GAC support values.
///
/// `support` is conceptually pure — the fill is just a cached helper.
pub(crate) trait CageFill {
    /// Returns the per-cell GAC support given the current cell domains.
    ///
    /// Each entry in the returned `Vec` is the subset of the corresponding
    /// input domain that appears in at least one valid tuple consistent with
    /// all current domains.
    fn support(&self, values: &[Fill]) -> Vec<Fill>;

    /// Returns `true` if no valid tuple exists (cage is infeasible).
    fn is_empty(&self) -> bool;
}

// ---- GivenFill ----

/// Cage fill for `Given` cages: a single fixed value.
#[derive(Debug, Clone)]
pub(crate) struct GivenFill {
    value: crate::N,
}

impl GivenFill {
    #[allow(clippy::cast_possible_truncation)] // target is always 1..=9 for Given
    pub(crate) const fn new(target: Target) -> Self {
        Self {
            value: target as crate::N,
        }
    }
}

impl CageFill for GivenFill {
    fn support(&self, values: &[Fill]) -> Vec<Fill> {
        debug_assert_eq!(values.len(), 1);
        let v = self.value;
        if values.first().is_some_and(|d| d.contains(v)) {
            vec![Fill::singleton(v)]
        } else {
            vec![Fill::default()]
        }
    }

    fn is_empty(&self) -> bool {
        false // A Given cage is only built when the value is in range.
    }
}

// ---- Trie ----

/// A node in the [`Trie`]. Each node maps an edge label (value `1..=n`) to a
/// child node index. The sentinel index `usize::MAX` marks a terminal (valid
/// tuple prefix completed).
#[derive(Debug, Clone)]
struct TrieNode {
    /// `children[v-1]` = child node index for label `v`, or `usize::MAX` if absent.
    children: Vec<usize>,
}

const TERMINAL: usize = usize::MAX;

impl TrieNode {
    fn new(n: usize) -> Self {
        Self {
            children: vec![TERMINAL; n],
        }
    }

    fn child(&self, v: crate::N) -> usize {
        self.children[v as usize - 1]
    }

    fn set_child(&mut self, v: crate::N, idx: usize) {
        self.children[v as usize - 1] = idx;
    }
}

/// Cage fill for non-monotonic operators (Subtract, Divide).
///
/// Stores the complete set of valid tuples as a prefix tree over `1..=n`.
/// Construction enumerates all `n^arity` tuples via an odometer and keeps
/// those that satisfy the operator constraint. Since Subtract and Divide are
/// always 2-cell, the trie is tiny (at most `2n` paths for any `n`).
///
/// [`support`] walks the trie, pruning branches incompatible with the current
/// cell domains, and collects per-position support values — the same structure
/// as [`MonotonicMDD::support`] but over the trie's explicit edges.
#[derive(Debug, Clone)]
pub(crate) struct Trie {
    nodes: Vec<TrieNode>,
    arity: usize,
    n: usize,
}

impl Trie {
    /// Builds a trie for `op`/`target` over the domain `1..=n` with `arity` variables.
    pub(crate) fn new(n: u32, op: NonMonotonicOp, target: Target, arity: usize) -> Self {
        let n = n as usize;
        let mut trie = Self {
            nodes: vec![TrieNode::new(n)], // root at index 0
            arity,
            n,
        };
        #[allow(clippy::cast_possible_truncation)] // n ≤ 9
        let n_val: crate::N = n as crate::N;

        // Odometer over arity-tuples of values 1..=n.
        let mut tuple: Vec<crate::N> = vec![1; arity];
        loop {
            if Self::satisfies(op, target, &tuple) {
                trie.insert(&tuple);
            }
            // Advance odometer.
            let mut pos = 0;
            loop {
                tuple[pos] += 1;
                if tuple[pos] <= n_val {
                    break;
                }
                tuple[pos] = 1;
                pos += 1;
                if pos == arity {
                    return trie;
                }
            }
        }
    }

    fn satisfies(op: NonMonotonicOp, target: Target, tuple: &[crate::N]) -> bool {
        match op {
            NonMonotonicOp::Subtract => {
                tuple.len() == 2
                    && Target::from(tuple[0]).abs_diff(Target::from(tuple[1])) == target
            }
            NonMonotonicOp::Divide => {
                tuple.len() == 2 && {
                    let (a, b) = (Target::from(tuple[0]), Target::from(tuple[1]));
                    a == b * target || b == a * target
                }
            }
        }
    }

    fn insert(&mut self, tuple: &[crate::N]) {
        let mut node_idx = 0;
        for &v in tuple {
            let child = self.nodes[node_idx].child(v);
            if child == TERMINAL {
                // Allocate a new node (or mark terminal at last level).
                let new_idx = self.nodes.len();
                self.nodes[node_idx].set_child(v, new_idx);
                self.nodes.push(TrieNode::new(self.n));
                node_idx = new_idx;
            } else {
                node_idx = child;
            }
        }
    }

    /// Walks the trie collecting per-position support for the given domains.
    ///
    /// Support is accumulated only for complete paths (tuples that satisfy
    /// every position's domain constraint), so a value at position `i` only
    /// contributes to support if the rest of the tuple can also be satisfied.
    fn walk(
        &self,
        node_idx: usize,
        depth: usize,
        values: &[Fill],
        path: &mut Vec<crate::N>,
        support: &mut Vec<Fill>,
    ) {
        if depth == self.arity {
            // Complete path — add all collected values to support.
            for (i, &v) in path.iter().enumerate() {
                support[i] = support[i] | Fill::singleton(v);
            }
            return;
        }
        let node = &self.nodes[node_idx];
        #[allow(clippy::cast_possible_truncation)] // n ≤ 9
        let n_val: crate::N = self.n as crate::N;
        for v in 1..=n_val {
            let child = node.child(v);
            if child == TERMINAL {
                continue;
            }
            if !values[depth].contains(v) {
                continue;
            }
            path.push(v);
            self.walk(child, depth + 1, values, path, support);
            let _ = path.pop();
        }
    }
}

impl CageFill for Trie {
    fn support(&self, values: &[Fill]) -> Vec<Fill> {
        let mut support = vec![Fill::default(); self.arity];
        let mut path: Vec<crate::N> = Vec::with_capacity(self.arity);
        self.walk(0, 0, values, &mut path, &mut support);
        support
    }

    fn is_empty(&self) -> bool {
        // The trie is empty iff the root has no outgoing edges.
        self.nodes[0].children.iter().all(|&c| c == TERMINAL)
    }
}

// ---- MonotonicMDD as CageFill ----

impl CageFill for MonotonicMDD {
    fn support(&self, values: &[Fill]) -> Vec<Fill> {
        self.support(values)
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

// ---- CageFillKind ----

/// Dispatching enum over all cage fill types.
#[derive(Debug, Clone)]
pub(crate) enum CageFillKind {
    Mdd(MonotonicMDD),
    Trie(Trie),
    Given(GivenFill),
}

impl CageFill for CageFillKind {
    fn support(&self, values: &[Fill]) -> Vec<Fill> {
        match self {
            Self::Mdd(m) => m.support(values),
            Self::Trie(t) => t.support(values),
            Self::Given(g) => g.support(values),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::Mdd(m) => m.is_empty(),
            Self::Trie(t) => t.is_empty(),
            Self::Given(g) => g.is_empty(),
        }
    }
}

/// Builds the appropriate [`CageFillKind`] for `operator`/`target`/`arity` over `1..=n`.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn build_fill(
    n: u32,
    operator: crate::Operator,
    target: Target,
    arity: usize,
) -> CageFillKind {
    use crate::Operator;
    match operator {
        Operator::Add => CageFillKind::Mdd(MonotonicMDD::new(
            n,
            MonotonicConstraint::Sum(Constraint {
                target: target as u32,
                arity: arity as u32,
            }),
        )),
        Operator::Multiply => CageFillKind::Mdd(MonotonicMDD::new(
            n,
            MonotonicConstraint::Product(Constraint {
                target: target as u32,
                arity: arity as u32,
            }),
        )),
        Operator::Given => CageFillKind::Given(GivenFill::new(target)),
        Operator::Subtract => {
            CageFillKind::Trie(Trie::new(n, NonMonotonicOp::Subtract, target, arity))
        }
        Operator::Divide => CageFillKind::Trie(Trie::new(n, NonMonotonicOp::Divide, target, arity)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Operator;

    fn full_domain(n: usize) -> Fill {
        Fill::all(n)
    }

    // ---- GivenFill ----

    #[test]
    fn given_fill_support_returns_singleton() {
        let fill = GivenFill::new(3);
        let result = fill.support(&[full_domain(4)]);
        assert_eq!(result, vec![Fill::singleton(3)]);
    }

    #[test]
    fn given_fill_not_in_domain_returns_empty() {
        let fill = GivenFill::new(5);
        let result = fill.support(&[Fill::new(&[1, 2, 3]).unwrap()]);
        assert!(result[0].is_empty());
    }

    #[test]
    fn given_fill_is_not_empty() {
        assert!(!GivenFill::new(3).is_empty());
    }

    // ---- Trie (Subtract) ----

    #[test]
    fn subtract_trie_n3_target1_has_correct_support() {
        // Subtract 1 in a 3×3: valid pairs are (1,2),(2,1),(2,3),(3,2).
        let trie = Trie::new(3, NonMonotonicOp::Subtract, 1, 2);
        let result = trie.support(&[full_domain(3), full_domain(3)]);
        let mut pos0 = result[0].values();
        let mut pos1 = result[1].values();
        pos0.sort_unstable();
        pos1.sort_unstable();
        assert_eq!(pos0, [1, 2, 3]);
        assert_eq!(pos1, [1, 2, 3]);
    }

    #[test]
    fn subtract_trie_respects_domain_constraints() {
        // Subtract 1, pin pos0 to {3}: only (3,2) survives, so pos1={2}.
        let trie = Trie::new(3, NonMonotonicOp::Subtract, 1, 2);
        let result = trie.support(&[Fill::new(&[3]).unwrap(), full_domain(3)]);
        assert_eq!(result[1], Fill::new(&[2]).unwrap());
    }

    #[test]
    fn subtract_trie_infeasible_target_is_empty() {
        // Subtract 9 in a 3×3 — impossible.
        let trie = Trie::new(3, NonMonotonicOp::Subtract, 9, 2);
        assert!(trie.is_empty());
    }

    // ---- Trie (Divide) ----

    #[test]
    fn divide_trie_n4_target2_support() {
        // Divide 2 in a 4×4: valid pairs are (1,2),(2,1),(2,4),(4,2).
        let trie = Trie::new(4, NonMonotonicOp::Divide, 2, 2);
        let result = trie.support(&[full_domain(4), full_domain(4)]);
        let mut pos0 = result[0].values();
        pos0.sort_unstable();
        assert_eq!(pos0, [1, 2, 4]);
    }

    // ---- CageFillKind (via build_fill) ----

    #[test]
    fn build_fill_add_returns_mdd() {
        let fill = build_fill(3, Operator::Add, 4, 2);
        assert!(matches!(fill, CageFillKind::Mdd(_)));
        assert!(!fill.is_empty());
    }

    #[test]
    fn build_fill_subtract_returns_trie() {
        let fill = build_fill(4, Operator::Subtract, 1, 2);
        assert!(matches!(fill, CageFillKind::Trie(_)));
        assert!(!fill.is_empty());
    }

    #[test]
    fn build_fill_given_returns_given_fill() {
        let fill = build_fill(4, Operator::Given, 3, 1);
        assert!(matches!(fill, CageFillKind::Given(_)));
    }

    #[test]
    fn trie_subtract_matches_brute_force() {
        // Cross-check Trie::support against independent enumeration for n=4, target=1.
        let n: crate::N = 4;
        let trie = Trie::new(n.into(), NonMonotonicOp::Subtract, 1, 2);
        let domains = [full_domain(n as usize), full_domain(n as usize)];
        let trie_result = trie.support(&domains);

        // Brute-force oracle.
        let mut oracle = [Fill::default(); 2];
        for a in 1..=n {
            for b in 1..=n {
                if u64::from(a).abs_diff(u64::from(b)) == 1 {
                    oracle[0] = oracle[0] | Fill::singleton(a);
                    oracle[1] = oracle[1] | Fill::singleton(b);
                }
            }
        }
        assert_eq!(trie_result[0], oracle[0]);
        assert_eq!(trie_result[1], oracle[1]);
    }

    #[test]
    fn trie_divide_matches_brute_force() {
        // Cross-check Trie::support against independent enumeration for n=4, target=2.
        let n: crate::N = 4;
        let trie = Trie::new(n.into(), NonMonotonicOp::Divide, 2, 2);
        let domains = [full_domain(n as usize), full_domain(n as usize)];
        let trie_result = trie.support(&domains);

        // Brute-force oracle.
        let mut oracle = [Fill::default(); 2];
        for a in 1..=n {
            for b in 1..=n {
                let (va, vb) = (u64::from(a), u64::from(b));
                if va == vb * 2 || vb == va * 2 {
                    oracle[0] = oracle[0] | Fill::singleton(a);
                    oracle[1] = oracle[1] | Fill::singleton(b);
                }
            }
        }
        assert_eq!(trie_result[0], oracle[0]);
        assert_eq!(trie_result[1], oracle[1]);
    }

    #[test]
    fn build_fill_multiply_returns_mdd() {
        let fill = build_fill(4, Operator::Multiply, 6, 2);
        assert!(matches!(fill, CageFillKind::Mdd(_)));
        assert!(!fill.is_empty());
    }

    #[test]
    fn build_fill_divide_returns_trie() {
        let fill = build_fill(4, Operator::Divide, 2, 2);
        assert!(matches!(fill, CageFillKind::Trie(_)));
        assert!(!fill.is_empty());
    }
}
