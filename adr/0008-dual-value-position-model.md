# ADR-0008: Dual value-position viewpoints for cage confinement

**Status:** Proposed
**Date:** 2026-06-13
**Deciders:** Bill McNeill (Mathdoku owner)

## Context

The solver keeps, for every cell, the set of values that cell can still take, and propagates two constraint families to a GAC fixpoint in `Puzzle::fixpoint`: each cage's arithmetic relation (a memo — ADR-0005, ADR-0006) and full-row / full-column all-different (Régin, `regin.rs`). A class of deductions a human makes by inspection is invisible to this model.

The motivating case is **confinement**. A ×42 triomino lying in one row has exactly two viable value-sets, $\{1,6,7\}$ and $\{2,3,7\}$. Both contain a 7, so whichever fill is chosen the cage consumes the row's single 7 — and every other cell in that row can drop 7. The deduction is not special to primes: it fires whenever *every* viable fill of a cage shares a value. A ×105 quadromino whose only viable fill is $\{1,3,5,7\}$ forces all four of those values into itself; a two-cell ×42 forces both 6 and 7. Products with a prime factor above $n/2$ (here 5 and 7) are merely the cheapest place to spot it, because such a prime's only multiple in $\{1,\dots,n\}$ is the prime itself.

The cell viewpoint cannot make this elimination by propagation, at any number of fixpoint iterations. The cage relation and the row all-different share only the cage's own cells. "A 7 occurs somewhere among these three cells" removes 7 from none of them individually — each cell still has a non-7 option — so no domain shrinks and the fact never crosses to the rest of the row. Recovering it requires constructive disjunction: tentatively commit each viable fill, propagate the whole row, and intersect the survivors. That is a search ply wrapped around the fixpoint, not a first-level constraint.

The gap bites hardest on **partial** puzzles, where grid all-different sees too few cells to compensate — interactive authoring in Designer and the partial boards walked while generating a unique-solution puzzle (ADR-0001) — and it is exactly the move a constructor relies on by hand.

Several forces shape the response.

**The fact lives on a value's position, not on a cell.** "The row's 7 sits inside this cage" is a statement about where the value 7 goes. The cell viewpoint owns no variable that can hold it, which is the whole reason a domain-only propagator cannot express it.

**The core fixpoint should stay GAC, not gain a search loop.** Decomposed per-constraint GAC is the right strength for the engine; the deduction should arrive as an ordinary domain reduction driven by the same worklist, not by a shave loop bolted around `fixpoint`.

**The viable fills are already computed.** The cage memo (ADR-0006) holds the conjunction of arithmetic and internal collinear distinctness; its projection enumerates precisely the viable value-sets this deduction consumes. No new enumeration is wanted.

**Régin must survive.** The all-different matching code is load-bearing and correct; the change must reuse it on whatever variables carry the line constraints, not replace it.

A secondary force is a preference for a mechanism with a known literature over a novel one.

## Model

### The board as a 0/1 cube

Index a solved $n \times n$ board by the cube $x[r,c,v] \in \{0,1\}$, with $x[r,c,v] = 1$ meaning "cell $(r,c)$ holds value $v$", over $r,c,v \in \{1,\dots,n\}$. The Latin-square conditions are three "exactly one" families, one per axis:

- **cell:** $\forall\,r,c:\ \sum_{v} x[r,c,v] = 1$ — each cell holds one value
- **row-line:** $\forall\,r,v:\ \sum_{c} x[r,c,v] = 1$ — each value once per row
- **col-line:** $\forall\,c,v:\ \sum_{r} x[r,c,v] = 1$ — each value once per column

### Three viewpoints

Each "exactly one" lets one axis be read off as a total function, giving three equivalent re-indexings of the same cube:

- **cell** $D[r,c] = v$ — the primal model; a cell ranges over values.
- **P** $P[r,v] = c$ — "which column does value $v$ take in row $r$"; ranges over columns.
- **Q** $Q[c,v] = r$ — "which row does value $v$ take in column $c$"; ranges over rows.

$P$ and $Q$ are the two value-indexed slices; $D$ is the position-indexed slice. All three are the identical cube, sliced on different axes.

### Information content

The three carry the same information once variables are set-valued, and the reason is worth stating, because it is easy to talk oneself out of. A bare function $[n] \to [n]$ carries $n \log_2 n$ bits; a relation $[n] \times [n] \to \{0,1\}$ — one cube slice — carries $n^2$ bits, and the two coincide only when the relation is constrained to be functional (the "exactly one" families). A CSP variable is never the bare function: it is the relation, because its value is a *domain*, a set. So $P[r,v]$ has type $[n] \to [n] \to \{[n]\}$, and $\{[n]\} \cong ([n] \to \{0,1\})$ restores the third axis — $P$ as solver state *is* the cube, transposed. The viewpoints differ only in which facts are cheap to express, not in what is representable. The bare solved functions differ; the search-state relations do not.

### Channelling and the Latin core

Maintaining more than one viewpoint creates an obligation that they agree. Generic $P$ and generic $Q$ describe unrelated boards; the **channelling** constraint pins them to one board:

$$P[r,v] = c \iff Q[c,v] = r \qquad (\text{both} \equiv x[r,c,v] = 1)$$

Channelling does more than synchronize. With $P$ and $Q$ total functions, it *entails* the two value-indexed line families:

> **Lemma.** If $P, Q$ are total and the channel holds, then for each value $v$ the map $P[\cdot,v] : r \mapsto c$ is a bijection $[n] \to [n]$, with $Q[\cdot,v]$ its inverse.
>
> *Proof.* If $P[r_1,v] = P[r_2,v] = c$, the channel gives $Q[c,v] = r_1$ and $Q[c,v] = r_2$; $Q$ is a function, so $r_1 = r_2$. Thus $P[\cdot,v]$ is injective, and injective + total on a finite set is bijective; the channel makes $Q[\cdot,v]$ its inverse. $\blacksquare$

So each value occupies a full permutation matrix — once per row and once per column — for free; the row-line and col-line families need not be posted. What channelling does **not** give is cell-uniqueness, because it only ever relates $P$ and $Q$ at a fixed $v$. Two values can collide: put both 1 and 2 on the main diagonal — $P$ and $Q$ are perfectly channel-consistent, yet the diagonal cells are doubly occupied and the off-diagonal cells are empty. That configuration is a consistent $(P, Q)$ pair that is not a Latin square.

The single missing family is cell-uniqueness — no two values share a cell — posted once as $\mathrm{AllDifferent}(P[r,\cdot])$ over values for each row (equivalently $\mathrm{AllDifferent}(Q[c,\cdot])$ per column). Channelling plus that one all-different family is a complete Latin square: each value a permutation matrix, the matrices pairwise disjoint, hence a partition of the grid. This is the minimal core; running all-different on additional slices is redundant for correctness but strengthens and speeds propagation.

### Cages as codomain restrictions

The Latin core fixes structure; a cage is puzzle content, and it enters only as a restriction on *where values may land* — the output side of $P$ and $Q$. A cage's content, after compiling arithmetic together with internal collinear distinctness (ADR-0006), is a set of viable value-sets $\{M_1, \dots, M_t\}$ (its memo's projection).

For an **axis-aligned** cage the restriction is a clean disjunction over one slice. A horizontal cage occupying columns $S$ of row $r$ (with $|S| = k$, each $M_i$ a $k$-subset of values):

$$\bigvee_{i}\left(\bigwedge_{v \in M_i} P[r,v] \in S\right)$$

The complement "values not in $M_i$ lie outside $S$" is entailed by $|S| = k$ and the per-row all-different, so it need not be written. A vertical cage is the same disjunction over $Q$ with a row-set $T$. A **blocky** cage (an L or fat block) aligns to neither slice: it ties together $P$-outputs and $Q$-outputs across several rows and columns at once, and stays a constraint over the cube cells with no single-slice reduction.

### Confinement

Let a cage's **universal values** be $U = \bigcap_i M_i$ — the values present in every viable fill ($\{7\}$ for the ×42 triomino; $\{1,3,5,7\}$ for the ×105 quadromino). For $v \in U$, every disjunct asserts $P[r,v] \in S$, so the disjunction entails, unconditionally,

$$P[r,v] \subseteq S.$$

Because $P[r,v]$ is a single variable shared — through the channel and the row all-different — with $Q$ and the cell view, **plain GAC on the one cage constraint performs this reduction**: it removes $v$ from every cell of row $r$ outside the cage. No search, no shave loop. This is the deduction the cell viewpoint cannot make, recovered as a first-level domain reduction, for exactly one reason: in the dual the fact "value $v$ is confined to columns $S$" *is* the domain of a variable, whereas in the primal it is a property of a set of cells that no single cell domain records.

The strength obtained is per-constraint GAC over $\{\,\text{channel},\ \mathrm{AllDifferent}(P[r,\cdot]),\ \text{cage disjunctions}\,\}$. It delivers forced-value confinement and the value-set correlations — when a disjunct dies (one of its memberships becomes impossible) the surviving disjuncts tighten $U$ and push more values out of $S$, e.g. forcing a 2 into a ×42 cage kills $\{1,6,7\}$ and confines 3 as well. It does **not** deliver full constructive disjunction: eliminations where no value is universal yet every disjunct independently kills some distant candidate are out of reach and remain a matter for shaving / singleton arc consistency, deliberately out of scope here. Expressing the Latin core dually loses nothing on the all-different itself: GAC on the channelling constraints already equals GAC on all-different (Walsh 2001).

## Decision

**Represent the board in all three viewpoints — the primal cell slice $D$, and the dual value-position slices $P$ and $Q$ — linked by channelling, and post each cage as a codomain disjunction on whichever slice it is aligned to.** Forced-value confinement then arises inside the single `fixpoint` worklist as an ordinary GAC domain reduction, with no constructive-disjunction loop in the core.

Concretely:

- The Latin core is channelling between $D$, $P$, $Q$, plus one cell-uniqueness all-different family run by the existing Régin propagator. $P$ and $Q$ are not a replacement for the cell model; they are redundant viewpoints whose only job is to give value-position facts a variable to live on.
- A horizontal cage posts its disjunction on $P$; a vertical cage on $Q$; a blocky cage stays a cube-cell constraint (the cell-slice memo of ADR-0006), gaining no confinement, which is correct — a blocky cage genuinely confines nothing to a single line.
- The cage's viable value-sets are read from the memo that already exists (ADR-0006). No new enumeration is introduced; the dual is a re-indexing of state plus per-cage disjunction constraints over it.

This is the dual-viewpoint / redundant-modelling pattern (Cheng, Choi, Lee & Wu 1999; Walsh 2001; Hnich, Smith & Walsh 2004); the $n^3$ occurrence cube is the standard exact-cover encoding of Latin-square problems (Knuth 2000). The contributed piece is posting the arithmetic cage as a disjunction over the value-position slices, which is what turns confinement into channelled propagation rather than search.

## Efficient implementation

The whole state is small enough to be a few hundred bytes of bitmasks and resident in cache; grid size is bounded at $n \le 9$, so every value domain over $\{1,\dots,n\}$ is an $n$-bit mask in the existing value-bitset type (`Fill`).

**State.** Three arrays of $n$-bit masks, each $n^2$ entries:

- `cell[r][c]` — value mask (the present primal `Fill`).
- `P[r][v]` — column mask: which columns value $v$ may take in row $r$.
- `Q[c][v]` — row mask: which rows value $v$ may take in column $c$.

For $n = 9$ each array is $81 \times 2$ bytes; all three together are well under 1 KB.

**Channelling as bit-sync.** A primitive change is "value $v$ cannot be at $(r,c)$" — clearing one cube bit — mirrored into all three arrays: clear bit $c$ of `cell[r][c]`'s position via the value, clear bit $c$ of `P[r][v]`, clear bit $r$ of `Q[c][v]`. A worklist over changed bits drives the rest. The channel is a handful of mask writes per change; it is the glue that carries a confinement found on $P$ over to $Q$ and the cell slice.

**Cage propagation (horizontal; vertical is the transpose on $Q$).** A cage stores its viable value-sets as masks $\{M_1, \dots, M_t\}$, its column mask $S$, and its orientation.

1. **Liveness.** Disjunct $M_i$ is alive iff every $v \in M_i$ still has `P[r][v] & S` nonzero. (A necessary condition; the exact within-$S$ matching feasibility is left to Régin on the row, so the cage propagator stays branch-free and cheap.)
2. **Forced in.** $U =$ `AND` of the alive $M_i$. For each $v \in U$: `P[r][v] &= S`. This is the confinement.
3. **Forced out.** `V_in` $=$ `OR` of the alive $M_i$. For each value $v \notin$ `V_in`: `P[r][v] &= ~S` — a value in no surviving fill cannot occupy the cage's columns.

Steps 2–3 are $O(t + n)$ mask operations with $t$ small (the viable-multiset count the Designer already reports). The value-set correlation propagates through liveness: a disjunct dying shrinks $U$ and `V_in`, tightening both directions on the next pass.

**Régin on bitsets.** `regin.rs` runs unchanged on the small per-line bitset domains; at $n \le 9$ the matching and SCC pass are negligible. At minimum it enforces the one cell-uniqueness family; running it additionally on the column and $Q$ slices buys extra pruning at trivial cost.

**Fixpoint integration.** Cage disjunctions, channelling, and Régin are all `PuzzleConstraint`s in the existing `generalized_arc_consistency` worklist. Confinement therefore occurs *inside* one fixpoint call; nothing wraps `fixpoint`.

**On the disjunction encoding.** The cage disjunction is a small OR of conjunctive membership masks. A Horn-clause or watched-literal (SAT-style) encoding is possible, but at $t$ and $n$ this small it buys nothing over folding the $t$ masks directly, and the direct fold is branch-predictable and allocation-free. The bitmask fold is the recommended form; a clause encoding is recorded only as the alternative it is.

## Options Considered

### Option A: Dual value-position viewpoints, channelled — *chosen*

| Dimension | Assessment |
|-----------|------------|
| Confinement | First-level GAC domain reduction; no loop |
| Partial puzzles | No special case — partial state is just unfixed cube bits |
| Régin | Reused as-is on bitset line domains |
| State cost | $3\times$ the domain arrays + channel sync — sub-kilobyte at $n \le 9$ |
| Generality | Clean only for axis-aligned cages; blocky cages stay cube-level |
| Novelty | Established (redundant modelling / dual viewpoints) |

**Pros:** The deduction becomes ordinary propagation in the existing fixpoint. Incomplete puzzles stop being a separate mode. Reuses the cage memo and Régin. Both value axes covered by maintaining both slices.
**Cons:** Carries two extra viewpoints and the channelling that keeps them honest. Blocky cages gain nothing. Captures confinement and value-set correlation, not full constructive disjunction.

### Option B: Primal-only confinement propagator from the intersection

Keep the cell model; compute $U = \bigcap_i M_i$ from the cage memo and post a bespoke propagator whose scope widens to the cage's line, emitting $v \notin \{\text{rest of line}\}$ for $v \in U$.

| Dimension | Assessment |
|-----------|------------|
| Confinement | Yes, for line-aligned cages |
| Partial puzzles | Still special-cased in the cell model |
| Régin | Reused |
| State cost | None beyond today |
| Generality | Bespoke per axis; one-value-at-a-time |
| Novelty | Local, ad hoc |

**Pros:** Smallest change; the memo already yields $U$; no new variables. Captures the same confinement family cheaply.
**Cons:** A special propagator outside the uniform GAC story; the cage's effective scope must reach past its own cells. It is the universal-value slice of Option A done by hand, and it does not generalize to the value-set correlations or to multi-value Hall confinement without growing into a second implementation of what the dual gives uniformly.

### Option C: Constructive disjunction (shaving) around the fixpoint

Branch each cage on its viable value-sets, run `fixpoint` under each, and intersect the surviving domains.

| Dimension | Assessment |
|-----------|------------|
| Confinement | Yes |
| Strength | Strongest — also catches non-universal cross-cage effects |
| Régin | Reused inside each branch |
| State cost | None beyond today |
| Cost model | A fixpoint pass per branch per cage per outer round |
| Novelty | Standard (constructive disjunction / SAC) |

**Pros:** Most powerful; matches the fullest manual reasoning; no representation change.
**Cons:** A meta-loop over `fixpoint`, not a first-level constraint; cost is multiplicative in the branch count; it does not address the partial-puzzle special-casing that motivates the dual.

## Trade-off Analysis

A versus B is a question of uniformity. Both reach the confinement family. B is the cheaper diff and adds no state, but it is a special-purpose propagator that has to reach outside its constraint's natural scope, and its reach stops at single universal values; pushing it to the value-set correlations and to the column axis re-derives, piecemeal, exactly the structure A provides once. A pays a fixed price — two redundant slices and their channelling — and in return the deduction is just GAC, the second axis is symmetric, and the long-standing irritant of special-casing incomplete puzzles disappears, because in the cube an unsolved board is only a cube with unfixed bits, propagated by the same constraints at every fill level.

A versus C is a question of where the power ceiling sits. C is strictly stronger — it catches eliminations with no universal value — but it buys that strength with a propagation pass per branch and remains a loop outside the fixpoint, and it leaves the partial-puzzle ergonomics untouched. A keeps the engine a single GAC fixpoint and takes the confinement-and-correlation tier, which is the tier the motivating deductions occupy. C is not foreclosed: it can be layered on later as a shaving pass over the same dual state when a puzzle needs the non-universal tier, and the dual representation only makes each branch's propagation cheaper.

The cost A is most fairly charged with is the $n^3$ cube and its channelling — the point that the dual is more bits to schedule than the cell model. Those bits are not new information; they are the same board re-indexed, and at $n \le 9$ the entire state is sub-kilobyte and cache-resident, so the real cost is propagator scheduling and channel synchronization, both bounded and small.

## Consequences

- Forced-value confinement becomes a first-class GAC reduction inside `Puzzle::fixpoint`; the constructive-disjunction loop is unnecessary for it.
- Incomplete puzzles cease to be a distinct mode. A partial board is a cube with unfixed bits, and every constraint is GAC over it regardless of fill level — removing a class of special-casing that existed only because the cell viewpoint could not represent "a value is somewhere in here, unplaced."
- The cage memo (ADR-0006) is reused, not duplicated: its projected viable value-sets are the masks the cage propagator folds. The dual adds viewpoints and per-cage disjunctions, not a second relation compiler.
- Régin is unchanged; it runs on bitset line domains. The Latin core needs one cell-uniqueness all-different family for correctness, since channelling already entails the per-value row/column families; additional all-different slices are optional propagation strength.
- Axis-aligned cages obtain clean single-slice confinement; blocky cages remain cube-cell constraints with no line confinement, which is correct rather than a gap.
- The achieved strength is per-constraint GAC: forced values plus value-set correlations. Eliminations requiring full constructive disjunction (no universal value) are out of scope and remain available only through a future shaving pass, which can sit atop this state unchanged.
- State grows to three slices plus channelling; at $n \le 9$ this is sub-kilobyte and the channel is a few mask writes per change. The disjunction is stored and propagated as a fold over bitmask value-sets; a clause/SAT encoding is an unused alternative at this scale.
- This builds on ADR-0006 without disturbing it. ADR-0006 fixed what a single cage's relation *is* (arithmetic $\wedge$ internal distinctness) and how it is compiled; ADR-0008 adds how that relation is *posted* — onto value-position slices — so its consequences cross between a cage and the lines it lies in.

## References

- J.-C. Régin. *A Filtering Algorithm for Constraints of Difference in CSPs.* AAAI-94, 362–367. GAC for all-different via bipartite matching — the propagator reused here.
- B. M. W. Cheng, K. M. F. Choi, J. H. M. Lee, J. C. K. Wu. *Increasing Constraint Propagation by Redundant Modeling: an Experience Report.* Constraints 4(2):167–192, 1999. Mutually redundant models linked by channelling constraints.
- T. Walsh. *Permutation Problems and Channelling Constraints.* LPAR 2001, LNCS 2250:377–391. Establishes that GAC on the channelling constraints equals GAC on the primal all-different, and that decomposed not-equals constraints add nothing over channelling.
- B. Hnich, B. M. Smith, T. Walsh. *Dual Modelling of Permutation and Injection Problems.* JAIR 21:357–391, 2004. Theory and experiments on primal, dual, and combined viewpoints for permutation problems.
- D. E. Knuth. *Dancing Links.* 2000 (arXiv:cs/0011047). Exact-cover encoding of Latin-square / Sudoku boards as $n^3$ occurrence rows — the cube model in its combinatorial form.
- ADR-0005 (cage memo ownership) and ADR-0006 (commutative cage memos enforce collinear distinctness) — the relation whose projection supplies each cage's viable value-sets.
