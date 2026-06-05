//! Multi-valued decision diagram (MDD) construction for monotonic constraints.
//!
//! An MDD encodes all solutions to a constraint as a directed acyclic graph.
//! Each path from the root to a terminal node represents one assignment of
//! values to variables that satisfies the constraint.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

use log::debug;

/// An MDD the enforces that either the sum or the products of all the values
/// equal a designated target.
///
/// Each internal node maps to the list of outgoing edges. A node absent from
/// `edges` is a terminal — its path through the diagram reached the constraint
/// target.
#[must_use]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MonotonicMDD {
    /// Maximum value any variable may take (values range over `1..=n`).
    n: u32,
    constraint: MonotonicConstraint,
    /// Adjacency map: head node → `(edge label, tail node)` pairs.
    edges: HashMap<Node, Vec<(u32, Node)>>,
}

impl MonotonicMDD {
    /// Build an MDD for `constraint` over the domain `1..=n`.
    pub fn new(n: u32, constraint: MonotonicConstraint) -> Self {
        let mut mmd = Self {
            n,
            constraint,
            edges: HashMap::new(),
        };
        let root = Node {
            depth: 0,
            value: constraint.unit(),
        };
        mmd.subtree(root);
        mmd
    }

    /// Recursively insert all edges reachable from `head`.
    ///
    /// Stops when the accumulated value reaches the constraint target or when
    /// the node is at the last level (depth == arity).
    fn subtree(&mut self, head: Node) {
        if self.edges.contains_key(&head) {
            return;
        }
        debug!("{self}");
        let remaining = self.constraint.arity() - head.depth - 1;
        for i in 1..=self.n {
            if self.constraint.pruned(head.value, i, remaining) {
                break;
            }
            if self.constraint.skipped(head.value, i, remaining, self.n) {
                continue;
            }
            let tail = Node {
                depth: head.depth + 1,
                value: self.constraint.operation(head.value, i),
            };
            self.insert_edge(head, i, tail);
            if !self.at_target(tail) && !self.at_arity(tail) {
                self.subtree(tail);
            }
        }
    }

    /// Return a copy of this MDD with `values` removed from support.
    ///
    /// `values` maps a variable index (0-based depth) to the set of domain values to
    /// forbid at that layer. Uses the MDD-4R algorithm: for each affected layer, choose
    /// between deleting dead arcs individually or resetting the whole layer, whichever
    /// is cheaper, then cascade unreachable nodes downward (`Q↓`) and dead-end nodes
    /// upward (`Q↑`).
    fn remove_support(&self, values: &HashMap<u32, HashSet<u32>>) -> Self {
        let mut mdd = Self {
            n: self.n,
            constraint: self.constraint,
            edges: self.edges.clone(),
        };
        let mut q_down: Vec<Node> = Vec::new();
        let mut q_up: Vec<Node> = Vec::new();

        for (&depth, forbidden) in values {
            let heads = mdd.heads_at_depth(depth);
            let total_arcs: usize = heads
                .iter()
                .filter_map(|h| mdd.edges.get(h))
                .map(Vec::len)
                .sum();
            let dead_arcs: usize = heads
                .iter()
                .filter_map(|h| mdd.edges.get(h))
                .flat_map(|es| es.iter())
                .filter(|(label, _)| forbidden.contains(label))
                .count();

            if dead_arcs > total_arcs / 2 {
                debug!("Layer {depth}: reset ({dead_arcs}/{total_arcs} arcs dead)");
                mdd.reset_layer(&heads, forbidden, &mut q_down, &mut q_up);
            } else {
                debug!("Layer {depth}: delete ({dead_arcs}/{total_arcs} arcs dead)");
                mdd.delete_layer(&heads, forbidden, &mut q_down, &mut q_up);
            }
        }

        mdd.cascade_down(&mut q_down);
        mdd.cascade_up(&mut q_up);
        mdd
    }

    /// Collect all head nodes at the given depth.
    fn heads_at_depth(&self, depth: u32) -> Vec<Node> {
        self.edges
            .keys()
            .filter(|n| n.depth == depth)
            .copied()
            .collect()
    }

    /// Collect all tail nodes reachable from `heads`.
    fn tails_of(edges: &HashMap<Node, Vec<(u32, Node)>>, heads: &[Node]) -> HashSet<Node> {
        heads
            .iter()
            .filter_map(|h| edges.get(h))
            .flat_map(|es| es.iter())
            .map(|(_, t)| *t)
            .collect()
    }

    /// Wipe a layer and rebuild it from surviving values, queuing displaced nodes.
    fn reset_layer(
        &mut self,
        heads: &[Node],
        forbidden: &HashSet<u32>,
        q_down: &mut Vec<Node>,
        q_up: &mut Vec<Node>,
    ) {
        let surviving: HashSet<u32> = (1..=self.n).filter(|v| !forbidden.contains(v)).collect();
        let tails_before = Self::tails_of(&self.edges, heads);

        // Snapshot then wipe the layer; rebuild from the snapshot.
        let orig: Vec<(Node, Vec<(u32, Node)>)> = heads
            .iter()
            .filter_map(|h| self.edges.remove(h).map(|es| (*h, es)))
            .collect();
        for (head, orig_edges) in orig {
            let new_edges: Vec<(u32, Node)> = orig_edges
                .into_iter()
                .filter(|(label, _)| surviving.contains(label))
                .collect();
            if !new_edges.is_empty() {
                let _ = self.edges.insert(head, new_edges);
            }
        }

        let tails_after = Self::tails_of(&self.edges, heads);
        q_down.extend(
            tails_before
                .into_iter()
                .filter(|t| !tails_after.contains(t)),
        );
        q_up.extend(
            heads
                .iter()
                .filter(|h| !self.edges.contains_key(h))
                .copied(),
        );
    }

    /// Remove forbidden-labelled arcs from a layer individually, queuing displaced nodes.
    fn delete_layer(
        &mut self,
        heads: &[Node],
        forbidden: &HashSet<u32>,
        q_down: &mut Vec<Node>,
        q_up: &mut Vec<Node>,
    ) {
        for head in heads {
            if let Some(es) = self.edges.get_mut(head) {
                let dead_tails: Vec<Node> = es
                    .iter()
                    .filter(|(label, _)| forbidden.contains(label))
                    .map(|(_, t)| *t)
                    .collect();
                es.retain(|(label, _)| !forbidden.contains(label));
                if es.is_empty() {
                    let _ = self.edges.remove(head);
                    q_up.push(*head);
                }
                for tail in dead_tails {
                    let still_reachable = heads.iter().any(|h| {
                        self.edges
                            .get(h)
                            .is_some_and(|es| es.iter().any(|(_, t)| *t == tail))
                    });
                    if !still_reachable {
                        q_down.push(tail);
                    }
                }
            }
        }
    }

    /// Remove nodes in `q` that have no incoming arcs (unreachable from root), cascading
    /// to their children.
    fn cascade_down(&mut self, q: &mut Vec<Node>) {
        while let Some(node) = q.pop() {
            if !self.edges.contains_key(&node) {
                continue;
            }
            // Only nodes at the preceding depth can have edges into this node.
            let has_incoming = node.depth > 0
                && self
                    .edges
                    .keys()
                    .filter(|h| h.depth == node.depth - 1)
                    .any(|h| self.edges[h].iter().any(|(_, t)| *t == node));
            if !has_incoming {
                let outgoing = self.edges.remove(&node).unwrap_or_default();
                for (_, tail) in outgoing {
                    q.push(tail);
                }
            }
        }
    }

    /// Remove nodes in `q` that have no outgoing arcs and are not terminals, cascading
    /// to their predecessors.
    fn cascade_up(&mut self, q: &mut Vec<Node>) {
        while let Some(node) = q.pop() {
            if self.edges.contains_key(&node) {
                continue;
            }
            let is_terminal =
                node.value == self.constraint.target() && node.depth == self.constraint.arity();
            if !is_terminal {
                // Only nodes at the preceding depth can have edges into this node.
                let heads: Vec<Node> = self
                    .edges
                    .keys()
                    .filter(|h| h.depth + 1 == node.depth)
                    .copied()
                    .collect();
                for head in heads {
                    if let Some(es) = self.edges.get_mut(&head) {
                        es.retain(|(_, t)| *t != node);
                        if es.is_empty() {
                            let _ = self.edges.remove(&head);
                            q.push(head);
                        }
                    }
                }
            }
        }
    }

    /// Add an edge labelled `value` from `head` to `tail`, creating `head`'s edge list if absent.
    fn insert_edge(&mut self, head: Node, value: u32, tail: Node) {
        Self::indented_debug(head.depth, &format!("{head} -{value}→ {tail}"));
        self.edges.entry(head).or_default().push((value, tail));
    }

    /// Returns `true` if `tail` is at the arity limit, panicking if it exceeded it.
    fn at_arity(&self, tail: Node) -> bool {
        let (d, a) = (u64::from(tail.depth), u64::from(self.constraint.arity()));
        assert!(d <= a, "depth {d} > arity {a}");
        let reached = d == a;
        if reached {
            Self::indented_debug(tail.depth, &format!("{tail} Arity limit met"));
        }
        reached
    }

    /// Returns `true` if further recursion from `node` cannot reach the constraint target.
    /// Equality with the target is prunable for sum (adding more can only increase the value)
    /// but not for product (multiplying by 1 stays at the target). See [`MonotonicConstraint::target_reached`].
    fn at_target(&self, node: Node) -> bool {
        let reached = self.constraint.target_reached(node.value);
        if reached {
            Self::indented_debug(node.depth, &format!("{node} Target reached"));
        }
        reached
    }

    /// Emit a debug-level log line indented by `depth` spaces.
    fn indented_debug(depth: u32, message: &str) {
        debug!("{:indent$}{message}", "", indent = depth as usize);
    }

    /// Returns `true` if the MDD has no solutions (no edges at all).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    /// Enumerates every solution tuple by walking root-to-terminal paths.
    #[must_use]
    pub fn tuples(&self) -> Vec<crate::Tuple> {
        let root = Node {
            depth: 0,
            value: self.constraint.unit(),
        };
        let mut result = Vec::new();
        self.collect_paths(root, &mut Vec::new(), &mut result);
        result
    }

    fn collect_paths(&self, head: Node, path: &mut crate::Tuple, result: &mut Vec<crate::Tuple>) {
        match self.edges.get(&head) {
            None => {
                if head.value == self.constraint.target() && head.depth == self.constraint.arity() {
                    result.push(path.clone());
                }
            }
            Some(edges) => {
                for &(label, tail) in edges {
                    #[allow(clippy::cast_possible_truncation)]
                    path.push(label as crate::Value);
                    self.collect_paths(tail, path, result);
                    let _ = path.pop();
                }
            }
        }
    }

    /// Computes per-cell GAC support given one `Values` set per variable.
    ///
    /// Returns the subset of each domain that appears in at least one solution
    /// tuple where every position's value lies in the corresponding `Values` set.
    #[must_use]
    pub fn support(&self, values: &[crate::Values]) -> Vec<crate::Values> {
        let arity = self.constraint.arity() as usize;
        let mut support = vec![crate::Values::default(); arity];
        for tuple in self.tuples() {
            if tuple.iter().zip(values).all(|(&v, d)| d.contains(v)) {
                for (i, &v) in tuple.iter().enumerate() {
                    support[i] = support[i] | crate::Values::singleton(v);
                }
            }
        }
        support
    }
}

impl Display for MonotonicMDD {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MDD({} {} nodes)", self.constraint, self.edges.len())
    }
}

/// Parameters shared by all monotonic constraint variants.
#[must_use]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Constraint {
    /// The value the accumulated result must equal at a terminal node.
    pub target: u32,
    /// Number of variables (levels in the MDD).
    pub arity: u32,
}

/// A constraint whose operation is monotonically non-decreasing.
///
/// Both sum and product accumulate strictly upward as values are assigned,
/// so the MDD can prune a branch as soon as the accumulated value exceeds
/// `target`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MonotonicConstraint {
    /// Variables sum to `target`.
    Sum(Constraint),
    /// Variables multiply to `target`.
    Product(Constraint),
}
impl MonotonicConstraint {
    const fn arity(&self) -> u32 {
        match self {
            Self::Sum(c) | Self::Product(c) => c.arity,
        }
    }

    const fn target(&self) -> u32 {
        match self {
            Self::Sum(c) | Self::Product(c) => c.target,
        }
    }

    /// Returns `true` if a node with accumulated value `v` cannot reach `target` by applying
    /// further operations, so its subtree can be pruned.
    ///
    /// For sum, `v >= target` is prunable: since each label is at least 1, the sum can only
    /// increase, so hitting the target exactly already rules out any valid completion.
    /// For product, only `v > target` is prunable: multiplying by 1 leaves the value unchanged,
    /// so `v == target` can still be completed by assigning 1s for the remaining variables.
    const fn target_reached(&self, v: u32) -> bool {
        match self {
            Self::Sum(c) => v >= c.target,
            Self::Product(c) => v > c.target,
        }
    }

    /// Returns `true` if assigning `v` here makes it impossible to reach `target`,
    /// and all larger values will too — so the caller can `break`.
    ///
    /// For sum: once the partial sum plus `v` exceeds `target`, adding any larger
    /// value only makes it worse.
    /// For product: once the partial product times `v` exceeds `target`, multiplying
    /// by anything larger only makes it worse.
    const fn pruned(&self, acc: u32, v: u32, _remaining: u32) -> bool {
        match self {
            Self::Sum(c) => acc + v > c.target,
            Self::Product(c) => acc * v > c.target,
        }
    }

    /// Returns `true` if assigning `v` here cannot reach `target` even with the
    /// best possible remaining values — but a larger `v` might still work, so the
    /// caller should `continue`.
    ///
    /// For sum: if even assigning `n` for all remaining steps can't reach `target`,
    /// this `v` is too small.
    /// For product: if `target` is not divisible by the running product after `v`,
    /// no completion can hit `target` exactly.
    const fn skipped(&self, acc: u32, v: u32, remaining: u32, n: u32) -> bool {
        match self {
            Self::Sum(c) => acc + v + remaining * n < c.target,
            Self::Product(c) => (acc * v) != 0 && c.target % (acc * v) != 0,
        }
    }

    const fn operation(&self, x: u32, y: u32) -> u32 {
        match self {
            Self::Sum(_) => x + y,
            Self::Product(_) => x * y,
        }
    }

    const fn unit(&self) -> u32 {
        match self {
            Self::Sum(_) => 0,
            Self::Product(_) => 1,
        }
    }
}

impl Display for MonotonicConstraint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (symbol, c) = match self {
            Self::Sum(c) => ('+', c),
            Self::Product(c) => ('×', c),
        };
        write!(f, "{symbol}{} [{}]", c.target, c.arity)
    }
}

/// A node in the MDD, identified by its level and accumulated value so far.
///
/// For a monotonic constraint, the valid continuations from a node depend only on
/// how many variables remain (`arity - depth`) and the accumulated value — not on
/// which specific path was taken to get here. Two nodes with the same `(depth, value)`
/// therefore have identical subtrees and can be merged. This `struct` is used as a
/// `HashMap` key, so `Hash + Eq` on `(depth, value)` *is* the hash-consing: the first
/// visit to a node builds its edge list, and subsequent visits return early, implicitly
/// sharing that list.
#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
struct Node {
    /// Level in the diagram (0 = root).
    depth: u32,
    /// Accumulated constraint value along the path from the root to this node.
    value: u32,
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node({} @ level {})", self.value, self.depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tuple;

    // ---- MonotonicMDD display ----

    #[test]
    fn sum_pair_display() {
        assert_eq!(
            MonotonicMDD::new(3, sum(4, 2)).to_string(),
            "MDD(+4 [2] 4 nodes)"
        );
    }

    #[test]
    fn sum_triple_display() {
        init_logging();
        assert_eq!(
            MonotonicMDD::new(3, sum(5, 3)).to_string(),
            "MDD(+5 [3] 7 nodes)"
        );
    }

    #[test]
    fn sum_triple_larger_n_display() {
        assert_eq!(
            MonotonicMDD::new(4, sum(6, 3)).to_string(),
            "MDD(+6 [3] 9 nodes)"
        );
    }

    #[test]
    fn product_pair_display() {
        assert_eq!(
            MonotonicMDD::new(4, product(6, 2)).to_string(),
            "MDD(×6 [2] 4 nodes)"
        );
    }

    #[test]
    fn product_triple_display() {
        assert_eq!(
            MonotonicMDD::new(4, product(4, 3)).to_string(),
            "MDD(×4 [3] 7 nodes)"
        );
    }

    #[test]
    fn sum_pair_tuples() {
        let mut tuples = MonotonicMDD::new(3, sum(4, 2)).tuples();
        tuples.sort();
        assert_eq!(tuples, vec![vec![1, 3], vec![2, 2], vec![3, 1]]);
    }

    #[test]
    fn sum_triple_tuples() {
        let mut tuples = MonotonicMDD::new(3, sum(5, 3)).tuples();
        tuples.sort();
        assert_eq!(
            tuples,
            vec![
                vec![1, 1, 3],
                vec![1, 2, 2],
                vec![1, 3, 1],
                vec![2, 1, 2],
                vec![2, 2, 1],
                vec![3, 1, 1],
            ]
        );
    }

    #[test]
    fn sum_triple_larger_n_tuples() {
        let mut tuples = MonotonicMDD::new(4, sum(6, 3)).tuples();
        tuples.sort();
        assert_eq!(
            tuples,
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
    fn product_pair_tuples() {
        let mut tuples = MonotonicMDD::new(4, product(6, 2)).tuples();
        tuples.sort();
        assert_eq!(tuples, vec![vec![2, 3], vec![3, 2]]);
    }

    #[test]
    fn product_triple_tuples() {
        let mut tuples = MonotonicMDD::new(4, product(4, 3)).tuples();
        tuples.sort();
        assert_eq!(
            tuples,
            vec![
                vec![1, 1, 4],
                vec![1, 2, 2],
                vec![1, 4, 1],
                vec![2, 1, 2],
                vec![2, 2, 1],
                vec![4, 1, 1],
            ]
        );
    }

    #[test]
    fn sum_arity() {
        assert_eq!(sum(10, 3).arity(), 3);
    }

    #[test]
    fn product_arity() {
        assert_eq!(product(6, 2).arity(), 2);
    }

    #[test]
    fn sum_operation() {
        assert_eq!(sum(7, 2).operation(3, 7), 10);
    }

    #[test]
    fn product_operation() {
        assert_eq!(product(4, 2).operation(3, 4), 12);
    }

    #[test]
    fn sum_display() {
        assert_eq!(sum(10, 3).to_string(), "+10 [3]");
    }

    #[test]
    fn product_display() {
        assert_eq!(product(6, 2).to_string(), "×6 [2]");
    }

    // ---- remove_support ----

    #[test]
    fn remove_support_empty_is_identity() {
        let mdd = MonotonicMDD::new(3, sum(5, 3));
        assert_eq!(
            sorted_tuples(&mdd.remove_support(&HashMap::new())),
            sorted_tuples(&mdd)
        );
    }

    #[test]
    fn remove_support_sum_triple_delete_var0() {
        // Forbid var0=1: 1 of 3 arcs at layer 0 dies → delete path.
        let mdd = MonotonicMDD::new(3, sum(5, 3)).remove_support(&forbidden(&[(0, &[1])]));
        assert_eq!(
            sorted_tuples(&mdd),
            vec![vec![2, 1, 2], vec![2, 2, 1], vec![3, 1, 1]]
        );
    }

    #[test]
    fn remove_support_sum_pair_delete_var0() {
        // Forbid var0=2 from sum(4,2) n=3: removes tuple (2,2).
        let mdd = MonotonicMDD::new(3, sum(4, 2)).remove_support(&forbidden(&[(0, &[2])]));
        assert_eq!(sorted_tuples(&mdd), vec![vec![1, 3], vec![3, 1]]);
    }

    #[test]
    fn remove_support_product_pair_delete_var0() {
        // Forbid var0=3 from product(6,2) n=4: removes (3,2), keeps (2,3).
        let mdd = MonotonicMDD::new(4, product(6, 2)).remove_support(&forbidden(&[(0, &[3])]));
        assert_eq!(sorted_tuples(&mdd), vec![vec![2, 3]]);
    }

    #[test]
    fn remove_support_sum_triple_reset_var1() {
        // Forbid var1=1 and var1=2: majority of layer-1 arcs die → reset path.
        let mdd = MonotonicMDD::new(3, sum(5, 3)).remove_support(&forbidden(&[(1, &[1, 2])]));
        assert_eq!(sorted_tuples(&mdd), vec![vec![1, 3, 1]]);
    }

    #[test]
    fn remove_support_sum_triple_two_layers() {
        // Forbid var0=1 and var2=1.
        let mdd =
            MonotonicMDD::new(3, sum(5, 3)).remove_support(&forbidden(&[(0, &[1]), (2, &[1])]));
        assert_eq!(sorted_tuples(&mdd), vec![vec![2, 1, 2]]);
    }

    #[test]
    fn remove_support_all_removed() {
        // Forbid the only surviving value at a layer: MDD becomes empty.
        let mdd = MonotonicMDD::new(3, sum(5, 3)).remove_support(&forbidden(&[(1, &[1, 2, 3])]));
        assert_eq!(mdd.tuples(), vec![] as Vec<Tuple>);
    }

    // ---- brute-force oracle cross-check ----

    #[test]
    fn sum_matches_brute_force_oracle() {
        for n in 3u32..=6 {
            for arity in 2u32..=4 {
                let max_target = n * arity + 1;
                for target in 1..=max_target {
                    assert_equiv(n, sum(target, arity));
                }
            }
        }
    }

    #[test]
    fn product_matches_brute_force_oracle() {
        for n in 3u32..=6 {
            for arity in 2u32..=3 {
                let max_target = n.pow(arity) + 1;
                for target in 1..=max_target {
                    assert_equiv(n, product(target, arity));
                }
            }
        }
    }

    #[test]
    #[ignore = "exhaustive property test; run with --include-ignored on merge to main"]
    fn matches_brute_force_across_n_arity_and_target() {
        for n in 3u32..=9 {
            for arity in 2u32..=5 {
                let max_sum = n * arity + 1;
                for target in 1..=max_sum {
                    assert_equiv(n, sum(target, arity));
                }
                let max_product = n.pow(arity) + 1;
                for target in 1..=max_product {
                    assert_equiv(n, product(target, arity));
                }
            }
        }
    }

    // ---- infeasibility ----

    #[test]
    fn sum_target_out_of_range_is_empty() {
        // Min sum is arity * 1, max is arity * n.
        assert_eq!(
            MonotonicMDD::new(3, sum(1, 3)).tuples(),
            vec![] as Vec<Tuple>
        );
        assert_eq!(
            MonotonicMDD::new(3, sum(10, 3)).tuples(),
            vec![] as Vec<Tuple>
        );
    }

    #[test]
    fn product_target_out_of_range_is_empty() {
        // No product of three values in 1..=3 equals 28.
        assert_eq!(
            MonotonicMDD::new(3, product(28, 3)).tuples(),
            vec![] as Vec<Tuple>
        );
    }

    // ---- reducedness ----

    #[test]
    fn constructed_mdd_is_reduced() {
        for (n, constraint) in [
            (4, sum(5, 2)),
            (6, sum(10, 3)),
            (9, sum(20, 4)),
            (4, product(6, 2)),
            (6, product(24, 3)),
        ] {
            assert_reduced(&MonotonicMDD::new(n, constraint));
        }
    }

    #[test]
    fn mdd_is_reduced_after_remove_support() {
        let mdd = MonotonicMDD::new(4, sum(6, 3));
        let pruned = mdd.remove_support(&forbidden(&[(0, &[1])]));
        assert_reduced(&pruned);
    }

    // ---- helpers and fixtures ----

    fn init_logging() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init();
    }

    fn sum(target: u32, arity: u32) -> MonotonicConstraint {
        MonotonicConstraint::Sum(Constraint { target, arity })
    }

    fn product(target: u32, arity: u32) -> MonotonicConstraint {
        MonotonicConstraint::Product(Constraint { target, arity })
    }

    fn forbidden(pairs: &[(u32, &[u32])]) -> HashMap<u32, HashSet<u32>> {
        pairs
            .iter()
            .map(|&(var, vals)| (var, vals.iter().copied().collect()))
            .collect()
    }

    fn sorted_tuples(mdd: &MonotonicMDD) -> Vec<Tuple> {
        let mut t = mdd.tuples();
        t.sort();
        t
    }

    /// Independent brute-force oracle: enumerate every `arity`-tuple over `1..=n`
    /// and keep those satisfying the constraint. Shares no code with `MonotonicMDD`.
    fn ref_tuples(n: u32, constraint: MonotonicConstraint) -> Vec<Tuple> {
        let arity = constraint.arity() as usize;
        let mut out = Vec::new();
        let mut t = vec![1u32; arity];
        loop {
            let acc = t
                .iter()
                .fold(constraint.unit(), |a, &v| constraint.operation(a, v));
            if acc == constraint.target() {
                out.push(t.iter().map(|&v| v as crate::Value).collect());
            }
            let mut i = 0;
            while i < arity && t[i] == n {
                t[i] = 1;
                i += 1;
            }
            if i == arity {
                break;
            }
            t[i] += 1;
        }
        out.sort();
        out
    }

    /// Assert that the MDD and the brute-force oracle produce the same sorted tuples.
    fn assert_equiv(n: u32, constraint: MonotonicConstraint) {
        let mdd = MonotonicMDD::new(n, constraint);
        let actual = sorted_tuples(&mdd);
        let expected = ref_tuples(n, constraint);
        assert_eq!(
            actual, expected,
            "mismatch for n={n}, constraint={constraint}"
        );
    }

    /// Assert that no two nodes in the MDD share the same `(depth, value)` key —
    /// the reducedness invariant guaranteed by hash-consing.
    fn assert_reduced(mdd: &MonotonicMDD) {
        let mut seen = HashSet::new();
        for node in mdd.edges.keys() {
            assert!(seen.insert(*node), "duplicate node {node} in MDD");
        }
    }
}
