//! Multivalued Decision Diagram (MDD) implementation of [`Memo`] and `Narrow`.
//!
//! Only commutative (add, multiply) constraints are supported. For non-commutative
//! constraints (subtract, divide), use `Table` instead.
use crate::mdk::Error::InvalidCellCageIndex;
use crate::mdk::fill::Fill;
use crate::mdk::memo::{Memo, fills_from_tuples};
use crate::mdk::operation::CommutativeOperator;
use crate::mdk::{Error, N, Target};
use log::debug;
use std::collections::{HashMap, HashSet};

/// A cage constraint stored as a multivalued decision diagram.
///
/// Nodes are keyed by `(depth, accumulated_value)`. Edges are labelled with the
/// cell value chosen at that depth. Valid tuples correspond to paths from the
/// root to a terminal node where `value == target` and `depth == arity`.
///
/// Per-position candidate sets ([`Fill`]s) are derived from the surviving paths
/// and cached; construction fails with [`EmptyFills`] if no valid tuples exist.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Mdd {
    n: usize,
    constraint: Constraint,
    edges: HashMap<Node, Vec<(N, Node)>>,
    fills: Vec<Fill>,
}

impl Mdd {
    /// Constructs an MDD for all `k`-tuples of values in `1..=n` satisfying
    /// `operator` applied to the tuple equals `target`.
    ///
    /// # Errors
    /// Returns [`Error::EmptyFills`] if no tuples satisfy the constraint.
    pub fn new(
        n: usize,
        k: usize,
        operator: CommutativeOperator,
        target: Target,
    ) -> Result<Self, Error> {
        #[allow(clippy::cast_possible_truncation)]
        let constraint = Constraint {
            operator,
            target,
            arity: k as N,
        };
        let mut mdd = Self {
            n,
            constraint,
            edges: HashMap::new(),
            fills: Vec::new(),
        };
        let root = Node {
            depth: 0,
            value: constraint.unit(),
        };
        mdd.subtree(root);
        mdd.fills = fills_from_tuples(n, &mdd.tuples())?;
        Ok(mdd)
    }

    /// Recursively builds the MDD rooted at `head`, adding edges for all values
    /// that are not pruned by the constraint's monotonicity bounds.
    fn subtree(&mut self, head: Node) {
        if self.edges.contains_key(&head) {
            return;
        }
        debug!("{self}");
        let remaining = self.constraint.arity - head.depth - 1;
        #[allow(clippy::cast_possible_truncation)]
        for i in 1..=self.n as N {
            if self.constraint.pruned(head.value, i, remaining) {
                break;
            }
            if self
                .constraint
                .skipped(head.value, i, remaining, self.n as N)
            {
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

    /// Returns a copy of this MDD with edges for forbidden values removed and
    /// dead nodes garbage-collected via downward and upward cascades.
    fn remove_support(&self, forbidden: &HashMap<N, HashSet<N>>) -> Self {
        let mut mdd = Self {
            n: self.n,
            constraint: self.constraint,
            edges: self.edges.clone(),
            fills: Vec::new(),
        };
        let mut q_down: Vec<Node> = Vec::new(); // nodes that may have lost all incoming edges
        let mut q_up: Vec<Node> = Vec::new(); // nodes that may have lost all outgoing edges

        for (&depth, forbidden) in forbidden {
            let heads = mdd.heads_at_depth(depth);
            let (total_arcs, dead_arcs) = heads
                .iter()
                .filter_map(|h| mdd.edges.get(h))
                .flat_map(|es| es.iter())
                .fold((0, 0), |(total, dead), (label, _)| {
                    (total + 1, dead + usize::from(forbidden.contains(label)))
                });

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

    fn heads_at_depth(&self, depth: N) -> Vec<Node> {
        self.edges
            .keys()
            .filter(|n| n.depth == depth)
            .copied()
            .collect()
    }

    fn tails_of(edges: &HashMap<Node, Vec<(N, Node)>>, heads: &[Node]) -> HashSet<Node> {
        heads
            .iter()
            .filter_map(|h| edges.get(h))
            .flat_map(|es| es.iter())
            .map(|(_, t)| *t)
            .collect()
    }

    fn reset_layer(
        &mut self,
        heads: &[Node],
        forbidden: &HashSet<N>,
        q_down: &mut Vec<Node>,
        q_up: &mut Vec<Node>,
    ) {
        #[allow(clippy::cast_possible_truncation)]
        let surviving: HashSet<N> = (1..=self.n as N)
            .filter(|v| !forbidden.contains(v))
            .collect();
        let tails_before = Self::tails_of(&self.edges, heads);

        let orig: Vec<(Node, Vec<(N, Node)>)> = heads
            .iter()
            .filter_map(|h| self.edges.remove(h).map(|es| (*h, es)))
            .collect();
        for (head, orig_edges) in orig {
            let new_edges: Vec<(N, Node)> = orig_edges
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

    fn delete_layer(
        &mut self,
        heads: &[Node],
        forbidden: &HashSet<N>,
        q_down: &mut Vec<Node>,
        q_up: &mut Vec<Node>,
    ) {
        for head in heads {
            if let Some(es) = self.edges.get_mut(head) {
                let dead_tails: Vec<Node> = es
                    .iter()
                    .filter(|(label, _)| forbidden.contains(label))
                    .map(|(_, t)| *t)
                    .collect(); // collect before retain to avoid borrow conflict
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

    fn cascade_down(&mut self, q: &mut Vec<Node>) {
        while let Some(node) = q.pop() {
            if !self.edges.contains_key(&node) {
                continue;
            }
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

    fn cascade_up(&mut self, q: &mut Vec<Node>) {
        while let Some(node) = q.pop() {
            if self.edges.contains_key(&node) {
                continue;
            }
            let is_terminal =
                node.value == self.constraint.target && node.depth == self.constraint.arity;
            if !is_terminal {
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

    fn insert_edge(&mut self, head: Node, value: N, tail: Node) {
        debug!(
            "{:indent$}{head} -{value}→ {tail}",
            "",
            indent = head.depth as usize
        );
        self.edges.entry(head).or_default().push((value, tail));
    }

    fn at_arity(&self, tail: Node) -> bool {
        let (d, a) = (u64::from(tail.depth), u64::from(self.constraint.arity));
        debug_assert!(d <= a, "depth {d} > arity {a}");
        Self::log_if(d == a, tail.depth, &format!("{tail} Arity limit met"))
    }

    fn at_target(&self, node: Node) -> bool {
        Self::log_if(
            self.constraint.target_reached(node.value),
            node.depth,
            &format!("{node} Target reached"),
        )
    }

    fn log_if(condition: bool, depth: N, message: &str) -> bool {
        if condition {
            debug!("{:indent$}{message}", "", indent = depth as usize);
        }
        condition
    }

    fn tuples(&self) -> Vec<Vec<N>> {
        let root = Node {
            depth: 0,
            value: self.constraint.unit(),
        };
        let mut result = Vec::new();
        self.collect_paths(root, &mut Vec::new(), &mut result);
        result
    }

    fn collect_paths(&self, head: Node, path: &mut Vec<N>, result: &mut Vec<Vec<N>>) {
        match self.edges.get(&head) {
            None => {
                if head.value == self.constraint.target && head.depth == self.constraint.arity {
                    result.push(path.clone());
                }
            }
            Some(edges) => {
                for &(label, tail) in edges {
                    path.push(label);
                    self.collect_paths(tail, path, result);
                    let _ = path.pop();
                }
            }
        }
    }
}

impl std::fmt::Display for Mdd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MDD({} {} nodes)", self.constraint, self.edges.len())
    }
}

impl Memo for Mdd {
    fn get(&self, index: usize) -> Result<Fill, Error> {
        self.fills
            .get(index)
            .cloned()
            .ok_or(InvalidCellCageIndex(index))
    }

    fn reset(&self) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        Self::new(
            self.n,
            self.constraint.arity as usize,
            self.constraint.operator,
            self.constraint.target,
        )
        .unwrap_or_else(|_| unreachable!("reset reconstructs a constraint that was already valid"))
    }

    fn narrow(&self, support: Vec<Fill>) -> Result<Self, Error> {
        #[allow(clippy::cast_possible_truncation)]
        let forbidden: HashMap<N, HashSet<N>> = support
            .iter()
            .enumerate()
            .filter_map(|(i, fill)| {
                let excluded: HashSet<N> =
                    (1..=self.n as N).filter(|v| !fill.contains(*v)).collect();
                if excluded.is_empty() {
                    None
                } else {
                    Some((i as N, excluded))
                }
            })
            .collect();
        let mut narrowed = self.remove_support(&forbidden);
        narrowed.fills = fills_from_tuples(self.n, &narrowed.tuples())?;
        Ok(narrowed)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct Constraint {
    operator: CommutativeOperator,
    target: N,
    arity: N,
}

impl Constraint {
    const fn target_reached(self, v: N) -> bool {
        match self.operator {
            CommutativeOperator::Add => v >= self.target,
            CommutativeOperator::Multiply => v > self.target,
        }
    }

    const fn pruned(self, acc: N, v: N, _remaining: N) -> bool {
        match self.operator {
            CommutativeOperator::Add => acc + v > self.target,
            CommutativeOperator::Multiply => acc * v > self.target,
        }
    }

    const fn skipped(self, acc: N, v: N, remaining: N, n: N) -> bool {
        match self.operator {
            CommutativeOperator::Add => acc + v + remaining * n < self.target,
            CommutativeOperator::Multiply => (acc * v) != 0 && !self.target.is_multiple_of(acc * v),
        }
    }

    const fn operation(self, x: N, y: N) -> N {
        self.operator.apply_pair(x, y)
    }

    const fn unit(self) -> N {
        self.operator.identity()
    }
}

impl std::fmt::Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let symbol = match self.operator {
            CommutativeOperator::Add => '+',
            CommutativeOperator::Multiply => '×',
        };
        write!(f, "{symbol}{} [{}]", self.target, self.arity)
    }
}

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
struct Node {
    depth: N,
    value: N,
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node({} @ level {})", self.value, self.depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mdk::Error::EmptyFills;
    use crate::mdk::operation::CommutativeOperator::{Add, Multiply};

    // ---- get ----

    #[test]
    fn add_fills_are_union_of_column_values() {
        let m = Mdd::new(4, 2, Add, 6).unwrap();
        assert_eq!(m.get(0).unwrap(), Fill::from(4, &[2, 3, 4]));
        assert_eq!(m.get(1).unwrap(), Fill::from(4, &[2, 3, 4]));
    }

    #[test]
    fn multiply_fills_contain_expected_values() {
        let m = Mdd::new(6, 2, Multiply, 6).unwrap();
        assert_eq!(m.get(0).unwrap(), Fill::from(6, &[1, 2, 3, 6]));
        assert_eq!(m.get(1).unwrap(), Fill::from(6, &[1, 2, 3, 6]));
    }

    #[test]
    fn commutative_no_solutions_returns_empty_fills_error() {
        assert!(matches!(Mdd::new(4, 2, Add, 9), Err(EmptyFills)));
    }

    #[test]
    fn fill_out_of_bounds_returns_index_error() {
        let m = Mdd::new(4, 2, Add, 5).unwrap();
        assert!(matches!(m.get(2), Err(InvalidCellCageIndex(2))));
    }

    // ---- narrow ----

    #[test]
    fn narrow_with_full_support_is_identity() {
        let m = Mdd::new(4, 2, Add, 5).unwrap();
        assert_eq!(m.narrow(vec![Fill::new(4), Fill::new(4)]).unwrap(), m);
    }

    #[test]
    fn narrow_filters_tuples_and_updates_fills() {
        // add to 5 in n=4: (1,4),(2,3),(3,2),(4,1)
        // restrict pos 0 to {1,2} → surviving: (1,4),(2,3)
        let m = Mdd::new(4, 2, Add, 5).unwrap();
        let narrowed = m
            .narrow(vec![Fill::from(4, &[1, 2]), Fill::from(4, &[1, 2, 3, 4])])
            .unwrap();
        assert_eq!(narrowed.get(0).unwrap(), Fill::from(4, &[1, 2]));
        assert_eq!(narrowed.get(1).unwrap(), Fill::from(4, &[3, 4]));
    }

    #[test]
    fn narrow_eliminating_all_tuples_returns_empty_fills_error() {
        let m = Mdd::new(4, 2, Add, 5).unwrap();
        assert!(matches!(
            m.narrow(vec![Fill::from(4, &[1]), Fill::from(4, &[1])]),
            Err(EmptyFills)
        ));
    }

    // ---- reset ----

    #[test]
    fn reset_equals_fresh_construction() {
        let m = Mdd::new(4, 2, Add, 5).unwrap();
        let narrowed = m
            .narrow(vec![Fill::from(4, &[1, 2]), Fill::from(4, &[1, 2, 3, 4])])
            .unwrap();
        assert_eq!(narrowed.reset(), m);
    }

    // ---- set ----

    #[test]
    fn set_restricts_to_compliment_of_assigned_fills() {
        // add to 5 in n=4: tuples (1,4),(2,3),(3,2),(4,1)
        // assign pos 0 = {1,2} → compliment = {3,4}
        // assign pos 1 = {3,4} → compliment = {1,2}
        // surviving tuples where pos-0 ∈ {3,4} and pos-1 ∈ {1,2}: (3,2),(4,1)
        let m = Mdd::new(4, 2, Add, 5).unwrap();
        let result = m
            .set(vec![Fill::from(4, &[1, 2]), Fill::from(4, &[3, 4])])
            .unwrap();
        assert_eq!(result.get(0).unwrap(), Fill::from(4, &[3, 4]));
        assert_eq!(result.get(1).unwrap(), Fill::from(4, &[1, 2]));
    }

    #[test]
    fn set_eliminating_all_tuples_returns_empty_fills_error() {
        // assign both positions the full fill → compliment = {} → no tuples survive
        let m = Mdd::new(4, 2, Add, 5).unwrap();
        assert!(matches!(
            m.set(vec![Fill::new(4), Fill::new(4)]),
            Err(EmptyFills)
        ));
    }

    // ---- display ----

    #[test]
    fn sum_pair_display() {
        assert_eq!(
            Mdd::new(3, 2, Add, 4).unwrap().to_string(),
            "MDD(+4 [2] 4 nodes)"
        );
    }

    #[test]
    fn sum_triple_display() {
        assert_eq!(
            Mdd::new(3, 3, Add, 5).unwrap().to_string(),
            "MDD(+5 [3] 7 nodes)"
        );
    }

    #[test]
    fn sum_triple_larger_n_display() {
        assert_eq!(
            Mdd::new(4, 3, Add, 6).unwrap().to_string(),
            "MDD(+6 [3] 9 nodes)"
        );
    }

    #[test]
    fn product_pair_display() {
        assert_eq!(
            Mdd::new(4, 2, Multiply, 6).unwrap().to_string(),
            "MDD(×6 [2] 4 nodes)"
        );
    }

    #[test]
    fn product_triple_display() {
        assert_eq!(
            Mdd::new(4, 3, Multiply, 4).unwrap().to_string(),
            "MDD(×4 [3] 7 nodes)"
        );
    }

    // ---- fill values ----

    #[test]
    fn sum_pair_fills() {
        let m = Mdd::new(3, 2, Add, 4).unwrap();
        assert_eq!(m.get(0).unwrap(), Fill::from(3, &[1, 2, 3]));
        assert_eq!(m.get(1).unwrap(), Fill::from(3, &[1, 2, 3]));
    }

    #[test]
    fn sum_triple_fills() {
        let m = Mdd::new(3, 3, Add, 5).unwrap();
        assert_eq!(m.get(0).unwrap(), Fill::from(3, &[1, 2, 3]));
        assert_eq!(m.get(1).unwrap(), Fill::from(3, &[1, 2, 3]));
        assert_eq!(m.get(2).unwrap(), Fill::from(3, &[1, 2, 3]));
    }

    #[test]
    fn product_pair_fills() {
        let m = Mdd::new(4, 2, Multiply, 6).unwrap();
        assert_eq!(m.get(0).unwrap(), Fill::from(4, &[2, 3]));
        assert_eq!(m.get(1).unwrap(), Fill::from(4, &[2, 3]));
    }

    #[test]
    fn product_triple_fills() {
        let m = Mdd::new(4, 3, Multiply, 4).unwrap();
        assert_eq!(m.get(0).unwrap(), Fill::from(4, &[1, 2, 4]));
        assert_eq!(m.get(1).unwrap(), Fill::from(4, &[1, 2, 4]));
        assert_eq!(m.get(2).unwrap(), Fill::from(4, &[1, 2, 4]));
    }

    // ---- infeasibility ----

    #[test]
    fn sum_target_out_of_range_is_empty_fills() {
        assert!(matches!(Mdd::new(3, 3, Add, 1), Err(EmptyFills)));
        assert!(matches!(Mdd::new(3, 3, Add, 10), Err(EmptyFills)));
    }

    #[test]
    fn product_target_out_of_range_is_empty_fills() {
        assert!(matches!(Mdd::new(3, 3, Multiply, 28), Err(EmptyFills)));
    }

    // ---- remove_support ----

    #[test]
    fn remove_support_empty_is_identity() {
        let m = Mdd::new(3, 3, Add, 5).unwrap();
        assert_eq!(
            sorted_tuples(&m.remove_support(&HashMap::new())),
            sorted_tuples(&m)
        );
    }

    #[test]
    fn remove_support_sum_triple_delete_var0() {
        let m = Mdd::new(3, 3, Add, 5)
            .unwrap()
            .remove_support(&forbidden(&[(0, &[1])]));
        assert_eq!(
            sorted_tuples(&m),
            vec![vec![2, 1, 2], vec![2, 2, 1], vec![3, 1, 1]]
        );
    }

    #[test]
    fn remove_support_sum_pair_delete_var0() {
        let m = Mdd::new(3, 2, Add, 4)
            .unwrap()
            .remove_support(&forbidden(&[(0, &[2])]));
        assert_eq!(sorted_tuples(&m), vec![vec![1, 3], vec![3, 1]]);
    }

    #[test]
    fn remove_support_product_pair_delete_var0() {
        let m = Mdd::new(4, 2, Multiply, 6)
            .unwrap()
            .remove_support(&forbidden(&[(0, &[3])]));
        assert_eq!(sorted_tuples(&m), vec![vec![2, 3]]);
    }

    #[test]
    fn remove_support_sum_triple_reset_var1() {
        let m = Mdd::new(3, 3, Add, 5)
            .unwrap()
            .remove_support(&forbidden(&[(1, &[1, 2])]));
        assert_eq!(sorted_tuples(&m), vec![vec![1, 3, 1]]);
    }

    #[test]
    fn remove_support_sum_triple_two_layers() {
        let m = Mdd::new(3, 3, Add, 5)
            .unwrap()
            .remove_support(&forbidden(&[(0, &[1]), (2, &[1])]));
        assert_eq!(sorted_tuples(&m), vec![vec![2, 1, 2]]);
    }

    #[test]
    fn remove_support_all_removed() {
        let m = Mdd::new(3, 3, Add, 5)
            .unwrap()
            .remove_support(&forbidden(&[(1, &[1, 2, 3])]));
        assert_eq!(sorted_tuples(&m), vec![] as Vec<Vec<N>>);
    }

    // ---- reducedness ----

    #[test]
    fn constructed_mdd_is_reduced() {
        let cases = [
            (4usize, Add, 5u32, 2usize),
            (6, Add, 10, 3),
            (9, Add, 20, 4),
            (4, Multiply, 6, 2),
            (6, Multiply, 24, 3),
        ];
        for (n, op, target, k) in cases {
            assert_reduced(&Mdd::new(n, k, op, target).unwrap());
        }
    }

    #[test]
    fn mdd_is_reduced_after_remove_support() {
        let m = Mdd::new(4, 3, Add, 6).unwrap();
        let pruned = m.remove_support(&forbidden(&[(0, &[1])]));
        assert_reduced(&pruned);
    }

    // ---- brute-force oracle cross-check ----

    #[test]
    #[ignore = "exhaustive property test; run with --include-ignored on merge to main"]
    fn matches_brute_force_across_n_arity_and_target() {
        for n in 3u32..=9 {
            for k in 2u32..=5 {
                let max_sum = n * k + 1;
                for target in 1..=max_sum {
                    assert_equiv(n, Add, target, k);
                }
                let max_product = n.pow(k) + 1;
                for target in 1..=max_product {
                    assert_equiv(n, Multiply, target, k);
                }
            }
        }
    }

    // ---- helpers ----

    fn forbidden(pairs: &[(N, &[N])]) -> HashMap<N, HashSet<N>> {
        pairs
            .iter()
            .map(|&(var, vals)| (var, vals.iter().copied().collect()))
            .collect()
    }

    fn sorted_tuples(m: &Mdd) -> Vec<Vec<N>> {
        let mut t = m.tuples();
        t.sort();
        t
    }

    fn ref_tuples(n: N, op: CommutativeOperator, target: N, k: u32) -> Vec<Vec<N>> {
        let k = k as usize;
        let mut out = Vec::new();
        let mut t = vec![1u32; k];
        loop {
            if op.apply(&t) == target {
                out.push(t.clone());
            }
            let mut i = 0;
            while i < k && t[i] == n {
                t[i] = 1;
                i += 1;
            }
            if i == k {
                break;
            }
            t[i] += 1;
        }
        out.sort();
        out
    }

    fn assert_equiv(n: N, op: CommutativeOperator, target: N, k: u32) {
        let expected = ref_tuples(n, op, target, k);
        match Mdd::new(n as usize, k as usize, op, target) {
            Ok(m) => {
                let mut actual = m.tuples();
                actual.sort();
                assert_eq!(
                    actual, expected,
                    "mismatch for n={n}, op={op:?}, target={target}, k={k}"
                );
            }
            Err(EmptyFills) => {
                assert!(
                    expected.is_empty(),
                    "Mdd returned EmptyFills but expected {expected:?} for n={n}, op={op:?}, target={target}, k={k}"
                );
            }
            Err(e) => panic!("unexpected error {e:?}"),
        }
    }

    fn assert_reduced(m: &Mdd) {
        let mut seen = HashSet::new();
        for node in m.edges.keys() {
            assert!(seen.insert(*node), "duplicate node {node} in MDD");
        }
    }
}
