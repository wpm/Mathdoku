//! Cell component: background rect and value digit display.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]

use leptos::prelude::*;

use crate::theme::{GREEN, INK, INK3, OP_INSET, SANS};

/// Background rect and value digits for a single grid cell.
#[component]
#[allow(clippy::needless_pass_by_value)] // Leptos component props must be owned
pub fn Cell(
    x: f64,
    y: f64,
    cell: f64,
    values: Vec<mathdoku::N>,
    fill: &'static str,
    /// Top margin reserved for the cage op label.
    top_margin: f64,
    /// Grid dimension n (used for fallback layout).
    n: usize,
    /// The correct solution value for this cell, if known. For singleton cells
    /// this is ignored (the value is already unambiguous). For multi-value cells
    /// a small green digit is placed in the upper-right corner of the cell while
    /// all candidates are rendered uniformly.
    solution_value: Option<mathdoku::N>,
    /// The candidate values this cell displayed before the change that mounted
    /// this Puzzle, or empty when there is no previous state to compare
    /// against. Candidates present here but missing from `values` flash out as
    /// red ghosts; a collapse from several candidates to one flashes the
    /// singleton digit in.
    prev_values: Vec<mathdoku::N>,
) -> impl IntoView {
    let ghosts = removed_glyphs(x, y, cell, &prev_values, &values, top_margin, n);
    // A cell that just collapsed to a single value flashes its glyph. The
    // class is safe to apply to all glyphs: a singleton renders exactly one.
    let flash_class = (values.len() == 1 && prev_values.len() > 1).then_some("flash-added");
    let glyphs = cell_glyphs(x, y, cell, &values, top_margin, n, solution_value);

    view! {
        <rect x=x y=y width=cell height=cell fill=fill />
        {glyphs.into_iter().map(|(cx, cy, label, font_size, color, weight)| view! {
            <text
                x=cx y=cy
                class=flash_class
                text-anchor="middle"
                dominant-baseline="central"
                font-family=SANS
                font-size=font_size
                font-weight=weight
                fill=color
            >{label}</text>
        }).collect::<Vec<_>>()}
        {ghosts.into_iter().map(|(cx, cy, label, font_size, color, weight)| view! {
            <text
                x=cx y=cy
                class="flash-removed"
                text-anchor="middle"
                dominant-baseline="central"
                font-family=SANS
                font-size=font_size
                font-weight=weight
                fill=color
            >{label}</text>
        }).collect::<Vec<_>>()}
    }
}

const VALUE_EDGE: f64 = 4.0;

/// A positioned digit to render in a cell: `(x, y, label, font_size, fill, font_weight)`.
type Glyph = (f64, f64, String, f64, &'static str, &'static str);

/// Computes the positioned digit glyphs for a cell's values.
///
/// A single value renders one large centred digit. Multiple candidates are laid
/// out as die-style pips (or a square sub-grid for counts above nine), all
/// rendered uniformly. When `solution_value` is present for a multi-value cell
/// a small green digit is added in the upper-right corner.
fn cell_glyphs(
    x: f64,
    y: f64,
    cell: f64,
    values: &[mathdoku::N],
    top_margin: f64,
    n: usize,
    solution_value: Option<mathdoku::N>,
) -> Vec<Glyph> {
    let zone_w = 2.0f64.mul_add(-VALUE_EDGE, cell);
    let zone_h = cell - top_margin - VALUE_EDGE;
    let value_f = (zone_h / 3.5).clamp(7.0, zone_h);

    let mut glyphs: Vec<Glyph> = Vec::new();

    if values.len() == 1 {
        let singleton_f = (cell * 0.5).max(12.0);
        glyphs.push((
            x + cell / 2.0,
            y + cell / 2.0,
            values[0].to_string(),
            singleton_f,
            INK,
            "600",
        ));
    } else if !values.is_empty() {
        let zone_x = x + VALUE_EDGE;
        let zone_y = y + top_margin;
        if let Some(pips) = pip_layout(values.len()) {
            for (i, &(fx, fy)) in pips.iter().enumerate() {
                if let Some(&v) = values.get(i) {
                    glyphs.push((
                        f64::from(fx).mul_add(zone_w, zone_x),
                        f64::from(fy).mul_add(zone_h, zone_y),
                        v.to_string(),
                        value_f,
                        INK3,
                        "normal",
                    ));
                }
            }
        } else {
            // Fallback for count > 9: sub×sub grid.
            let sub = (n as f64).sqrt().ceil() as usize;
            let sub_w = zone_w / sub as f64;
            let sub_h = zone_h / sub as f64;
            for (i, &v) in values.iter().enumerate() {
                let sr = i / sub;
                let sc = i % sub;
                glyphs.push((
                    (sc as f64 + 0.5).mul_add(sub_w, zone_x),
                    (sr as f64 + 0.5).mul_add(sub_h, zone_y),
                    v.to_string(),
                    value_f,
                    INK3,
                    "normal",
                ));
            }
        }

        // Green solution-value digit in the upper-right corner, aligned with
        // the cage op label: same font size, same top inset (OP_INSET).
        if let Some(sv) = solution_value {
            let op_f = 2.0f64.mul_add(-OP_INSET, top_margin).max(7.0);
            glyphs.push((
                x + cell - VALUE_EDGE - op_f * 0.35,
                y + OP_INSET + op_f * 0.5,
                sv.to_string(),
                op_f,
                GREEN,
                "600",
            ));
        }
    }

    glyphs
}

/// Computes ghost glyphs for the candidates in `prev_values` that `values` no
/// longer contains.
///
/// Each ghost sits where its candidate appeared in the previous layout
/// (computed from `prev_values`), so it fades out in place while the surviving
/// candidates re-flow into their new positions.
fn removed_glyphs(
    x: f64,
    y: f64,
    cell: f64,
    prev_values: &[mathdoku::N],
    values: &[mathdoku::N],
    top_margin: f64,
    n: usize,
) -> Vec<Glyph> {
    // cell_glyphs yields one glyph per value, in order (no solution badge).
    cell_glyphs(x, y, cell, prev_values, top_margin, n, None)
        .into_iter()
        .zip(prev_values.iter().copied())
        .filter(|(_, v)| !values.contains(v))
        .map(|(glyph, _)| glyph)
        .collect()
}

fn pip_layout(count: usize) -> Option<&'static [(f32, f32)]> {
    LAYOUTS.get(count.wrapping_sub(1)).copied()
}

// Pip positions as (x, y) fractions in [0,1]² for 1–9 candidate values.
// 1–6 follow standard die faces; 7–9 are symmetric extensions.
const LAYOUTS: [&[(f32, f32)]; 9] = [
    /* 1 */ &[(0.5, 0.5)],
    /* 2 */ &[(0.25, 0.25), (0.75, 0.75)],
    /* 3 */ &[(0.25, 0.25), (0.5, 0.5), (0.75, 0.75)],
    /* 4 */ &[(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)],
    /* 5 */
    &[
        (0.25, 0.25),
        (0.75, 0.25),
        (0.5, 0.5),
        (0.25, 0.75),
        (0.75, 0.75),
    ],
    /* 6 */
    &[
        (0.25, 0.2),
        (0.75, 0.2),
        (0.25, 0.5),
        (0.75, 0.5),
        (0.25, 0.8),
        (0.75, 0.8),
    ],
    /* 7 */
    &[
        (0.25, 0.15),
        (0.75, 0.15),
        (0.25, 0.5),
        (0.5, 0.5),
        (0.75, 0.5),
        (0.25, 0.85),
        (0.75, 0.85),
    ],
    /* 8 */
    &[
        (0.25, 0.14),
        (0.75, 0.14),
        (0.25, 0.38),
        (0.75, 0.38),
        (0.25, 0.62),
        (0.75, 0.62),
        (0.25, 0.86),
        (0.75, 0.86),
    ],
    /* 9 */
    &[
        (0.25, 0.15),
        (0.5, 0.15),
        (0.75, 0.15),
        (0.25, 0.5),
        (0.5, 0.5),
        (0.75, 0.5),
        (0.25, 0.85),
        (0.5, 0.85),
        (0.75, 0.85),
    ],
];

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{cell_glyphs, pip_layout, removed_glyphs};
    use crate::theme::{GREEN, INK, INK3};

    #[test]
    fn layout_count_matches_pip_count_for_one_through_nine() {
        for count in 1..=9 {
            let pips = pip_layout(count);
            assert!(pips.is_some(), "expected a layout for count {count}");
            assert_eq!(
                pips.map(<[(f32, f32)]>::len),
                Some(count),
                "layout for count {count} has the wrong number of pips"
            );
        }
    }

    #[test]
    fn layout_zero_is_none() {
        assert!(pip_layout(0).is_none());
    }

    #[test]
    fn layout_above_nine_is_none() {
        assert!(pip_layout(10).is_none());
        assert!(pip_layout(100).is_none());
    }

    #[test]
    fn layout_pips_are_within_unit_square() {
        for count in 1..=9 {
            if let Some(pips) = pip_layout(count) {
                for &(x, y) in pips {
                    assert!((0.0..=1.0).contains(&x), "x={x} out of range");
                    assert!((0.0..=1.0).contains(&y), "y={y} out of range");
                }
            }
        }
    }

    #[test]
    fn glyphs_empty_values_produces_nothing() {
        let glyphs = cell_glyphs(0.0, 0.0, 60.0, &[], 16.0, 4, None);
        assert!(glyphs.is_empty());
    }

    #[test]
    fn glyphs_singleton_is_one_centered_ink_digit() {
        let glyphs = cell_glyphs(10.0, 20.0, 60.0, &[5], 16.0, 4, None);
        assert_eq!(glyphs.len(), 1);
        let (cx, cy, ref label, _font, fill, weight) = glyphs[0];
        // Centred within the cell.
        assert!((cx - 40.0).abs() < f64::EPSILON);
        assert!((cy - 50.0).abs() < f64::EPSILON);
        assert_eq!(label, "5");
        assert_eq!(fill, INK);
        assert_eq!(weight, "600");
    }

    #[test]
    fn glyphs_multi_value_uses_pip_layout() {
        let glyphs = cell_glyphs(0.0, 0.0, 60.0, &[1, 2, 3], 16.0, 4, None);
        assert_eq!(glyphs.len(), 3);
        let labels: Vec<&str> = glyphs.iter().map(|g| g.2.as_str()).collect();
        assert_eq!(labels, vec!["1", "2", "3"]);
    }

    #[test]
    fn glyphs_solution_value_badge_in_upper_right_for_multi_value() {
        // top_margin=16.0, OP_INSET=4.0 → op_f = 16.0 - 8.0 = 8.0
        let glyphs = cell_glyphs(0.0, 0.0, 60.0, &[1, 2, 3], 16.0, 4, Some(2));
        // Candidates are all rendered uniformly in INK3.
        let candidate_glyphs: Vec<_> = glyphs.iter().filter(|g| g.4 == INK3).collect();
        assert_eq!(candidate_glyphs.len(), 3);
        assert!(candidate_glyphs.iter().all(|g| g.5 == "normal"));
        // A separate green badge for the solution value is appended last.
        let badge = glyphs.last().unwrap();
        assert_eq!(badge.2, "2");
        assert_eq!(badge.4, GREEN);
        assert_eq!(badge.5, "600");
        // Badge is in the right half of the cell.
        assert!(badge.0 > 30.0, "badge should be in the right half");
        // Badge centre is at y + OP_INSET + op_f/2 = 0 + 4 + 4 = 8 — well above top_margin (16).
        assert!(badge.1 < 16.0, "badge should be above the candidate zone");
    }

    #[test]
    fn glyphs_no_badge_without_solution_value() {
        let glyphs = cell_glyphs(0.0, 0.0, 60.0, &[1, 2, 3], 16.0, 4, None);
        assert_eq!(glyphs.len(), 3);
        assert!(glyphs.iter().all(|g| g.4 != GREEN));
    }

    #[test]
    fn removed_ghosts_keep_their_previous_layout_positions() {
        let prev = vec![1, 2, 3, 4];
        let now = vec![1, 3];
        let ghosts = removed_glyphs(0.0, 0.0, 60.0, &prev, &now, 16.0, 4);
        let prev_layout = cell_glyphs(0.0, 0.0, 60.0, &prev, 16.0, 4, None);
        // 2 and 4 vanished; their ghosts sit where the 4-pip layout placed them.
        assert_eq!(ghosts.len(), 2);
        assert_eq!(ghosts[0], prev_layout[1]);
        assert_eq!(ghosts[1], prev_layout[3]);
    }

    #[test]
    fn removed_ghosts_empty_when_nothing_removed() {
        assert!(removed_glyphs(0.0, 0.0, 60.0, &[1, 2], &[1, 2], 16.0, 4).is_empty());
    }

    #[test]
    fn removed_ghosts_empty_without_previous_values() {
        assert!(removed_glyphs(0.0, 0.0, 60.0, &[], &[1, 2], 16.0, 4).is_empty());
    }

    #[test]
    fn removed_ghosts_empty_when_candidates_were_added() {
        // Undo restores candidates: previous values all survive, so no ghosts.
        assert!(removed_glyphs(0.0, 0.0, 60.0, &[1], &[1, 2, 3], 16.0, 4).is_empty());
    }

    #[test]
    fn glyphs_more_than_nine_use_square_fallback() {
        let values: Vec<mathdoku::N> = (1..=10).collect();
        let glyphs = cell_glyphs(0.0, 0.0, 120.0, &values, 16.0, 10, None);
        // The fallback grid renders every candidate.
        assert_eq!(glyphs.len(), 10);
    }
}
