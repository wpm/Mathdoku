//! Tooltip copy for the in-app help system (ADR-0007).
//!
//! The app gets tooltips, not documentation: every string here is rendered
//! through native `title` attributes (HTML controls) or SVG `<title>`
//! children (grid elements), so the same copy behaves identically in the
//! Tauri desktop build and the `--features web` browser preview. Long-form
//! conceptual and workflow documentation lives on the project website, which
//! the desktop Help menu links to.
//!
//! Centralizing the copy in one module keeps it reviewable as user-facing UI
//! text and keeps wording consistent across the components that show it.

use mathdoku::Operator;

/// Tooltip for an operator tab in the operation selector, explaining the
/// constraint that operator places on the cage.
#[must_use]
pub const fn operator_tooltip(op: Operator) -> &'static str {
    match op {
        Operator::Given => "Given: the cell holds exactly this value.",
        Operator::Add => "Add: the cage's values must sum to the target.",
        Operator::Subtract => {
            "Subtract: the difference between the cage's two values must equal the target."
        }
        Operator::Multiply => "Multiply: the cage's values must multiply to the target.",
        Operator::Divide => {
            "Divide: the larger of the cage's two values divided by the smaller \
             must equal the target."
        }
    }
}

/// Tooltip for the puzzle grid, summarizing keyboard-driven cage construction.
pub const GRID_TOOLTIP: &str = "Arrow keys move the selection. Hold Shift and use the arrow keys \
     to draw a cage, then press Enter to choose its operator. Press Escape on a cage to rework \
     it, or Delete to remove it.";

/// Tooltip for the grid-size dropdown in the New-puzzle dialog.
pub const SIZE_SELECT_TOOLTIP: &str =
    "Grid size: an n\u{d7}n puzzle is filled with the numbers 1 through n.";

/// Tooltip for the New-puzzle dialog's creation button — "Random Solution"
/// in `without-solution` builds, "Create" in with-solution-only builds.
pub const RANDOM_SOLUTION_TOOLTIP: &str = "Create a puzzle with a fixed random solution; every \
     cage's target is computed from that solution as you draw.";

/// Tooltip for the New-puzzle dialog's "No Solution" button (Without-Solution
/// builds only).
#[cfg(feature = "without-solution")]
pub const NO_SOLUTION_TOOLTIP: &str = "Create an empty puzzle with no fixed solution; you choose \
     each cage's operator and target, filtered to what keeps the puzzle solvable.";

/// Tooltip for the solution-count indicator below the grid.
pub const SOLUTION_COUNT_TOOLTIP: &str = "How many ways the finished grid can be filled. A fair, \
     publishable puzzle has exactly 1 solution.";

/// Tooltip for the per-cage multiset/tuple statistics below the grid.
pub const CAGE_STATS_TOOLTIP: &str = "Viable value combinations for the selected cage: multisets \
     ignore cell order, tuples count each arrangement.";

/// Tooltip for the Without-Solution target dropdown in the operation selector.
#[cfg(feature = "without-solution")]
pub const TARGET_SELECT_TOOLTIP: &str =
    "Choose the cage's target; only values that keep the puzzle solvable are listed.";

/// Tooltip for the Save button in the unsaved-changes dialog.
pub const SAVE_TOOLTIP: &str = "Save the puzzle as a .mathdoku file before closing.";

/// Tooltip for the Don't Save button in the unsaved-changes dialog.
pub const DISCARD_TOOLTIP: &str = "Close without saving; unsaved changes are lost.";

#[cfg(test)]
mod tests {
    use super::operator_tooltip;
    use mathdoku::Operator;

    const ALL_OPERATORS: [Operator; 5] = [
        Operator::Given,
        Operator::Add,
        Operator::Subtract,
        Operator::Multiply,
        Operator::Divide,
    ];

    #[test]
    fn every_operator_has_a_distinct_tooltip() {
        let tips: Vec<&str> = ALL_OPERATORS.into_iter().map(operator_tooltip).collect();
        for (i, tip) in tips.iter().enumerate() {
            assert!(!tip.is_empty());
            assert!(
                tips[i + 1..].iter().all(|other| other != tip),
                "duplicate tooltip: {tip}"
            );
        }
    }

    #[test]
    fn arithmetic_tooltips_mention_the_target() {
        for op in [
            Operator::Add,
            Operator::Subtract,
            Operator::Multiply,
            Operator::Divide,
        ] {
            assert!(
                operator_tooltip(op).contains("target"),
                "{op:?} tooltip should explain the target constraint"
            );
        }
    }

    #[test]
    fn tooltips_read_as_sentences() {
        // Tooltip copy is user-facing UI text: complete sentences, terminated.
        let mut all = vec![
            super::GRID_TOOLTIP,
            super::SIZE_SELECT_TOOLTIP,
            super::RANDOM_SOLUTION_TOOLTIP,
            super::SOLUTION_COUNT_TOOLTIP,
            super::CAGE_STATS_TOOLTIP,
            super::SAVE_TOOLTIP,
            super::DISCARD_TOOLTIP,
        ];
        #[cfg(feature = "without-solution")]
        all.extend([super::NO_SOLUTION_TOOLTIP, super::TARGET_SELECT_TOOLTIP]);
        all.extend(ALL_OPERATORS.into_iter().map(operator_tooltip));
        for tip in all {
            assert!(
                tip.ends_with('.'),
                "tooltip should end with a period: {tip}"
            );
            assert!(
                !tip.contains("  "),
                "tooltip has a double space (continuation-line artifact): {tip}"
            );
        }
    }
}
