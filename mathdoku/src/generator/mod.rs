//! Puzzle generation: Latin square construction and cage labeling.
//!
//! Generation proceeds in two phases:
//!
//! 1. **Latin square** ([`latin_square`]) — fill an `n`×`n` grid with a random
//!    permutation-matrix solution using a randomized backtracking solver.
//! 2. **Cage labeling** ([`generate`]) — partition the filled grid into cages
//!    (polyominoes drawn from a [`SizeDistribution`](generate::SizeDistribution)),
//!    then assign each cage an [`Operation`](crate::Operation) and target value
//!    via a caller-supplied or default policy.
//!
//! The public entry points are re-exported on [`Puzzle`](crate::puzzle::Puzzle):
//! [`Puzzle::generate`](crate::puzzle::Puzzle::generate) and
//! [`Puzzle::generate_with`](crate::puzzle::Puzzle::generate_with).

pub mod generate;
mod latin_square;
