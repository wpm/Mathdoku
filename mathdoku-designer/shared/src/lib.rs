//! Types shared between the Tauri backend (`src-tauri`) and the Leptos
//! frontend (`src`). Keeping them here avoids duplicating serde definitions
//! and ensures both sides agree on a serialization format over the IPC bridge.

use std::collections::BTreeSet;

use mathdoku::{Cell, Grid, Polyomino, Puzzle};
use serde::{Deserialize, Serialize};

/// Document state returned by `get_doc_state`.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DocState {
    pub dirty: bool,
    pub path: Option<String>,
}

/// Result of `save_puzzle`, carrying the path that was written.
#[derive(Clone, Serialize, Deserialize)]
pub struct SaveResult {
    pub path: String,
}

/// Full designer state: the unit of serialization for save files and undo/redo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    /// Cage structure being designed.
    pub puzzle: Puzzle,
    /// The fixed Latin Square generated when the puzzle was created.
    pub solution: Grid,
    /// Working grid: cell domains constrained by the current cages.
    pub current: Grid,
    /// The currently active cell.
    pub active: Cell,
    /// Provisional cage regions: disjoint from each other and from puzzle cages.
    pub provisional_cages: BTreeSet<Polyomino>,
}

impl State {
    /// Creates a new blank `State` for an *n*×*n* puzzle with no solution or cages.
    ///
    /// # Errors
    /// Returns an error if `n` is invalid for `Puzzle` or `Grid`.
    pub fn new(n: usize) -> Result<Self, String> {
        let puzzle = Puzzle::new(n).map_err(|e| e.to_string())?;
        let solution = Grid::new(n).map_err(|e| e.to_string())?;
        let current = solution.clone();
        Ok(Self {
            puzzle,
            solution,
            current,
            active: Cell::new(0, 0),
            provisional_cages: BTreeSet::new(),
        })
    }
}
