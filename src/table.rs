// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Write;

use markdown::mdast::Node;
use unicode_width::UnicodeWidthStr;

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
/// Lines may contain ANSI SGR escape sequences for styled header cells
/// (bold) — these are embedded directly rather than routed through the
/// `Renderer`'s style stack, since this module is intentionally pure and
/// stateless.
///
/// Layout rules:
///   - Header row text is **centered** and **bold**.
///   - Data rows are left-aligned with one space padding on each side.
///   - Horizontal separators appear between every pair of adjacent rows.
///   - Column widths are sized to the widest cell in each column.
///
/// An empty table (no rows) yields an empty `Vec`; the caller is
/// expected to treat that as "nothing to render".
pub fn layout_table(table: &markdown::mdast::Table) -> Vec<String> {
    let rows: Vec<Vec<String>> = table.children.iter().map(row_texts).collect();
    if rows.is_empty() {
        return Vec::new();
    }

    // ── Measure column widths ────────────────────────────────────────────
    // Each column is sized to fit the widest cell. Short rows (fewer
    // cells than the max) contribute nothing and are later padded with
    // empty cells during rendering.
    let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut col_widths: Vec<usize> = vec![0; num_cols];
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            let w = UnicodeWidthStr::width(cell.as_str());
            if w > col_widths[i] {
                col_widths[i] = w;
            }
        }
    }

    // ── Build the lines ──────────────────────────────────────────────────
    let mut lines: Vec<String> = Vec::new();
    lines.push(border_line(&col_widths, TOP_LEFT, TOP_T, TOP_RIGHT));

    for (row_idx, row) in rows.iter().enumerate() {
        lines.push(row_line(row, &col_widths, /* is_header */ row_idx == 0));
        if row_idx < rows.len() - 1 {
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
// Line builders
// =========================================================

/// Build a single row of table content as one line. Header rows are
/// centered and bold; data rows are left-aligned.
fn row_line(cells: &[String], col_widths: &[usize], is_header: bool) -> String {
    let mut line = String::new();

    for (col, width) in col_widths.iter().copied().enumerate() {
        // Left border of this cell (which also doubles as the right
        // border of the previous cell) + one space of padding.
        line.push_str(VERTICAL);
        line.push(' ');

        let cell: &str = cells.get(col).map(String::as_str).unwrap_or("");
        let cell_width = UnicodeWidthStr::width(cell);
        let padding = width.saturating_sub(cell_width);

        if is_header {
            // Header cells are centered. The extra column of the padding
            // (when `padding` is odd) goes on the right.
            let left_pad = padding / 2;
            let right_pad = padding - left_pad;
            let _ = write!(
                line,
                "{SGR_BOLD}{:left_pad$}{cell}{:right_pad$}{SGR_NORMAL_INTENSITY}",
                "", "",
            );
        } else {
            let _ = write!(line, "{cell}{:padding$}", "");
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
