//! Constraint-satisfaction engine: traits, propagation loop, and search.
//!
//! The three submodules form a layered stack:
//!
//! | Layer | Module | Responsibility |
//! |-------|--------|----------------|
//! | Types | [`constraint`] | [`Constraint`](constraint::Constraint) trait, [`PropagationCtx`](constraint::PropagationCtx), [`Outcome`](constraint::Outcome), [`propagate_to_fixpoint`](constraint::propagate_to_fixpoint) |
//! | Constraints | [`all_different`] | GAC all-different via Régin's algorithm |
//! | Search | [`solver`] | Depth-first branching iterator over [`Store`](crate::puzzle::store::Store) states |
//!
//! Concrete constraint implementations live in [`crate::puzzle::cage`] (tuple-based
//! GAC for arithmetic cages) and here in [`all_different`] (bipartite-matching GAC
//! for row/column uniqueness). Both implement [`Constraint`](constraint::Constraint)
//! and are composed by [`crate::puzzle::Puzzle`] into a single propagation pass.

pub mod all_different;
pub mod constraint;
pub mod solver;
