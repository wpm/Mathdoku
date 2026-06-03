//! MDD-based (multivalued decision diagram) implementation of [`Memo`].
use crate::mdk::Error;
use crate::mdk::Error::InvalidCell;
use crate::mdk::N;
use crate::mdk::Target;
use crate::mdk::cage::Commutative;
use crate::mdk::fill::{Fill, Memo};
use crate::mdk::grid::{Cell, Polyomino};
use log::debug;
use std::collections::{HashMap, HashSet};

/// Monotonic cage-fill memo backed by an MDD.
///
/// Suitable for cages whose constraint has monotonic structure (e.g. addition, multiplication).
pub struct Mdd {
    cells: Vec<Cell>,
    inner: MonotonicMDD,
}

impl Mdd {
    /// Creates an MDD memo for `polyomino` with the monotonic `op` and `target` on a grid of size `n`.
    pub fn new(n: usize, polyomino: &Polyomino, op: Commutative, target: Target) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        let constraint = match op {
            Commutative::Add => MonotonicConstraint::Sum(Constraint {
                target,
                arity: polyomino.len() as N,
            }),
            Commutative::Multiply => MonotonicConstraint::Product(Constraint {
                target,
                arity: polyomino.len() as N,
            }),
        };
        #[allow(clippy::cast_possible_truncation)]
        let inner = MonotonicMDD::new(n as N, constraint);
        let cells = polyomino.iter().copied().collect();
        Self { cells, inner }
    }

    fn index(&self, cell: &Cell) -> Result<usize, Error> {
        self.cells
            .iter()
            .position(|c| c == cell)
            .ok_or(InvalidCell(*cell))
    }
}

impl Memo for Mdd {
    fn fill(&self, cell: &Cell) -> Result<Fill, Error> {
        let index = self.index(cell)?;
        let ns: Vec<N> = self.inner.tuples().iter().map(|t| t[index]).collect();
        Ok(Fill::from(&ns))
    }

    fn remove(&self, fills: HashMap<Cell, Fill>) -> Result<Self, Error> {
        let mut values: HashMap<N, HashSet<N>> = HashMap::new();
        for (cell, fill) in &fills {
            let index = self.index(cell)?;
            #[allow(clippy::cast_possible_truncation)]
            let depth = index as N;
            let forbidden: HashSet<N> = (1..=self.inner.n).filter(|v| !fill.contains(*v)).collect();
            drop(values.insert(depth, forbidden));
        }
        Ok(Self {
            cells: self.cells.clone(),
            inner: self.inner.remove_support(&values),
        })
    }
}

// ---------------------------------------------------------------------------
// MonotonicMDD — copied from mdd.rs (src/mdd.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Eq, PartialEq)]
struct MonotonicMDD {
    n: N,
    constraint: MonotonicConstraint,
    edges: HashMap<Node, Vec<(N, Node)>>,
}

impl MonotonicMDD {
    fn new(n: N, constraint: MonotonicConstraint) -> Self {
        let mut mdd = Self {
            n,
            constraint,
            edges: HashMap::new(),
        };
        let root = Node {
            depth: 0,
            value: constraint.unit(),
        };
        mdd.subtree(root);
        mdd
    }

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

    fn remove_support(&self, values: &HashMap<N, HashSet<N>>) -> Self {
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
        let surviving: HashSet<N> = (1..=self.n).filter(|v| !forbidden.contains(v)).collect();
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
                node.value == self.constraint.target() && node.depth == self.constraint.arity();
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
        Self::indented_debug(head.depth, &format!("{head} -{value}→ {tail}"));
        self.edges.entry(head).or_default().push((value, tail));
    }

    fn at_arity(&self, tail: Node) -> bool {
        let (d, a) = (u64::from(tail.depth), u64::from(self.constraint.arity()));
        assert!(d <= a, "depth {d} > arity {a}");
        let reached = d == a;
        if reached {
            Self::indented_debug(tail.depth, &format!("{tail} Arity limit met"));
        }
        reached
    }

    fn at_target(&self, node: Node) -> bool {
        let reached = self.constraint.target_reached(node.value);
        if reached {
            Self::indented_debug(node.depth, &format!("{node} Target reached"));
        }
        reached
    }

    fn indented_debug(depth: N, message: &str) {
        debug!("{:indent$}{message}", "", indent = depth as usize);
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
                if head.value == self.constraint.target() && head.depth == self.constraint.arity() {
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

impl std::fmt::Display for MonotonicMDD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MDD({} {} nodes)", self.constraint, self.edges.len())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct Constraint {
    target: N,
    arity: N,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum MonotonicConstraint {
    Sum(Constraint),
    Product(Constraint),
}

impl MonotonicConstraint {
    const fn arity(&self) -> N {
        match self {
            Self::Sum(c) | Self::Product(c) => c.arity,
        }
    }

    const fn target(&self) -> N {
        match self {
            Self::Sum(c) | Self::Product(c) => c.target,
        }
    }

    const fn target_reached(&self, v: N) -> bool {
        match self {
            Self::Sum(c) => v >= c.target,
            Self::Product(c) => v > c.target,
        }
    }

    const fn pruned(&self, acc: N, v: N, _remaining: N) -> bool {
        match self {
            Self::Sum(c) => acc + v > c.target,
            Self::Product(c) => acc * v > c.target,
        }
    }

    const fn skipped(&self, acc: N, v: N, remaining: N, n: N) -> bool {
        match self {
            Self::Sum(c) => acc + v + remaining * n < c.target,
            Self::Product(c) => (acc * v) != 0 && c.target % (acc * v) != 0,
        }
    }

    const fn operation(&self, x: N, y: N) -> N {
        match self {
            Self::Sum(_) => x + y,
            Self::Product(_) => x * y,
        }
    }

    const fn unit(&self) -> N {
        match self {
            Self::Sum(_) => 0,
            Self::Product(_) => 1,
        }
    }
}

impl std::fmt::Display for MonotonicConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (symbol, c) = match self {
            Self::Sum(c) => ('+', c),
            Self::Product(c) => ('×', c),
        };
        write!(f, "{symbol}{} [{}]", c.target, c.arity)
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
    use crate::mdk::fill::Memo;
    use crate::mdk::grid::Polyomino;

    // ---- Mdd::fill and Mdd::remove ----

    #[test]
    fn sum_pair_display() {
        assert_eq!(
            MonotonicMDD::new(3, sum_c(4, 2)).to_string(),
            "MDD(+4 [2] 4 nodes)"
        );
    }

    #[test]
    fn sum_triple_display() {
        assert_eq!(
            MonotonicMDD::new(3, sum_c(5, 3)).to_string(),
            "MDD(+5 [3] 7 nodes)"
        );
    }

    #[test]
    fn sum_triple_larger_n_display() {
        assert_eq!(
            MonotonicMDD::new(4, sum_c(6, 3)).to_string(),
            "MDD(+6 [3] 9 nodes)"
        );
    }

    #[test]
    fn product_pair_display() {
        assert_eq!(
            MonotonicMDD::new(4, product_c(6, 2)).to_string(),
            "MDD(×6 [2] 4 nodes)"
        );
    }

    #[test]
    fn product_triple_display() {
        assert_eq!(
            MonotonicMDD::new(4, product_c(4, 3)).to_string(),
            "MDD(×4 [3] 7 nodes)"
        );
    }

    #[test]
    fn sum_pair_tuples() {
        let m = mdd(3, &pair(1, 1, 1, 2), Commutative::Add, 4);
        assert_eq!(m.fill(&Cell::new(1, 1)).unwrap(), Fill::from(&[1, 2, 3]));
        assert_eq!(m.fill(&Cell::new(1, 2)).unwrap(), Fill::from(&[1, 2, 3]));
    }

    #[test]
    fn sum_triple_tuples() {
        let m = mdd(3, &triple(1, 1, 1, 2, 1, 3), Commutative::Add, 5);
        assert_eq!(m.fill(&Cell::new(1, 1)).unwrap(), Fill::from(&[1, 2, 3]));
        assert_eq!(m.fill(&Cell::new(1, 2)).unwrap(), Fill::from(&[1, 2, 3]));
        assert_eq!(m.fill(&Cell::new(1, 3)).unwrap(), Fill::from(&[1, 2, 3]));
    }

    #[test]
    fn sum_triple_larger_n_tuples() {
        let m = mdd(4, &triple(1, 1, 1, 2, 1, 3), Commutative::Add, 6);
        assert_eq!(m.fill(&Cell::new(1, 1)).unwrap(), Fill::from(&[1, 2, 3, 4]));
        assert_eq!(m.fill(&Cell::new(1, 2)).unwrap(), Fill::from(&[1, 2, 3, 4]));
        assert_eq!(m.fill(&Cell::new(1, 3)).unwrap(), Fill::from(&[1, 2, 3, 4]));
    }

    #[test]
    fn product_pair_tuples() {
        let m = mdd(4, &pair(1, 1, 1, 2), Commutative::Multiply, 6);
        assert_eq!(m.fill(&Cell::new(1, 1)).unwrap(), Fill::from(&[2, 3]));
        assert_eq!(m.fill(&Cell::new(1, 2)).unwrap(), Fill::from(&[2, 3]));
    }

    #[test]
    fn product_triple_tuples() {
        let m = mdd(4, &triple(1, 1, 1, 2, 1, 3), Commutative::Multiply, 4);
        assert_eq!(m.fill(&Cell::new(1, 1)).unwrap(), Fill::from(&[1, 2, 4]));
        assert_eq!(m.fill(&Cell::new(1, 2)).unwrap(), Fill::from(&[1, 2, 4]));
        assert_eq!(m.fill(&Cell::new(1, 3)).unwrap(), Fill::from(&[1, 2, 4]));
    }

    #[test]
    fn sum_arity() {
        assert_eq!(sum_c(10, 3).arity(), 3);
    }

    #[test]
    fn product_arity() {
        assert_eq!(product_c(6, 2).arity(), 2);
    }

    #[test]
    fn sum_operation() {
        assert_eq!(sum_c(7, 2).operation(3, 7), 10);
    }

    #[test]
    fn product_operation() {
        assert_eq!(product_c(4, 2).operation(3, 4), 12);
    }

    #[test]
    fn sum_display() {
        assert_eq!(sum_c(10, 3).to_string(), "+10 [3]");
    }

    #[test]
    fn product_display() {
        assert_eq!(product_c(6, 2).to_string(), "×6 [2]");
    }

    // ---- remove_support ----

    #[test]
    fn remove_support_empty_is_identity() {
        let mdd = MonotonicMDD::new(3, sum_c(5, 3));
        assert_eq!(
            sorted_tuples_raw(&mdd.remove_support(&HashMap::new())),
            sorted_tuples_raw(&mdd)
        );
    }

    #[test]
    fn remove_support_sum_triple_delete_var0() {
        let mdd = MonotonicMDD::new(3, sum_c(5, 3)).remove_support(&forbidden(&[(0, &[1])]));
        assert_eq!(
            sorted_tuples_raw(&mdd),
            vec![vec![2, 1, 2], vec![2, 2, 1], vec![3, 1, 1]]
        );
    }

    #[test]
    fn remove_support_sum_pair_delete_var0() {
        let mdd = MonotonicMDD::new(3, sum_c(4, 2)).remove_support(&forbidden(&[(0, &[2])]));
        assert_eq!(sorted_tuples_raw(&mdd), vec![vec![1, 3], vec![3, 1]]);
    }

    #[test]
    fn remove_support_product_pair_delete_var0() {
        let mdd = MonotonicMDD::new(4, product_c(6, 2)).remove_support(&forbidden(&[(0, &[3])]));
        assert_eq!(sorted_tuples_raw(&mdd), vec![vec![2, 3]]);
    }

    #[test]
    fn remove_support_sum_triple_reset_var1() {
        let mdd = MonotonicMDD::new(3, sum_c(5, 3)).remove_support(&forbidden(&[(1, &[1, 2])]));
        assert_eq!(sorted_tuples_raw(&mdd), vec![vec![1, 3, 1]]);
    }

    #[test]
    fn remove_support_sum_triple_two_layers() {
        let mdd =
            MonotonicMDD::new(3, sum_c(5, 3)).remove_support(&forbidden(&[(0, &[1]), (2, &[1])]));
        assert_eq!(sorted_tuples_raw(&mdd), vec![vec![2, 1, 2]]);
    }

    #[test]
    fn remove_support_all_removed() {
        let mdd = MonotonicMDD::new(3, sum_c(5, 3)).remove_support(&forbidden(&[(1, &[1, 2, 3])]));
        assert_eq!(sorted_tuples_raw(&mdd), vec![] as Vec<Vec<N>>);
    }

    // ---- Memo::fill ----

    #[test]
    fn fill_sum_pair_c0_all_values() {
        // sum(4,2) n=3: all values {1,2,3} appear in column 0
        let m = mdd(3, &pair(1, 1, 1, 2), Commutative::Add, 4);
        assert_eq!(m.fill(&Cell::new(1, 1)).unwrap(), Fill::from(&[1, 2, 3]));
    }

    #[test]
    fn fill_invalid_cell_returns_error() {
        let m = mdd(3, &pair(1, 1, 1, 2), Commutative::Add, 4);
        assert!(matches!(m.fill(&Cell::new(9, 9)), Err(InvalidCell(_))));
    }

    // ---- Memo::remove ----

    #[test]
    fn remove_prunes_fill() {
        let m = mdd(4, &triple(1, 1, 1, 2, 1, 3), Commutative::Add, 6);
        let pruned = m
            .remove(HashMap::from([(Cell::new(1, 1), Fill::from(&[2, 3, 4]))]))
            .unwrap();
        // var0 restricted to {2,3,4}: tuples with var0=1 removed
        assert_eq!(
            pruned.fill(&Cell::new(1, 1)).unwrap(),
            Fill::from(&[2, 3, 4])
        );
        assert_eq!(
            pruned.fill(&Cell::new(1, 2)).unwrap(),
            Fill::from(&[1, 2, 3])
        );
        assert_eq!(
            pruned.fill(&Cell::new(1, 3)).unwrap(),
            Fill::from(&[1, 2, 3])
        );
    }

    #[test]
    fn remove_invalid_cell_returns_error() {
        let m = mdd(3, &pair(1, 1, 1, 2), Commutative::Add, 4);
        assert!(matches!(
            m.remove(HashMap::from([(Cell::new(9, 9), Fill::from(&[1]))])),
            Err(InvalidCell(_))
        ));
    }

    // ---- brute-force oracle cross-check ----

    #[test]
    fn sum_matches_brute_force_oracle() {
        for n in 3u32..=6 {
            for arity in 2u32..=4 {
                let max_target = n * arity + 1;
                for target in 1..=max_target {
                    assert_equiv(n, sum_c(target, arity));
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
                    assert_equiv(n, product_c(target, arity));
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
                    assert_equiv(n, sum_c(target, arity));
                }
                let max_product = n.pow(arity) + 1;
                for target in 1..=max_product {
                    assert_equiv(n, product_c(target, arity));
                }
            }
        }
    }

    // ---- infeasibility ----

    #[test]
    fn sum_target_out_of_range_is_empty() {
        // min sum for arity=3, n=3 is 3; max is 9
        let low = mdd(3, &triple(1, 1, 1, 2, 1, 3), Commutative::Add, 1);
        assert_eq!(low.fill(&Cell::new(1, 1)).unwrap(), Fill::from(&[]));
        let high = mdd(3, &triple(1, 1, 1, 2, 1, 3), Commutative::Add, 10);
        assert_eq!(high.fill(&Cell::new(1, 1)).unwrap(), Fill::from(&[]));
    }

    #[test]
    fn product_target_out_of_range_is_empty() {
        // no product of three values in 1..=3 equals 28
        let m = mdd(3, &triple(1, 1, 1, 2, 1, 3), Commutative::Multiply, 28);
        assert_eq!(m.fill(&Cell::new(1, 1)).unwrap(), Fill::from(&[]));
    }

    // ---- reducedness ----

    #[test]
    fn constructed_mdd_is_reduced() {
        for (n, constraint) in [
            (4, sum_c(5, 2)),
            (6, sum_c(10, 3)),
            (9, sum_c(20, 4)),
            (4, product_c(6, 2)),
            (6, product_c(24, 3)),
        ] {
            assert_reduced(&MonotonicMDD::new(n, constraint));
        }
    }

    #[test]
    fn mdd_is_reduced_after_remove_support() {
        let mdd = MonotonicMDD::new(4, sum_c(6, 3));
        let pruned = mdd.remove_support(&forbidden(&[(0, &[1])]));
        assert_reduced(&pruned);
    }

    // ---- helpers and fixtures ----

    fn sum_c(target: N, arity: N) -> MonotonicConstraint {
        MonotonicConstraint::Sum(Constraint { target, arity })
    }

    fn product_c(target: N, arity: N) -> MonotonicConstraint {
        MonotonicConstraint::Product(Constraint { target, arity })
    }

    fn pair(r0: usize, c0: usize, r1: usize, c1: usize) -> Polyomino {
        Polyomino::from_cells([Cell::new(r0, c0), Cell::new(r1, c1)])
    }

    fn triple(r0: usize, c0: usize, r1: usize, c1: usize, r2: usize, c2: usize) -> Polyomino {
        Polyomino::from_cells([Cell::new(r0, c0), Cell::new(r1, c1), Cell::new(r2, c2)])
    }

    fn mdd(n: usize, polyomino: &Polyomino, op: Commutative, target: Target) -> Mdd {
        Mdd::new(n, polyomino, op, target)
    }

    fn forbidden(pairs: &[(N, &[N])]) -> HashMap<N, HashSet<N>> {
        pairs
            .iter()
            .map(|&(var, vals)| (var, vals.iter().copied().collect()))
            .collect()
    }

    fn sorted_tuples_raw(mdd: &MonotonicMDD) -> Vec<Vec<N>> {
        let mut t = mdd.tuples();
        t.sort();
        t
    }

    fn ref_tuples(n: N, constraint: MonotonicConstraint) -> Vec<Vec<N>> {
        let arity = constraint.arity() as usize;
        let mut out = Vec::new();
        let mut t = vec![1u32; arity];
        loop {
            let acc = t
                .iter()
                .fold(constraint.unit(), |a, &v| constraint.operation(a, v));
            if acc == constraint.target() {
                out.push(t.clone());
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

    fn assert_equiv(n: N, constraint: MonotonicConstraint) {
        let mdd = MonotonicMDD::new(n, constraint);
        let mut actual = mdd.tuples();
        actual.sort();
        let expected = ref_tuples(n, constraint);
        assert_eq!(
            actual, expected,
            "mismatch for n={n}, constraint={constraint}"
        );
    }

    fn assert_reduced(mdd: &MonotonicMDD) {
        let mut seen = std::collections::HashSet::new();
        for node in mdd.edges.keys() {
            assert!(seen.insert(*node), "duplicate node {node} in MDD");
        }
    }
}
