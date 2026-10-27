// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use markdown::mdast::{AlignKind, Node};

use crate::render::LongWords;
use crate::styled::{StyledLine, TextStyle};

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

const HEADER_STYLE: TextStyle =
	TextStyle { bold: true, italic: false, dim: false, strikethrough: false, underline: crate::styled::Underline::None, foreground: None };

/// Per-cell padding overhead on each line. Each cell contributes
/// `"│ "` on its left and a single space of right padding (the next
/// cell's `"│ "` doubles as this cell's right border). The final
/// closing `│` adds one more column to the whole row. These numbers
/// are used to translate between "available terminal width" and
/// "content width budget" for the column distribution pass.
const PER_CELL_OVERHEAD: usize = 3; // "│ " + trailing " "
const TRAILING_BORDER: usize = 1; // final "│"

/// A width no cell reaches: a cell laid out at it stays on one line.
const UNLIMITED: usize = usize::MAX;

// =========================================================
// Cells
// =========================================================

/// The inline content of each cell in a row.
fn row_cells(row: &Node) -> Vec<&[Node]> {
	row.children().into_iter().flatten().map(|cell| cell.children().map(Vec::as_slice).unwrap_or_default()).collect()
}

/// Display width of the widest of `lines`, 0 for none.
fn widest(lines: &[StyledLine]) -> usize {
	lines.iter().map(StyledLine::width).max().unwrap_or(0)
}

// =========================================================
// Table layout
// =========================================================

/// Lay out a GFM table as a list of styled lines. Each returned line is
/// ready to be appended by the caller *after* the caller emits its own
/// per-line prefix (nesting indent, blockquote bars, etc.) via its
/// normal `write_indent` machinery.
///
/// `available_width` is the number of display columns the caller is
/// willing to give the table (typically `Renderer::effective_width`
/// after subtracting nesting and blockquote prefixes). The layout pass
/// uses it to shrink wide tables down via proportional column
/// distribution and cell wrapping. A boxed table never breaks a word:
/// when a column would get narrower than the widest word of its cells,
/// the table is laid out as records instead (see [`layout_records`]).
///
/// `render_cell` lays out the inline content of a cell as running text
/// of the given width, so a cell keeps its inline styles and wraps where
/// running text would; its [`LongWords`] argument says whether a word
/// wider than that width overflows or breaks. Header cells add bold on
/// top of those styles. The caller lays its own context style over these
/// spans.
///
/// Layout rules of the boxed table:
///   - A column follows the alignment of its delimiter row (`:--` left,
///     `:-:` centered, `--:` right), header and data cells alike. A
///     column without one has its header **centered** and its data
///     left-aligned. Alignment applies per wrapped line.
///   - Header row text is **bold**.
///   - Cells have one space padding on each side.
///   - Horizontal separators appear between every pair of adjacent
///     rows.
///   - Column widths are sized to fit within `available_width`,
///     shrinking proportionally toward each column's minimum (widest
///     word that cannot break), see [`distribute_widths`].
///
/// An empty table (no rows) yields an empty `Vec`; the caller is
/// expected to treat that as "nothing to render".
pub fn layout_table(
	table: &markdown::mdast::Table,
	available_width: usize,
	render_cell: impl Fn(&[Node], usize, LongWords) -> Vec<StyledLine>,
) -> Vec<StyledLine> {
	let rows: Vec<Vec<&[Node]>> = table.children.iter().map(row_cells).collect();
	if rows.is_empty() {
		return Vec::new();
	}

	let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
	if num_cols == 0 {
		return Vec::new();
	}

	// ── Measure per-column natural and minimum widths ───────────────────
	// `natural`[i] = width of the widest cell in column i laid out on one
	//                line (what we'd use if space were unlimited).
	// `minimum`[i] = width of the widest word in any cell in column i
	//                that cannot break — the floor below which a word in
	//                the column would have to break. Laid out one column
	//                wide without breaking, a cell puts every such word on
	//                a line of its own.
	// Both are floored at 1 so that a column of empty cells still has a
	// visible, paddable slot.
	let mut natural: Vec<usize> = vec![0; num_cols];
	let mut minimum: Vec<usize> = vec![0; num_cols];
	for row in &rows {
		for (i, cell) in row.iter().enumerate() {
			natural[i] = natural[i].max(widest(&render_cell(cell, UNLIMITED, LongWords::Overflow)));
			minimum[i] = minimum[i].max(widest(&render_cell(cell, 1, LongWords::Overflow)));
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

	// ── Compute the column budget ───────────────────────────────────────
	// The table's total line width is sum(col_widths) + 3*num_cols + 1
	// (see PER_CELL_OVERHEAD / TRAILING_BORDER). Solve for how much
	// budget is available for cell content given the terminal width.
	// When the widest words do not fit it, some column would get narrower
	// than its widest word and that word would break: the table becomes
	// records instead.
	let border_overhead = PER_CELL_OVERHEAD * num_cols + TRAILING_BORDER;
	let content_budget = available_width.saturating_sub(border_overhead);
	if minimum.iter().sum::<usize>() > content_budget {
		return layout_records(&rows, num_cols, available_width, &render_cell);
	}
	let col_widths = distribute_widths(&natural, &minimum, content_budget);

	// ── Wrap each cell to its allocated column width ────────────────────
	// Each cell becomes its wrapped lines. Row height is the max cell
	// height; shorter cells, and cells that draw nothing, are rendered
	// blank on the extra lines so the whole row aligns vertically. No
	// column is narrower than its widest word, so no word overflows.
	let wrapped_rows: Vec<Vec<Vec<StyledLine>>> = rows
		.iter()
		.map(|row| {
			(0..num_cols)
				.map(|i| match row.get(i) {
					Some(cell) => render_cell(cell, col_widths[i], LongWords::Overflow),
					None => Vec::new(),
				})
				.collect()
		})
		.collect();

	// ── Build the lines ─────────────────────────────────────────────────
	let mut lines: Vec<StyledLine> = Vec::new();
	lines.push(border_line(&col_widths, TOP_LEFT, TOP_T, TOP_RIGHT));

	for (row_idx, wrapped) in wrapped_rows.iter().enumerate() {
		let row_height = wrapped.iter().map(Vec::len).max().unwrap_or(1).max(1);
		let is_header = row_idx == 0;
		for line_idx in 0..row_height {
			lines.push(row_line(wrapped, line_idx, &col_widths, is_header, &table.align));
		}
		if row_idx < wrapped_rows.len() - 1 {
			lines.push(border_line(&col_widths, LEFT_T, CROSS, RIGHT_T));
		}
	}

	lines.push(border_line(&col_widths, BOTTOM_LEFT, BOTTOM_T, BOTTOM_RIGHT));

	lines
}

// =========================================================
// Column width distribution
// =========================================================

/// Distribute `budget` columns across a set of columns, respecting
/// each column's minimum width and shrinking proportionally away from
/// its natural width. The minimums must fit `budget`; a table whose
/// minimums do not is laid out as records.
///
/// Rules:
/// 1. If the natural widths already fit within `budget`, return them
///    unchanged (fast path — tables that fit don't shrink).
/// 2. Otherwise, seed every column with its `minimum` width.
/// 3. Distribute `leftover = budget - sum(minimums)` on top of each
///    column proportionally to its "want" (`natural[i] - minimum[i]`).
///    Any rounding remainder is given out round-robin to columns still
///    below their natural width.
fn distribute_widths(natural: &[usize], minimum: &[usize], budget: usize) -> Vec<usize> {
	let num_cols = natural.len();
	debug_assert_eq!(num_cols, minimum.len());

	let natural_sum: usize = natural.iter().sum();
	if natural_sum <= budget {
		return natural.to_vec();
	}

	let minimum_sum: usize = minimum.iter().sum();
	debug_assert!(minimum_sum <= budget, "the minimums fit the budget");

	// At this point sum(minimum) <= budget < sum(natural). We have
	// `leftover` columns of budget above the minimums to distribute
	// among columns that "want" more (natural > minimum).
	let leftover = budget - minimum_sum;
	let wants: Vec<usize> = natural.iter().zip(minimum.iter()).map(|(n, m)| n - m).collect();
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
// Records
// =========================================================

/// The style of a record's names on top of their own styles: bold and
/// dim.
const NAME_STYLE: TextStyle =
	TextStyle { bold: true, italic: false, dim: true, strikethrough: false, underline: crate::styled::Underline::None, foreground: None };

/// The rule drawn across the width between two records.
const RULE: &str = HORIZONTAL;

/// The style of that rule: plain, as the borders of a boxed table.
const RULE_STYLE: TextStyle =
	TextStyle { bold: false, italic: false, dim: false, strikethrough: false, underline: crate::styled::Underline::None, foreground: None };

/// Columns between the widest name and the values beside it.
const RECORD_GAP: usize = 2;

/// Columns a value is indented by under its name.
const STACKED_INDENT: usize = 2;

/// Lay out a table that does not fit as a box as records, `width`
/// columns wide: one record per data row, one field per column, and a
/// rule across the width between records. A field is the column's name,
/// its header cell, and then the row's cell, the value.
///
/// Fields are laid out one of two ways, the same for every record of the
/// table:
///   - **Side by side**: the name, padded to the widest name, a gap, and
///     the value, wrapped in the room after the names; its further lines
///     start under its first.
///   - **Stacked**: the name on a line of its own, the value under it,
///     indented.
///
/// Side by side is used unless it would break a word or a character that
/// the stacked value keeps whole, or leaves no room at all. A word wider
/// than the stacked room breaks either way, so it does not decide.
///
/// An empty header cell, or a column past the header's last cell, has an
/// empty name; when every name is empty, values start at the left edge.
/// A field without a value is its name alone, so every record has the
/// same lines in the same order, and a field without a name or a value
/// is a blank line. A table without data rows is one record of its
/// names. Delimiter-row alignment does not apply.
///
/// A line is wider than `width` only where one unit a word breaks into
/// (see `break_units` in `render.rs`) is wider than the room it gets.
fn layout_records(
	rows: &[Vec<&[Node]>],
	num_cols: usize,
	width: usize,
	render_cell: &impl Fn(&[Node], usize, LongWords) -> Vec<StyledLine>,
) -> Vec<StyledLine> {
	let header = &rows[0];
	let records: Vec<Vec<&[Node]>> = if rows.len() > 1 { rows[1..].to_vec() } else { vec![Vec::new()] };

	// ── Names and the room beside them ──────────────────────────────────
	// A cell holds no line ending, so laid out without a width limit a
	// name takes at most one line.
	let names: Vec<StyledLine> = (0..num_cols)
		.map(|i| match header.get(i) {
			Some(cell) => render_cell(cell, UNLIMITED, LongWords::Overflow).into_iter().next().unwrap_or_default(),
			None => StyledLine::default(),
		})
		.collect();
	let name_width = widest(&names);
	let gap = if name_width == 0 { 0 } else { RECORD_GAP };
	let room = width.saturating_sub(name_width + gap);

	// ── Side by side or stacked ─────────────────────────────────────────
	// Laid out one column wide, a value puts every word on a line of its
	// own, and with breaking every unit a word breaks into. Side by side
	// would break such a word or unit, where the stacked value keeps it
	// whole, when it is wider than the room beside the names and no wider
	// than the room under them. `width` is at least 1, so the indent
	// leaves the stacked value at least one column.
	let indent = STACKED_INDENT.min(width.saturating_sub(1));
	let stacked_room = width - indent;
	let kept_whole_only_when_stacked = |piece: &StyledLine| {
		let piece_width = piece.width();
		room < piece_width && piece_width <= stacked_room
	};
	let stacked = room == 0
		|| records.iter().flatten().any(|cell| {
			[LongWords::Overflow, LongWords::Break]
				.into_iter()
				.flat_map(|long_words| render_cell(cell, 1, long_words))
				.any(|piece| kept_whole_only_when_stacked(&piece))
		});

	// ── Build the lines ─────────────────────────────────────────────────
	let mut lines: Vec<StyledLine> = Vec::new();
	for (record_idx, record) in records.iter().enumerate() {
		if record_idx > 0 {
			let mut rule = StyledLine::default();
			rule.push_str(&RULE.repeat(width), RULE_STYLE);
			lines.push(rule);
		}
		for (i, name) in names.iter().enumerate() {
			let value = record.get(i);
			if stacked {
				if let Some(cell) = header.get(i) {
					lines.extend(render_cell(cell, width, LongWords::Break).iter().map(record_name));
				}
				let value_lines = value.map(|cell| render_cell(cell, stacked_room, LongWords::Break)).unwrap_or_default();
				lines.extend(value_lines.iter().map(|line| indented(line, indent)));
			} else {
				let mut value_lines = value.map(|cell| render_cell(cell, room, LongWords::Break)).unwrap_or_default().into_iter();
				let mut first = record_name(name);
				if let Some(value_line) = value_lines.next() {
					let padding = name_width - name.width() + gap;
					first.push_str(&" ".repeat(padding), TextStyle::default());
					append(&mut first, &value_line);
				}
				lines.push(first);
				lines.extend(value_lines.map(|line| indented(&line, name_width + gap)));
			}
		}
	}
	lines
}

/// A name line of a record: its spans with [`NAME_STYLE`] on top.
fn record_name(name: &StyledLine) -> StyledLine {
	let mut line = StyledLine::default();
	for span in name {
		line.push_str(&span.text, span.style.patch(NAME_STYLE));
	}
	line
}

/// `line` behind `columns` unstyled blanks.
fn indented(line: &StyledLine, columns: usize) -> StyledLine {
	let mut indented = StyledLine::default();
	indented.push_str(&" ".repeat(columns), TextStyle::default());
	append(&mut indented, line);
	indented
}

/// Append the spans of `tail` to `line`.
fn append(line: &mut StyledLine, tail: &StyledLine) {
	for span in tail {
		line.push_str(&span.text, span.style);
	}
}

// =========================================================
// Line builders
// =========================================================

/// Build a single visual line of a wrapped row. `wrapped[col]` is the
/// list of wrapped lines for column `col`; `line_idx` is which of those
/// wrapped lines we're emitting on this visual line. A cell that is
/// shorter than the current row's height contributes an empty line on
/// the overflow lines.
///
/// `align` holds the alignment of each column from the delimiter row. A
/// row may have more cells than the delimiter row has columns; those
/// cells render as unaligned.
fn row_line(wrapped: &[Vec<StyledLine>], line_idx: usize, col_widths: &[usize], is_header: bool, align: &[AlignKind]) -> StyledLine {
	let plain = TextStyle::default();
	let empty = StyledLine::default();
	let mut line = StyledLine::default();

	for (col, &width) in col_widths.iter().enumerate() {
		// Left border of this cell (also doubles as the right border of
		// the previous cell) + one space of padding.
		line.push_str(VERTICAL, plain);
		line.push_str(" ", plain);

		let cell_line = wrapped[col].get(line_idx).unwrap_or(&empty);
		// No column is narrower than its widest word, so a line is never
		// wider than `width`.
		let padding = width.saturating_sub(cell_line.width());

		// A column without an alignment centers its header and puts its
		// data on the left. A centered cell gives the extra column of an
		// odd padding to the right.
		let alignment = align.get(col).copied().unwrap_or(AlignKind::None);
		let (left_pad, right_pad) = match (alignment, is_header) {
			(AlignKind::Right, _) => (padding, 0),
			(AlignKind::Center, _) | (AlignKind::None, true) => (padding / 2, padding - padding / 2),
			(AlignKind::Left, _) | (AlignKind::None, false) => (0, padding),
		};
		// A header cell is bold on top of its own styles, and its
		// alignment padding is part of the bold.
		let pad_style = if is_header { HEADER_STYLE } else { plain };
		line.push_str(&" ".repeat(left_pad), pad_style);
		for span in cell_line {
			let style = if is_header { span.style.patch(HEADER_STYLE) } else { span.style };
			line.push_str(&span.text, style);
		}
		line.push_str(&" ".repeat(right_pad), pad_style);

		// Trailing space of the cell's padding.
		line.push_str(" ", plain);
	}

	// Final right border of the row.
	line.push_str(VERTICAL, plain);
	line
}

/// Build a horizontal border line: `left` + repeated `─` segments joined
/// by `middle`, ending with `right`. Width of each segment is the cell
/// column width plus two — one for each side of the cell padding.
fn border_line(col_widths: &[usize], left: &str, middle: &str, right: &str) -> StyledLine {
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
	let mut styled = StyledLine::default();
	styled.push_str(&line, TextStyle::default());
	styled
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
	fn distribute_returns_minimum_when_it_matches_the_budget() {
		let natural = vec![10, 20, 30];
		let minimum = vec![5, 8, 12];
		assert_eq!(distribute_widths(&natural, &minimum, 25), vec![5, 8, 12]);
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
