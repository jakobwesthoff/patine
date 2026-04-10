// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Write;

use markdown::mdast::Node;
use unicode_width::UnicodeWidthStr;

use crate::wrap;

// =========================================================
// Box-drawing characters
// =========================================================

const TOP_LEFT: &str = "┌";
const TOP_RIGHT: &str = "┐";
const BOTTOM_LEFT: &str = "└";
const BOTTOM_RIGHT: &str = "┘";
const TOP_T: &str = "┬";
const BOTTOM_T: &str = "┴";
const LEFT_T: &str = "├";
const RIGHT_T: &str = "┤";
const CROSS: &str = "┼";
const HORIZONTAL: &str = "─";
const VERTICAL: &str = "│";

// ANSI SGR sequences for header styling. The table module emits these
// directly into the returned line strings rather than going through the
// Renderer's style machinery — this keeps `layout_table` pure.
//
// NormalIntensity (SGR 22) is used instead of NoBold (SGR 21) because
// several terminals misinterpret SGR 21 as "doubly underlined".
const SGR_BOLD: &str = "\x1b[1m";
const SGR_NORMAL_INTENSITY: &str = "\x1b[22m";

/// Per-cell padding overhead on each line. Each cell contributes
/// `"│ "` on its left and a single space of right padding (the next
/// cell's `"│ "` doubles as this cell's right border). The final
/// closing `│` adds one more column to the whole row. These numbers
/// are used to translate between "available terminal width" and
/// "content width budget" for the column distribution pass.
const PER_CELL_OVERHEAD: usize = 3; // "│ " + trailing " "
const TRAILING_BORDER: usize = 1; // final "│"

// =========================================================
// Cell text extraction
// =========================================================

/// Recursively extract the plain text content of an AST node, stripping
/// all formatting markers.
fn extract_text(node: &Node) -> String {
    match node {
        Node::Text(t) => t.value.clone(),
        Node::InlineCode(c) => c.value.clone(),
        other => other
            .children()
            .map(|children| children.iter().map(extract_text).collect::<String>())
            .unwrap_or_default(),
    }
}

/// Extract the plain text for each cell in a row.
fn row_texts(row: &Node) -> Vec<String> {
    match row {
        Node::TableRow(r) => r
            .children
            .iter()
            .map(|cell| match cell {
                Node::TableCell(c) => c.children.iter().map(extract_text).collect::<String>(),
                other => extract_text(other),
            })
            .collect(),
        _ => Vec::new(),
    }
}

// =========================================================
// Table layout
// =========================================================

/// Lay out a GFM table as a list of pre-formatted lines. Each returned
/// line is ready to be written verbatim by the caller *after* the
/// caller emits its own per-line prefix (nesting indent, blockquote
/// bars, etc.) via its normal `write_indent` machinery.
///
/// `available_width` is the number of display columns the caller is
/// willing to give the table (typically `Renderer::effective_width`
/// after subtracting nesting and blockquote prefixes). The layout pass
/// uses it to shrink wide tables down via proportional column
/// distribution and cell wrapping. When the shrink floor (per-column
/// minimum widths) still exceeds `available_width`, the table is
/// rendered at its minimum widths and overflows — the caller is
/// expected to tolerate this, same as the long-URL-on-its-own-line
/// behavior.
///
/// Lines may contain ANSI SGR escape sequences for styled header cells
/// (bold) — these are embedded directly rather than routed through the
/// `Renderer`'s style stack, since this module is intentionally pure
/// and stateless.
///
/// Layout rules:
///   - Header row text is **centered** and **bold**, per wrapped line.
///   - Data rows are left-aligned with one space padding on each side.
///   - Horizontal separators appear between every pair of adjacent
///     rows.
///   - Column widths are sized to fit within `available_width` when
///     possible, shrinking proportionally toward each column's minimum
///     (widest unbreakable token).
///
/// An empty table (no rows) yields an empty `Vec`; the caller is
/// expected to treat that as "nothing to render".
pub fn layout_table(table: &markdown::mdast::Table, available_width: usize) -> Vec<String> {
    let rows: Vec<Vec<String>> = table.children.iter().map(row_texts).collect();
    if rows.is_empty() {
        return Vec::new();
    }

    let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if num_cols == 0 {
        return Vec::new();
    }

    // ── Measure per-column natural and minimum widths ───────────────────
    // `natural`[i] = width of the widest cell in column i (what we'd
    //                use if space were unlimited).
    // `minimum`[i] = width of the widest single unbreakable token in any
    //                cell in column i — the floor below which wrapping
    //                cannot shrink the column without making a token
    //                overflow. We additionally floor at 1 so that a
    //                column of empty cells still has a visible, paddable
    //                slot.
    let mut natural: Vec<usize> = vec![0; num_cols];
    let mut minimum: Vec<usize> = vec![0; num_cols];
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            let w = UnicodeWidthStr::width(cell.as_str());
            if w > natural[i] {
                natural[i] = w;
            }
            let m = wrap::widest_word(cell);
            if m > minimum[i] {
                minimum[i] = m;
            }
        }
    }
    for m in &mut minimum {
        *m = (*m).max(1);
    }
    // A column cannot be asked to render narrower than its minimum.
    for i in 0..num_cols {
        if natural[i] < minimum[i] {
            natural[i] = minimum[i];
        }
    }

    // ── Compute the column budget and distribute widths ─────────────────
    // The table's total line width is sum(col_widths) + 3*num_cols + 1
    // (see PER_CELL_OVERHEAD / TRAILING_BORDER). Solve for how much
    // budget is available for cell content given the terminal width.
    let border_overhead = PER_CELL_OVERHEAD * num_cols + TRAILING_BORDER;
    let content_budget = available_width.saturating_sub(border_overhead);
    let col_widths = distribute_widths(&natural, &minimum, content_budget);

    // ── Wrap each cell to its allocated column width ────────────────────
    // Each cell becomes a Vec<String> of wrapped lines. Row height is
    // the max cell height; shorter cells are rendered blank on the
    // extra lines so the whole row aligns vertically.
    let wrapped_rows: Vec<Vec<Vec<String>>> = rows
        .iter()
        .map(|row| {
            (0..num_cols)
                .map(|i| {
                    let text: &str = row.get(i).map(String::as_str).unwrap_or("");
                    wrap::wrap_text(text, col_widths[i])
                })
                .collect()
        })
        .collect();

    // ── Build the lines ─────────────────────────────────────────────────
    let mut lines: Vec<String> = Vec::new();
    lines.push(border_line(&col_widths, TOP_LEFT, TOP_T, TOP_RIGHT));

    for (row_idx, wrapped) in wrapped_rows.iter().enumerate() {
        let row_height = wrapped.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let is_header = row_idx == 0;
        for line_idx in 0..row_height {
            lines.push(row_line(wrapped, line_idx, &col_widths, is_header));
        }
        if row_idx < wrapped_rows.len() - 1 {
            lines.push(border_line(&col_widths, LEFT_T, CROSS, RIGHT_T));
        }
    }

    lines.push(border_line(
        &col_widths,
        BOTTOM_LEFT,
        BOTTOM_T,
        BOTTOM_RIGHT,
    ));

    lines
}

// =========================================================
// Column width distribution
// =========================================================

/// Distribute `budget` columns across a set of columns, respecting
/// each column's minimum width and shrinking proportionally away from
/// its natural width.
///
/// Rules:
/// 1. If the natural widths already fit within `budget`, return them
///    unchanged (fast path — tables that fit don't shrink).
/// 2. Otherwise, seed every column with its `minimum` width.
/// 3. If even the minimums exceed `budget`, return the minimums and
///    accept the overflow. This is the graceful fallback: a table of
///    unbreakable tokens stays readable at the cost of its width.
/// 4. Otherwise, distribute `leftover = budget - sum(minimums)` on top
///    of each column proportionally to its "want" (`natural[i] -
///    minimum[i]`). Any rounding remainder is given out round-robin to
///    columns still below their natural width.
fn distribute_widths(natural: &[usize], minimum: &[usize], budget: usize) -> Vec<usize> {
    let num_cols = natural.len();
    debug_assert_eq!(num_cols, minimum.len());

    let natural_sum: usize = natural.iter().sum();
    if natural_sum <= budget {
        return natural.to_vec();
    }

    let minimum_sum: usize = minimum.iter().sum();
    if minimum_sum >= budget {
        // Even the minimums don't fit. Return them and let the caller
        // accept the overflow — preferred over making content
        // unreadable.
        return minimum.to_vec();
    }

    // At this point sum(minimum) < budget < sum(natural). We have
    // `leftover` columns of budget above the minimums to distribute
    // among columns that "want" more (natural > minimum).
    let leftover = budget - minimum_sum;
    let wants: Vec<usize> = natural
        .iter()
        .zip(minimum.iter())
        .map(|(n, m)| n - m)
        .collect();
    let total_want: usize = wants.iter().sum();

    let mut result = minimum.to_vec();
    if total_want == 0 {
        // No column wants more than its minimum; nothing to distribute.
        return result;
    }

    // First pass: proportional share via integer division. Each column
    // gets `share_i = (want_i * leftover) / total_want`, capped at its
    // own `want_i` so we never exceed the natural width.
    for i in 0..num_cols {
        let share = (wants[i] * leftover) / total_want;
        result[i] += share.min(wants[i]);
    }

    // Second pass: distribute any rounding remainder round-robin to
    // columns that are still below their natural width. Terminates in
    // at most one full circuit because `distributed <= leftover` and
    // `leftover < total_want` (by construction of `total_want > 0`
    // when we reached this branch).
    let mut distributed: usize = result.iter().zip(minimum.iter()).map(|(r, m)| r - m).sum();
    let mut i = 0;
    while distributed < leftover {
        if result[i] < natural[i] {
            result[i] += 1;
            distributed += 1;
        }
        i = (i + 1) % num_cols;
        // Safety: if every column has reached its natural width, stop.
        if (0..num_cols).all(|j| result[j] >= natural[j]) {
            break;
        }
    }

    result
}

// =========================================================
// Line builders
// =========================================================

/// Build a single visual line of a wrapped row. `wrapped[col]` is the
/// list of wrapped lines for column `col`; `line_idx` is which of those
/// wrapped lines we're emitting on this visual line. A cell that is
/// shorter than the current row's height contributes an empty string
/// on the overflow lines.
fn row_line(
    wrapped: &[Vec<String>],
    line_idx: usize,
    col_widths: &[usize],
    is_header: bool,
) -> String {
    let mut line = String::new();

    for (col, &width) in col_widths.iter().enumerate() {
        // Left border of this cell (also doubles as the right border of
        // the previous cell) + one space of padding.
        line.push_str(VERTICAL);
        line.push(' ');

        let cell_line: &str = wrapped[col]
            .get(line_idx)
            .map(String::as_str)
            .unwrap_or("");
        let cell_width = UnicodeWidthStr::width(cell_line);
        // If a token overflowed its allotted width during wrapping,
        // `cell_width` may be greater than `width`. In that case we
        // produce zero padding — the row will be wider than `width`
        // for this line, which is the overflow fallback.
        let padding = width.saturating_sub(cell_width);

        if is_header {
            // Header cells are centered per wrapped line. The extra
            // column of the padding (when `padding` is odd) goes on
            // the right. Each wrapped header line carries its own
            // bold/normal escape pair so terminals that see only the
            // opening SGR of a line still render correctly.
            let left_pad = padding / 2;
            let right_pad = padding - left_pad;
            let _ = write!(
                line,
                "{SGR_BOLD}{:left_pad$}{cell_line}{:right_pad$}{SGR_NORMAL_INTENSITY}",
                "", "",
            );
        } else {
            let _ = write!(line, "{cell_line}{:padding$}", "");
        }

        // Trailing space of the cell's padding.
        line.push(' ');
    }

    // Final right border of the row.
    line.push_str(VERTICAL);
    line
}

/// Build a horizontal border line: `left` + repeated `─` segments joined
/// by `middle`, ending with `right`. Width of each segment is the cell
/// column width plus two — one for each side of the cell padding.
fn border_line(col_widths: &[usize], left: &str, middle: &str, right: &str) -> String {
    let mut line = String::new();
    line.push_str(left);
    for (i, &w) in col_widths.iter().enumerate() {
        for _ in 0..(w + 2) {
            line.push_str(HORIZONTAL);
        }
        if i < col_widths.len() - 1 {
            line.push_str(middle);
        }
    }
    line.push_str(right);
    line
}

// =========================================================
// Unit tests
// =========================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distribute_returns_natural_when_it_fits() {
        let natural = vec![5, 10, 15];
        let minimum = vec![1, 1, 1];
        let budget = 30;
        assert_eq!(distribute_widths(&natural, &minimum, budget), vec![5, 10, 15]);
    }

    #[test]
    fn distribute_returns_minimum_when_budget_too_small() {
        let natural = vec![10, 20, 30];
        let minimum = vec![5, 8, 12];
        let budget = 20; // < sum(minimum) = 25
        assert_eq!(distribute_widths(&natural, &minimum, budget), vec![5, 8, 12]);
    }

    #[test]
    fn distribute_shrinks_proportionally() {
        // Naturals sum to 60, minimums sum to 6, budget = 30.
        // leftover = 24 to distribute over total_want = 54.
        // Wants: [9, 18, 27]. Proportional: [9*24/54=4, 18*24/54=8,
        //   27*24/54=12], capped at wants. Sum = 24. Perfect fit.
        // Result: [1+4, 2+8, 3+12] = [5, 10, 15].
        let natural = vec![10, 20, 30];
        let minimum = vec![1, 2, 3];
        let budget = 30;
        let result = distribute_widths(&natural, &minimum, budget);
        assert_eq!(result.iter().sum::<usize>(), 30);
        assert!(result[0] < result[1]);
        assert!(result[1] < result[2]);
        assert!(result.iter().zip(minimum.iter()).all(|(r, m)| r >= m));
        assert!(result.iter().zip(natural.iter()).all(|(r, n)| r <= n));
    }

    #[test]
    fn distribute_handles_rounding_remainder_via_round_robin() {
        // A case where integer division leaves a remainder: naturals
        // sum = 20, minimums sum = 3, budget = 10. leftover = 7 on
        // total_want = 17. Shares: [7*7/17=2, 6*7/17=2, 4*7/17=1],
        // sum = 5. Remainder of 2 is distributed round-robin.
        let natural = vec![8, 7, 5];
        let minimum = vec![1, 1, 1];
        let budget = 10;
        let result = distribute_widths(&natural, &minimum, budget);
        assert_eq!(result.iter().sum::<usize>(), 10);
        assert!(result.iter().zip(minimum.iter()).all(|(r, m)| r >= m));
        assert!(result.iter().zip(natural.iter()).all(|(r, n)| r <= n));
    }
}
