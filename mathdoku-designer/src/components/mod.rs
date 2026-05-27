//! SVG components that render a Mathdoku puzzle.
//!
//! [`Puzzle`] is the only item exported from this module. All submodules are
//! internal; components wire up via Leptos context rather than direct imports.

pub(super) mod cage;
pub(super) mod cage_stats;
pub(super) mod cell;
pub(super) mod operation_selector;
pub(super) mod puzzle;
pub(super) mod region;
pub(super) mod selection;
pub(super) mod solution_count;

pub use operation_selector::PendingCommit;
pub use puzzle::Puzzle;
