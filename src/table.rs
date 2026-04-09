// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::Write;

use anyhow::{Context, Result};
use crossterm::{
    queue,
    style::{Attribute, SetAttribute},
};
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
// Table rendering
// =========================================================

/// Render a GFM table with Unicode box-drawing borders.
///
/// Layout rules (from output-style.md):
///   - Header row text is **centered** and **bold**.
///   - Data rows are **left-aligned** with 1 space padding on each side.
///   - Horizontal separators appear between the header and every data row.
///   - Column widths are sized to the widest cell in each column.
pub fn render_table<W: Write>(
    writer: &mut W,
    table: &markdown::mdast::Table,
    indent: &str,
) -> Result<()> {
    let rows: Vec<Vec<String>> = table.children.iter().map(row_texts).collect();
    if rows.is_empty() {
        return Ok(());
    }

    // ── Measure column widths ────────────────────────────────────────────
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

    // ── Draw the table ───────────────────────────────────────────────────
    write_border(writer, indent, &col_widths, TOP_LEFT, TOP_T, TOP_RIGHT)?;

    for (row_idx, row) in rows.iter().enumerate() {
        let is_header = row_idx == 0;

        write!(writer, "{indent}").context("write table indent")?;
        for (col, cell) in row.iter().enumerate() {
            write!(writer, "{VERTICAL} ").context("write cell border")?;
            let width = col_widths[col];
            let cell_width = UnicodeWidthStr::width(cell.as_str());
            let padding = width.saturating_sub(cell_width);

            if is_header {
                // Header: centered + bold.
                let left_pad = padding / 2;
                let right_pad = padding - left_pad;
                queue!(writer, SetAttribute(Attribute::Bold))?;
                write!(
                    writer,
                    "{:left_pad$}{cell}{:right_pad$}",
                    "",
                    "",
                    left_pad = left_pad,
                    right_pad = right_pad,
                )
                .context("write header cell")?;
                // Use NormalIntensity (SGR 22) instead of NoBold (SGR 21)
                // to avoid the "doubly underlined" misinterpretation on some
                // terminals.
                queue!(writer, SetAttribute(Attribute::NormalIntensity))?;
            } else {
                // Data: left-aligned (per spec; alignment from markdown
                // column markers is intentionally ignored for now).
                write!(writer, "{cell}{:padding$}", "", padding = padding)
                    .context("write data cell")?;
            }
            write!(writer, " ").context("write cell trailing space")?;
        }
        // Fill any missing cells in this row.
        for &width in col_widths.iter().skip(row.len()) {
            write!(writer, "{VERTICAL} {:width$} ", "").context("write empty cell")?;
        }
        writeln!(writer, "{VERTICAL}").context("write row end")?;

        // Separator after header and between every data row.
        if row_idx < rows.len() - 1 {
            write_border(writer, indent, &col_widths, LEFT_T, CROSS, RIGHT_T)?;
        }
    }

    write_border(
        writer,
        indent,
        &col_widths,
        BOTTOM_LEFT,
        BOTTOM_T,
        BOTTOM_RIGHT,
    )?;

    Ok(())
}

/// Write a horizontal border line: `left` + repeated `─` segments joined
/// by `middle`, ending with `right`.
fn write_border<W: Write>(
    writer: &mut W,
    indent: &str,
    col_widths: &[usize],
    left: &str,
    middle: &str,
    right: &str,
) -> Result<()> {
    write!(writer, "{indent}{left}").context("write border start")?;
    for (i, &w) in col_widths.iter().enumerate() {
        // +2 for the 1-space padding on each side of the cell content.
        let seg: String = HORIZONTAL.repeat(w + 2);
        write!(writer, "{seg}").context("write border segment")?;
        if i < col_widths.len() - 1 {
            write!(writer, "{middle}").context("write border middle")?;
        }
    }
    writeln!(writer, "{right}").context("write border end")?;
    Ok(())
}
