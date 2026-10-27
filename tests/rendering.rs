// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Renderer tests on the line model: what `render_lines` lays out and how
//! each span is styled. The ANSI encoding of those lines is tested in
//! `ansi.rs`.

use insta::assert_snapshot;
use patine::{AnsiColor, Html, QuoteStyle, RenderOptions, SoftBreaks, StyledLine, TextStyle, Underline, render_lines, render_lines_with};

/// Render markdown at 80 columns into the canonical text form used by the
/// snapshots (see [`serialize`]).
fn render(markdown: &str) -> String {
	render_with_width(markdown, 80)
}

/// Like [`render`], but with a custom width.
fn render_with_width(markdown: &str, width: usize) -> String {
	serialize(&render_lines(markdown, width))
}

/// Write lines in a canonical text form: plain spans as their bare text,
/// styled spans as `[attributes]text[]` with the span's complete style.
/// Attributes appear in a fixed order (bold, dim, italic, underline or
/// double-underline, strike, color name), so every visual state has
/// exactly one spelling, and each line ends with a newline.
fn serialize(lines: &[StyledLine]) -> String {
	let mut out = String::new();
	for line in lines {
		for span in line {
			let attributes = attribute_names(span.style);
			if attributes.is_empty() {
				out.push_str(&span.text);
			} else {
				out.push_str(&format!("[{}]{}[]", attributes.join(","), span.text));
			}
		}
		out.push('\n');
	}
	out
}

fn attribute_names(style: TextStyle) -> Vec<&'static str> {
	let mut names = Vec::new();
	if style.bold {
		names.push("bold");
	}
	if style.dim {
		names.push("dim");
	}
	if style.italic {
		names.push("italic");
	}
	match style.underline {
		Underline::None => {}
		Underline::Single => names.push("underline"),
		Underline::Double => names.push("double-underline"),
	}
	if style.strikethrough {
		names.push("strike");
	}
	if let Some(color) = style.foreground {
		names.push(match color {
			AnsiColor::Black => "black",
			AnsiColor::Red => "red",
			AnsiColor::Green => "green",
			AnsiColor::Yellow => "yellow",
			AnsiColor::Blue => "blue",
			AnsiColor::Magenta => "magenta",
			AnsiColor::Cyan => "cyan",
			AnsiColor::White => "white",
		});
	}
	names
}

/// The plain text of every rendered line, without styling.
fn plain_lines(markdown: &str, width: usize) -> Vec<String> {
	render_lines(markdown, width).iter().map(ToString::to_string).collect()
}

/// Every character of `line` paired with the style it is drawn in.
fn styled_chars(line: &StyledLine) -> Vec<(char, TextStyle)> {
	line.spans.iter().flat_map(|span| span.text.chars().map(move |c| (c, span.style))).collect()
}

fn dim() -> TextStyle {
	let mut style = TextStyle::default();
	style.dim = true;
	style
}

// =========================================================
// Empty / whitespace input
// =========================================================

#[test]
fn empty_input() {
	assert_snapshot!(render(""), @"");
}

#[test]
fn whitespace_only_input() {
	assert_snapshot!(render("   "), @"");
}

// =========================================================
// Headings
// =========================================================

#[test]
fn h1_bold_italic_double_underline() {
	assert_snapshot!(render("# Hello World"));
}

#[test]
fn h2_bold_underline() {
	assert_snapshot!(render("## Section Title"));
}

#[test]
fn h3_bold() {
	assert_snapshot!(render("### Sub Section"));
}

#[test]
fn h4_bold() {
	assert_snapshot!(render("#### Deep Heading"));
}

#[test]
fn consecutive_headings() {
	assert_snapshot!(render("# Title\n\n## Section\n\n### Sub"));
}

#[test]
fn h1_with_inline_formatting() {
	assert_snapshot!(render("# Hello **bold** World"));
}

// =========================================================
// Paragraphs
// =========================================================

#[test]
fn simple_paragraph() {
	assert_snapshot!(render("Hello world"));
}

#[test]
fn two_paragraphs() {
	assert_snapshot!(render("First paragraph.\n\nSecond paragraph."));
}

#[test]
fn paragraph_after_heading() {
	assert_snapshot!(render("## Title\n\nSome body text here."));
}

// =========================================================
// Inline formatting
// =========================================================

#[test]
fn bold_text() {
	assert_snapshot!(render("This is **bold** text."));
}

#[test]
fn italic_text() {
	assert_snapshot!(render("This is *italic* text."));
}

#[test]
fn bold_italic_text() {
	assert_snapshot!(render("This is ***bold italic*** text."));
}

#[test]
fn mixed_inline() {
	assert_snapshot!(render("Start **bold** middle *italic* end."));
}

#[test]
fn adjacent_bold_italic() {
	assert_snapshot!(render("**bold***italic*"));
}

#[test]
fn bold_entire_paragraph() {
	assert_snapshot!(render("**Everything is bold here.**"));
}

#[test]
fn bold_with_trailing_space_before_closing() {
	// Trailing space before `**` prevents markdown from parsing as bold.
	// This must still render the `**` markers literally.
	assert_snapshot!(render("**not bold **"));
}

#[test]
fn bold_without_trailing_space() {
	// Without trailing space, markdown correctly parses as bold.
	assert_snapshot!(render("**is bold**"));
}

// =========================================================
// Word wrapping
// =========================================================

#[test]
fn wraps_at_79_columns() {
	// 80+ characters of content to trigger a wrap.
	let long = "word ".repeat(20); // 100 chars of "word word word ..."
	assert_snapshot!(render(&long));
}

#[test]
fn single_long_word_no_break() {
	// A single word longer than 79 chars should not be broken (no hyphenation).
	let word = "a".repeat(100);
	assert_snapshot!(render(&word));
}

#[test]
fn wide_characters_wrap_by_display_width() {
	// `日本語` is 6 columns wide in 9 bytes: it overflows a 4-column line
	// on its own, and `ok` cannot join it.
	assert_eq!(render_with_width("日本語 ok", 4), "日本語\nok\n");
}

#[test]
fn wrap_preserves_bold() {
	// Bold text that spans across a line wrap should re-apply bold on the
	// new line.
	let long = format!("**{}**", "bold ".repeat(20).trim());
	assert_snapshot!(render(&long));
}

#[test]
fn wrap_preserves_nested_styles() {
	// Bold + italic spanning a wrap: both should be re-applied.
	let long = format!("***{}***", "styled ".repeat(15).trim());
	assert_snapshot!(render(&long));
}

#[test]
fn wrap_mid_paragraph_with_mixed_inline() {
	let text = format!("Before **bold section that is {} enough to wrap** after.", "quite long ".repeat(8));
	assert_snapshot!(render(&text));
}

// =========================================================
// Soft breaks
// =========================================================

#[test]
fn soft_break_becomes_space() {
	// A single newline in the source (not preceded by two spaces) is a soft
	// break, which should render as a space between words.
	assert_snapshot!(render("first\nsecond"));
}

// =========================================================
// Inline code
// =========================================================

#[test]
fn inline_code_colored() {
	assert_snapshot!(render("Use `println!` to print."));
}

#[test]
fn inline_code_in_bold_context() {
	assert_snapshot!(render("**bold `code` bold**"));
}

#[test]
fn multiple_inline_code_spans() {
	assert_snapshot!(render("Use `foo` and `bar` together."));
}

// =========================================================
// Strikethrough
// =========================================================

#[test]
fn strikethrough_text() {
	assert_snapshot!(render("This is ~~deleted~~ text."));
}

#[test]
fn strikethrough_with_bold() {
	assert_snapshot!(render("~~**bold and struck**~~"));
}

// =========================================================
// Links
// =========================================================

#[test]
fn link_with_text() {
	assert_snapshot!(render("[click here](https://example.com)"));
}

#[test]
fn link_in_sentence() {
	assert_snapshot!(render("See the [documentation](https://example.com/docs) for details."));
}

#[test]
fn bare_autolink() {
	assert_snapshot!(render("<https://example.com>"));
}

#[test]
fn link_with_bold_text() {
	assert_snapshot!(render("[**bold link**](https://example.com)"));
}

// A link whose text is its URL already shows the URL, so it gets no
// `(url)` suffix. An email autolink's URL is its text behind `mailto:`.

#[test]
fn autolinks_show_their_url_once() {
	assert_eq!(render("See <https://example.com> now."), "See [underline]https://example.com[] now.\n");
	assert_eq!(render("Mail <someone@example.com>."), "Mail [underline]someone@example.com[].\n");
	assert_eq!(render("Mail <mailto:someone@example.com>."), "Mail [underline]mailto:someone@example.com[].\n");
}

#[test]
fn literal_autolinks_show_their_url_once() {
	assert_eq!(
		render("See https://example.com or someone@example.com"),
		"See [underline]https://example.com[] or [underline]someone@example.com[]\n"
	);
}

#[test]
fn link_named_after_its_url_shows_it_once() {
	assert_eq!(render("[https://example.com](https://example.com)"), "[underline]https://example.com[]\n");
	assert_eq!(render("[**https://example.com**](https://example.com)"), "[bold,underline]https://example.com[]\n");
}

#[test]
fn link_whose_text_differs_from_its_url_keeps_the_suffix() {
	assert_eq!(render("[docs](https://example.com)"), "[underline]docs[][dim] (https://example.com)[]\n");
	assert_eq!(render("[https://example.com](https://example.org)"), "[underline]https://example.com[][dim] (https://example.org)[]\n");
	// A `www.` literal autolink gets `http://` in front of its URL.
	assert_eq!(render("See www.example.com"), "See [underline]www.example.com[][dim] (http://www.example.com)[]\n");
	// Only `mailto:` is left out of the comparison.
	assert_eq!(render("[example.com](https://example.com)"), "[underline]example.com[][dim] (https://example.com)[]\n");
	// Raw HTML counts as text where it is drawn.
	assert_eq!(
		render_with_options("[<b>https://x.org</b>](https://x.org)", 80, LITERAL),
		"[underline]<b>https://x.org</b>[][dim] (https://x.org)[]\n"
	);
}

#[test]
fn link_named_after_its_url_around_dropped_html_shows_it_once() {
	// With `Html::Drop` the tags are not drawn, so the link text that
	// shows is its URL.
	assert_eq!(render("[<b>https://x.org</b>](https://x.org)"), "[underline]https://x.org[]\n");
	assert_eq!(render("[**<i>https://x.org</i>**](https://x.org)"), "[bold,underline]https://x.org[]\n");
}

// =========================================================
// Regression: link underline must not bleed into surrounding space
// =========================================================

#[test]
fn link_space_before_not_underlined() {
	// The space between preceding text and the link must not be underlined.
	let lines = render_lines("word [link](https://x.com) word", 80);
	let spans = &lines[0].spans;
	// The underline must start at "link", not at the space before it.
	assert_eq!(spans[0].text, "word ", "{spans:?}");
	assert_eq!(spans[0].style, TextStyle::default());
	assert_eq!(spans[1].text, "link", "{spans:?}");
	assert_eq!(spans[1].style.underline, Underline::Single);
}

#[test]
fn space_before_an_inline_element_is_unstyled() {
	// The space that separates a word from the inline element after it
	// belongs to the surrounding text, so it is not struck through,
	// colored or bold along with the element.
	let lines = render_lines("Run `cargo` or ~~not~~ **this** *now*", 80);
	let chars = styled_chars(&lines[0]);
	for (i, &(c, style)) in chars.iter().enumerate() {
		if c == ' ' {
			assert_eq!(style, TextStyle::default(), "space at column {i} is styled\n{}", serialize(&lines));
		}
	}
}

#[test]
fn spaces_inside_an_inline_element_keep_its_style() {
	let lines = render_lines("a ~~two words~~ b", 80);
	let struck: Vec<&str> = lines[0].spans.iter().filter(|span| span.style.strikethrough).map(|span| span.text.as_str()).collect();
	assert_eq!(struck, ["two words"]);
}

#[test]
fn link_text_is_underlined() {
	let lines = render_lines("[click here](https://example.com)", 80);
	let link = &lines[0].spans[0];
	assert_eq!(link.text, "click here");
	assert_eq!(link.style.underline, Underline::Single);
}

#[test]
fn long_link_url_wraps_onto_new_line() {
	// A URL that won't fit on the current line alongside the link text
	// should move to its own line. The URL itself must never be broken
	// mid-string (it may overflow on its own line — overflow is preferable
	// to making a URL uncopyable).
	let url = "https://example.com/some/very/long/path/that/exceeds";
	let lines = plain_lines(&format!("[link]({url})"), 40);
	let output = lines.join("\n");
	// The link text "link" should be on one line; the URL suffix "(url)"
	// should begin a different line because it did not fit next to "link".
	let link_line = lines.iter().find(|l| l.contains("link")).expect("must contain link text");
	assert!(!link_line.contains(url), "URL must not follow link text on the same line when it doesn't fit:\nline: {link_line}\nfull:\n{output}");
	// The URL must still appear intact on some line.
	assert!(output.contains(url), "URL must not be broken mid-string:\n{output}");
}

#[test]
fn long_image_url_wraps_onto_new_line() {
	let url = "https://example.com/some/very/long/path/that/exceeds.png";
	let lines = plain_lines(&format!("![alt]({url})"), 40);
	let output = lines.join("\n");
	let alt_line = lines.iter().find(|l| l.contains("[image: alt]")).expect("must contain image alt marker");
	assert!(!alt_line.contains(url), "URL must not follow image marker on the same line when it doesn't fit:\nline: {alt_line}\nfull:\n{output}");
	assert!(output.contains(url), "URL must not be broken mid-string:\n{output}");
}

// =========================================================
// Regression: code blocks must have extra 2-space indent
// =========================================================

#[test]
fn code_block_extra_indent() {
	// Code blocks should be indented 2 spaces (one nesting level).
	let lines = plain_lines("```\ncode\n```", 80);
	assert_eq!(lines, ["  code"]);
}

// =========================================================
// Regression: list must have blank line separation from preceding block
// =========================================================

#[test]
fn list_has_blank_line_after_heading() {
	let lines = plain_lines("## Features\n\n- Item one\n- Item two", 80);
	let output = lines.join("\n");
	// Line 0: heading, Line 1: blank, Line 2: first list item.
	assert!(lines.len() >= 3, "expected at least 3 lines (heading, blank, list item)");
	assert!(
		lines[1].trim().is_empty(),
		"there must be a blank line between heading and first list item.\nLine 1: '{}'\nFull output:\n{output}",
		lines[1]
	);
}

#[test]
fn list_has_blank_line_after_paragraph() {
	let lines = plain_lines("Some text.\n\n- Item one\n- Item two", 80);
	let output = lines.join("\n");
	assert!(lines.len() >= 3, "expected at least 3 lines (paragraph, blank, list item)");
	assert!(
		lines[1].trim().is_empty(),
		"there must be a blank line between paragraph and first list item.\nLine 1: '{}'\nFull output:\n{output}",
		lines[1]
	);
}

// =========================================================
// Ordered lists
// =========================================================

#[test]
fn ordered_list_basic() {
	assert_snapshot!(render("1. First\n2. Second\n3. Third"));
}

#[test]
fn ordered_list_with_bold() {
	assert_snapshot!(render("1. **Item one** — description\n2. **Item two** — description"));
}

#[test]
fn ordered_list_custom_start() {
	assert_snapshot!(render("5. Fifth\n6. Sixth"));
}

#[test]
fn ordered_list_after_heading() {
	assert_snapshot!(render("## Section\n\n1. First\n2. Second"));
}

// =========================================================
// Unordered lists
// =========================================================

#[test]
fn unordered_list_basic() {
	assert_snapshot!(render("- Alpha\n- Beta\n- Gamma"));
}

#[test]
fn unordered_list_with_inline_code() {
	assert_snapshot!(render("- Use `foo`\n- Use `bar`"));
}

// =========================================================
// Task lists
// =========================================================

#[test]
fn open_task_shows_an_empty_box() {
	assert_eq!(render("- [ ] open"), "☐ open\n");
}

#[test]
fn done_task_shows_a_checked_box_and_dims_its_line() {
	assert_eq!(render("- [x] done"), "[dim]☑ done[]\n");
	assert_eq!(render("- [X] done"), "[dim]☑ done[]\n");
}

#[test]
fn numbered_task_keeps_its_number_before_the_box() {
	// The number of a done task dims with the rest of its line.
	assert_eq!(render("1. [ ] open\n2. [x] done"), "1. ☐ open\n[dim]2. ☑ done[]\n");
}

#[test]
fn task_text_wraps_past_the_box() {
	assert_eq!(plain_lines("- [ ] aaaa bbbb cccc", 11), ["☐ aaaa bbbb", "  cccc"]);
	assert_eq!(plain_lines("1. [x] aaaa bbbb", 12), ["1. ☑ aaaa", "     bbbb"]);
}

#[test]
fn items_under_a_done_task_keep_their_own_style() {
	assert_eq!(render("- [x] done\n  - [ ] open\n  - plain"), "[dim]☑ done[]\n  ☐ open\n  • plain\n");
}

#[test]
fn blocks_of_a_done_task_are_dim_but_its_sub_items_are_not() {
	// Code tokens keep their color under the dim; a later paragraph of
	// the item is dim as well.
	assert_eq!(
		render("- [x] Ship it\n\n  ```\n  cargo publish\n  ```\n\n  Then tag.\n\n  - [ ] notes"),
		"[dim]☑ Ship it[]\n\n    [dim,yellow]cargo publish[]\n\n  [dim]Then tag.[]\n  ☐ notes\n"
	);
}

#[test]
fn tasks_and_plain_items_mix() {
	assert_eq!(plain_lines("- [x] a\n- b\n- [ ] c", 80), ["☑ a", "• b", "☐ c"]);
}

#[test]
fn task_in_a_quote_keeps_the_bar_style() {
	assert_eq!(render("> - [x] q"), "[cyan]│ [][dim]☑ q[]\n");
}

#[test]
fn an_item_with_only_a_check_is_no_task() {
	// markdown-rs wants text after the check, so this stays a plain item.
	assert_eq!(plain_lines("- [x]", 80), ["• [x]"]);
}

#[test]
fn task_markers_are_one_column_wide() {
	// The hang of a task item counts on this.
	use unicode_width::UnicodeWidthStr;
	for marker in ["☐", "☑", "•"] {
		assert_eq!(UnicodeWidthStr::width(marker), 1, "{marker}");
	}
}

// =========================================================
// Nested lists
// =========================================================

#[test]
fn nested_unordered_list() {
	assert_snapshot!(render("- Top\n  - Nested\n    - Deep\n  - Back\n- Top again"));
}

#[test]
fn ordered_inside_unordered() {
	assert_snapshot!(render("- Item\n  1. Sub one\n  2. Sub two\n- Item"));
}

#[test]
fn unordered_inside_ordered() {
	assert_snapshot!(render("1. Item\n   - Sub a\n   - Sub b\n2. Item"));
}

// =========================================================
// Tables
// =========================================================

#[test]
fn table_basic() {
	assert_snapshot!(render("| Level | Items |\n|---|---|\n| High | Fix bugs |\n| Low | Cleanup |"));
}

#[test]
fn table_header_centered() {
	// Header text should be centered within the column width.
	assert_snapshot!(render("| Name | Description |\n|---|---|\n| a | Short |"));
}

#[test]
fn table_with_bold_cells() {
	assert_snapshot!(render("| Level | Items |\n|---|---|\n| **Blocker** | Auth system |"));
}

#[test]
fn table_single_column() {
	assert_snapshot!(render("| Item |\n|---|\n| One |\n| Two |"));
}

#[test]
fn table_after_heading() {
	assert_snapshot!(render("## Priority\n\n| Level | Items |\n|---|---|\n| High | Bugs |"));
}

// =========================================================
// Regression: tables inside blockquotes must carry blockquote bars
// =========================================================

#[test]
fn table_in_blockquote_has_bars() {
	// Every line of a table rendered inside a blockquote must start
	// with the blockquote `│ ` bar. The table itself uses `│` for cell
	// borders, so we must check the line prefix — not just whether `│`
	// appears anywhere.
	let lines = plain_lines("> | col1 | col2 |\n> |------|------|\n> | a    | b    |", 80);
	let output = lines.join("\n");
	let table_line_markers = ["┌", "│ col1", "├", "│ a ", "└"];
	for marker in table_line_markers {
		let line = lines.iter().find(|l| l.contains(marker)).unwrap_or_else(|| panic!("missing table line with {marker:?}\nfull:\n{output}"));
		assert!(line.starts_with("│ "), "line containing {marker:?} must start with a blockquote bar:\nline: {line:?}\nfull:\n{output}");
	}
}

#[test]
fn table_in_nested_blockquote_has_double_bars() {
	// Two levels of blockquote nesting should produce two bar prefixes
	// on every table line.
	let lines = plain_lines("> > | a | b |\n> > |--|--|\n> > | 1 | 2 |", 80);
	let output = lines.join("\n");
	let table_line_markers = ["┌", "│ a", "├", "│ 1", "└"];
	for marker in table_line_markers {
		let line = lines.iter().find(|l| l.contains(marker)).unwrap_or_else(|| panic!("missing table line with {marker:?}\nfull:\n{output}"));
		assert!(line.starts_with("│ │ "), "line containing {marker:?} must start with two blockquote bars:\nline: {line:?}\nfull:\n{output}");
	}
}

#[test]
fn table_after_blockquote_still_works() {
	// A table immediately following a blockquote (not inside it) must
	// render normally with no bars. This guards against the new
	// per-line `write_indent` loop accidentally picking up stale state.
	assert_snapshot!(render("> a quote\n\n| col |\n|---|\n| val |"));
}

#[test]
fn table_in_blockquote_snapshot() {
	// Full visual pin for a table inside a single-level blockquote.
	// Complements the per-line bar assertions above by locking in the
	// exact expected output shape (bar prefix, box drawing, bold header,
	// data cells) so future refactors cannot silently change it.
	assert_snapshot!(render("> | col1 | col2 |\n> |------|------|\n> | a    | b    |"));
}

#[test]
fn table_in_nested_blockquote_snapshot() {
	// Same, for two levels of blockquote nesting.
	assert_snapshot!(render("> > | a | b |\n> > |--|--|\n> > | 1 | 2 |"));
}

// =========================================================
// Table cell wrapping and width-aware column layout (phase 2)
// =========================================================

#[test]
fn table_wraps_long_cell_text() {
	// A table with a long sentence in one cell, rendered at a width too
	// narrow to fit naturally. After wrapping, no output line should
	// exceed the target width and every word from the input must still
	// appear somewhere in the rendered output.
	let md = "\
| Topic | Notes |
|---|---|
| cats | The quick brown fox jumps over the lazy dog repeatedly |";
	let lines = render_lines(md, 30);
	let output = serialize(&lines);
	for line in &lines {
		assert!(line.width() <= 30, "line exceeds width 30: {line}\nfull:\n{output}");
	}
	for word in ["quick", "brown", "fox", "jumps", "over", "the", "lazy", "dog", "repeatedly"] {
		assert!(output.contains(word), "missing word {word:?} after wrapping:\n{output}");
	}
}

#[test]
fn table_preserves_column_proportions() {
	// Three columns with very different natural widths. Their sum far
	// exceeds a 40-column budget, so the table must shrink. After
	// shrinking, (a) the overall table must fit in 40 columns and
	// (b) the column containing longer content must still be wider
	// than the one containing shorter content — proportions are
	// respected, not flattened.
	let md = "\
| A | Middle column | Much longer final column content |
|---|---|---|
| x | medium text | a lot more text here to pad it out |";
	let lines = render_lines(md, 40);
	let output = serialize(&lines);
	// Every line must fit in 40 columns — this is the primary "shrank"
	// assertion that fails without width-aware layout.
	for line in &lines {
		assert!(line.width() <= 40, "line exceeds width 40: {line}\nfull:\n{output}");
	}
	// Find the top border line (deterministic shape): `┌─...┬─...┬─...┐`.
	let border = lines.iter().map(ToString::to_string).find(|l| l.starts_with('┌')).expect("expected top border");
	// Segment widths are the number of `─` runs between the corners/T
	// junctions. Split on the junction characters to get each column's
	// segment (including its +2 padding).
	let segments: Vec<usize> =
		border.trim_start_matches('┌').trim_end_matches('┐').split('┬').map(|seg| seg.chars().filter(|&c| c == '─').count()).collect();
	assert_eq!(segments.len(), 3, "expected 3 columns, got {segments:?}");
	// Column 3 (longest natural width) must remain widest; column 1
	// (shortest) must remain narrowest, even after shrinking.
	assert!(segments[2] > segments[1], "column 3 should be wider than column 2: {segments:?}");
	assert!(segments[1] > segments[0], "column 2 should be wider than column 1: {segments:?}");
}

#[test]
fn table_header_centered_with_wrapping() {
	// A long header name forced to wrap to a narrow column. The header
	// cell must span more than one visual line (= wrapping happened),
	// and every header word must be bold, on every wrapped line.
	let md = "\
| Very Long Header Name | x |
|---|---|
| a | b |";
	let lines = render_lines(md, 24);
	let output = serialize(&lines);

	// Count distinct header-content lines: lines that start with the
	// table's left border `│` and contain any of the header words.
	let header_words = ["Very", "Long", "Header", "Name"];
	let header_line_count =
		lines.iter().map(ToString::to_string).filter(|l| l.starts_with('│') && header_words.iter().any(|w| l.contains(w))).count();
	assert!(header_line_count >= 2, "header should wrap to ≥2 lines, saw {header_line_count}:\n{output}");

	// Every header word must sit inside a bold span.
	for word in header_words {
		let found = lines.iter().flatten().any(|span| span.style.bold && span.text.contains(word));
		assert!(found, "header word {word:?} must appear bold:\n{output}");
	}
}

#[test]
fn table_in_blockquote_with_wrapping() {
	// Phase 1 × Phase 2: a table inside a blockquote at narrow width.
	// Every line must still carry the blockquote bar prefix, and the
	// cells must be wrapped so the whole structure fits within the
	// available (content minus bar) width.
	let md = "\
> | Topic | Notes |
> |---|---|
> | cats | The quick brown fox jumps over the lazy dog |";
	let lines = render_lines(md, 36);
	let output = serialize(&lines);
	// Every non-border table line and every border line must begin
	// with the blockquote bar.
	for line in &lines {
		let visible = line.to_string();
		if visible.is_empty() {
			continue;
		}
		// Identify lines that belong to the table by the box-drawing
		// characters; they must all start with the blockquote bar.
		let is_table_line = ['┌', '├', '└', '│'].iter().any(|c| visible.contains(*c));
		if is_table_line {
			assert!(visible.starts_with("│ "), "table line inside blockquote missing bar prefix:\nline: {visible:?}\nfull:\n{output}");
		}
	}
	// The table must fit within 36 columns.
	for line in &lines {
		assert!(line.width() <= 36, "line exceeds width 36: {line}\nfull:\n{output}");
	}
}

#[test]
fn table_variable_row_heights() {
	// One cell wraps to multiple lines while its sibling is short. The
	// row must be tall enough for the wrapped cell, with the short
	// sibling blank-padded on the extra lines (not collapsed).
	let md = "\
| Tall | Short |
|---|---|
| one two three four five six | x |";
	// Count the number of content lines between the header separator
	// and the bottom border by looking for lines that begin with the
	// table's left border `│` and contain the short "x" cell or any
	// wrapped word of the tall cell.
	let lines = plain_lines(md, 22);
	let output = lines.join("\n");
	// Indices of separator and bottom border anchor the data region.
	let sep_idx = lines.iter().position(|l| l.starts_with('├')).expect("separator");
	let bot_idx = lines.iter().position(|l| l.starts_with('└')).expect("bottom border");
	// The data row should span more than one content line because the
	// tall cell wrapped.
	let row_lines = &lines[sep_idx + 1..bot_idx];
	assert!(row_lines.len() >= 2, "expected variable-height row to span ≥2 lines, got {}: {row_lines:#?}", row_lines.len());
	// The "x" must appear on at least one of those lines.
	assert!(row_lines.iter().any(|l| l.contains(" x ")), "short cell 'x' must appear on at least one row line:\n{output}");
}

// =========================================================
// Table column alignment
// =========================================================

/// The rows of a rendered table that hold cell text, without the border
/// and separator lines.
fn table_rows(markdown: &str, width: usize) -> Vec<String> {
	plain_lines(markdown, width).into_iter().filter(|line| line.starts_with('│')).collect()
}

#[test]
fn aligned_columns_follow_the_delimiter_row() {
	// A centred cell puts the odd column of its padding on the right.
	assert_eq!(
		table_rows("| left | centre | right | none |\n|:--|:-:|--:|---|\n| a | b | c | d |", 80),
		["│ left │ centre │ right │ none │", "│ a    │   b    │     c │ d    │"]
	);
}

#[test]
fn header_of_an_aligned_column_follows_its_alignment() {
	assert_eq!(
		table_rows("| h | h | h | h |\n|:--|:-:|--:|---|\n| aaaaa | bbbbb | ccccc | ddddd |", 80),
		["│ h     │   h   │     h │   h   │", "│ aaaaa │ bbbbb │ ccccc │ ddddd │"]
	);
}

#[test]
fn unaligned_columns_keep_centred_header_and_left_data() {
	assert_eq!(table_rows("| h | head |\n|---|---|\n| aaaaa | a |", 80), ["│   h   │ head │", "│ aaaaa │ a    │"]);
}

#[test]
fn wrapped_cell_lines_align_each_on_their_own() {
	assert_eq!(table_rows("| x |\n|--:|\n| aaaa bb |", 8), ["│    x │", "│ aaaa │", "│   bb │"]);
}

#[test]
fn header_padding_of_an_aligned_column_is_bold() {
	let lines = render_lines("| h |\n|:--|\n| aaaaa |", 80);
	let header: Vec<(char, TextStyle)> = styled_chars(&lines[1])[2..7].to_vec();
	let mut bold = TextStyle::default();
	bold.bold = true;
	assert_eq!(header, [('h', bold), (' ', bold), (' ', bold), (' ', bold), (' ', bold)], "{}", serialize(&lines));
}

#[test]
fn cells_past_the_delimiter_row_render_unaligned() {
	// markdown-rs keeps a cell that has no column in the delimiter row,
	// and the table gives it a column without an alignment.
	assert_eq!(table_rows("| aaa |\n|--:|\n| b | c |", 80), ["│ aaa │   │", "│   b │ c │"]);
}

// =========================================================
// Inline content in table cells
// =========================================================

// A cell is laid out as running text: it keeps the styles of its inline
// content and wraps by the same rules. A header cell adds bold on top.

/// The rows of a rendered table that hold cell text, in the canonical
/// styled form of [`serialize`].
fn styled_table_rows(markdown: &str, width: usize) -> Vec<String> {
	serialize(&render_lines(markdown, width)).lines().filter(|line| line.starts_with('│')).map(str::to_string).collect()
}

#[test]
fn table_cells_keep_their_inline_styles() {
	let row = "| **b** | *i* | `c` | ~~s~~ |";
	assert_eq!(
		styled_table_rows(&format!("{row}\n|---|---|---|---|\n{row}"), 80),
		["│ [bold]b[] │ [bold,italic]i[] │ [bold,yellow]c[] │ [bold,strike]s[] │", "│ [bold]b[] │ [italic]i[] │ [yellow]c[] │ [strike]s[] │",]
	);
}

#[test]
fn link_in_a_table_cell_renders_as_in_running_text() {
	let link = "[docs](https://x.org)";
	let paragraph = render(link);
	assert_eq!(paragraph, "[underline]docs[][dim] (https://x.org)[]\n");
	assert_eq!(styled_table_rows(&format!("| h |\n|---|\n| {link} |"), 80)[1], format!("│ {} │", paragraph.trim_end()));
	// A link whose text is its URL gets no suffix, in a cell as anywhere.
	assert_eq!(styled_table_rows("| h |\n|---|\n| <https://x.org> |", 80)[1], "│ [underline]https://x.org[] │");
}

#[test]
fn image_in_a_table_cell_shows_its_marker() {
	let image = "![alt](p.png)";
	let paragraph = render(image);
	assert_eq!(paragraph, "[image: alt][dim] (p.png)[]\n");
	assert_eq!(styled_table_rows(&format!("| h |\n|---|\n| {image} |"), 80)[1], format!("│ {} │", paragraph.trim_end()));
}

#[test]
fn glued_pieces_in_a_cell_wrap_as_one_word() {
	// Width 8 leaves 4 columns for the cell, the width of `bbcc`, which
	// cannot break.
	assert_eq!(styled_table_rows("| hdr |\n|---|\n| aa **bb**cc |", 8), ["│ [bold]hdr [] │", "│ aa   │", "│ [bold]bb[]cc │"]);
}

#[test]
fn link_url_in_a_narrow_cell_moves_to_its_own_line() {
	// Width 19 leaves 15 columns for the cell, the width of the URL in
	// its parentheses, so the URL goes under the link text. Its dim space
	// stays behind: a line does not start with a space.
	assert_eq!(
		styled_table_rows("| h |\n|---|\n| [docs](https://x.org) |", 19)[1..],
		["│ [underline]docs[]            │", "│ [dim](https://x.org)[] │"]
	);
}

#[test]
fn styled_cell_aligns_by_display_width() {
	assert_eq!(styled_table_rows("| head |\n|--:|\n| *x* |", 80)[1], "│    [italic]x[] │");
}

#[test]
fn whitespace_runs_in_a_cell_do_not_widen_its_column() {
	let expected = ["│ a b │ c │", "│ x   │ y │"];
	assert_eq!(table_rows("| a    b | c |\n|---|---|\n| x | y |", 80), expected);
	// Inline HTML that is not drawn takes no room either.
	assert_eq!(table_rows("| a <b></b> b | c |\n|---|---|\n| x | y |", 80), expected);
}

// =========================================================
// Tables laid out as records
// =========================================================

// A table whose columns would get narrower than the widest word of their
// cells is laid out as records: one record per data row, one line per
// column with its name from the header row and then the cell's text, and
// a rule across the width between records. A boxed table never breaks a
// word.

/// Eight columns of short words, which need 66 columns as a boxed table.
const EIGHT_COLUMNS: &str = "\
| step | owner | doc | due | state | prio | team | area |
|--|--|--|--|--|--|--|--|
| retry | jaw | policy | 11-02 | open | high | core | queue |
| backoff | ana | notes | 11-09 | done | low | tui | pane |";

#[test]
fn table_lays_out_as_records_when_a_word_would_break() {
	assert_eq!(
		plain_lines(EIGHT_COLUMNS, 22),
		[
			"step   retry",
			"owner  jaw",
			"doc    policy",
			"due    11-02",
			"state  open",
			"prio   high",
			"team   core",
			"area   queue",
			"──────────────────────",
			"step   backoff",
			"owner  ana",
			"doc    notes",
			"due    11-09",
			"state  done",
			"prio   low",
			"team   tui",
			"area   pane",
		]
	);
}

#[test]
fn records_start_where_a_column_would_get_narrower_than_its_widest_word() {
	let md = "| a | b |\n|--|--|\n| xxxxx | yyyyy |";
	// The widest words take 10 columns and the borders 7.
	assert!(plain_lines(md, 17)[0].starts_with('┌'));
	assert_eq!(plain_lines(md, 16), ["a  xxxxx", "b  yyyyy"]);
}

#[test]
fn record_names_are_bold_and_dim_over_their_own_styles() {
	assert_eq!(serialize(&render_lines("| *n* | m |\n|--|--|\n| v | w |", 6)), "[bold,dim,italic]n[]  v\n[bold,dim]m[]  w\n");
}

#[test]
fn a_long_value_wraps_under_its_value() {
	assert_eq!(
		plain_lines("| step | note |\n|--|--|\n| retry | waits for the queue to drain before it tries again |", 16),
		["step  retry", "note  waits for", "      the queue", "      to drain", "      before it", "      tries", "      again",]
	);
}

#[test]
fn records_stack_when_side_by_side_would_break_a_word() {
	// Beside the names, 5 columns are left: `policy` and `backoff` would
	// break there, and fit under their names.
	assert_eq!(
		plain_lines(EIGHT_COLUMNS, 12),
		[
			"step",
			"  retry",
			"owner",
			"  jaw",
			"doc",
			"  policy",
			"due",
			"  11-02",
			"state",
			"  open",
			"prio",
			"  high",
			"team",
			"  core",
			"area",
			"  queue",
			"────────────",
			"step",
			"  backoff",
			"owner",
			"  ana",
			"doc",
			"  notes",
			"due",
			"  11-09",
			"state",
			"  done",
			"prio",
			"  low",
			"team",
			"  tui",
			"area",
			"  pane",
		]
	);
}

#[test]
fn records_stack_when_side_by_side_would_break_a_wide_character() {
	// Beside the name, one column is left, narrower than each character.
	assert_eq!(plain_lines("| Beschreibung |\n|---|\n| 日本語のテキストです |", 15), ["Beschreibung", "  日本語のテキ", "  ストです"]);
}

/// A long URL in a cell, which needs 66 columns as a boxed table.
const LONG_URL_TABLE: &str = "| step | doc |\n|---|---|\n| retry | see [the policy](https://example.com/handbook/operations/retry-policy) |";

#[test]
fn a_value_word_wider_than_any_room_breaks_beside_its_name() {
	// The URL is wider than the full width as well, so it breaks beside
	// its name rather than under it.
	assert_eq!(
		plain_lines(LONG_URL_TABLE, 40),
		["step  retry", "doc   see the policy", "      (https://example.com/handbook/oper", "      ations/retry-policy)",]
	);
	for line in render_lines(LONG_URL_TABLE, 40) {
		assert!(line.width() <= 40, "{line}");
	}
}

#[test]
fn a_broken_url_in_a_record_stays_dim() {
	assert_eq!(
		serialize(&render_lines(LONG_URL_TABLE, 40)).lines().skip(2).collect::<Vec<_>>(),
		["      [dim](https://example.com/handbook/oper[]", "      [dim]ations/retry-policy)[]",]
	);
}

#[test]
fn a_broken_name_keeps_its_style() {
	// No room is left beside the name, so it stands on lines of its own.
	assert_eq!(serialize(&render_lines("| aaaaaaaaaa |\n|---|\n| b |", 9)), "[bold,dim]aaaaaaaaa[]\n[bold,dim]a[]\n  b\n");
}

#[test]
fn an_empty_header_cell_gives_an_empty_name() {
	assert_eq!(plain_lines("| a | | c |\n|--|--|--|\n| 1 | 2 | 3 |", 10), ["a  1", "   2", "c  3"]);
}

#[test]
fn a_cell_past_the_header_has_an_empty_name() {
	assert_eq!(plain_lines("| a |\n|--|\n| xxxxx | yy |", 8), ["a  xxxxx", "   yy"]);
}

#[test]
fn values_start_at_the_edge_when_every_name_is_empty() {
	assert_eq!(plain_lines("| | |\n|--|--|\n| xxxx | yyyy |", 10), ["xxxx", "yyyy"]);
}

#[test]
fn a_missing_cell_shows_its_name_alone() {
	// Every record has the same lines, so a field without a name or a
	// value is a blank line.
	assert_eq!(plain_lines("| a | | c |\n|--|--|--|\n| 1 | 2 | 3 |\n| 4 |", 10), ["a  1", "   2", "c  3", "──────────", "a  4", "", "c"]);
}

#[test]
fn a_table_without_data_rows_lays_out_its_names() {
	assert_eq!(plain_lines("| step | owner |\n|--|--|", 10), ["step", "owner"]);
}

#[test]
fn records_in_a_blockquote_carry_the_bars() {
	assert_eq!(
		plain_lines("> | a | b |\n> |--|--|\n> | xxxxx | yyyyy |\n> | z | w |", 12),
		["│ a  xxxxx", "│ b  yyyyy", "│ ──────────", "│ a  z", "│ b  w"]
	);
}

#[test]
fn a_value_word_wider_than_its_room_breaks_between_characters() {
	assert_eq!(
		plain_lines("| Col1 | Col2 |\n|---|---|\n| antidisestablishmentarianism | supercalifragilisticexpialidocious |", 10),
		[
			"Col1  anti",
			"      dise",
			"      stab",
			"      lish",
			"      ment",
			"      aria",
			"      nism",
			"Col2  supe",
			"      rcal",
			"      ifra",
			"      gili",
			"      stic",
			"      expi",
			"      alid",
			"      ocio",
			"      us",
		]
	);
}

#[test]
fn wide_characters_break_between_characters() {
	assert_eq!(plain_lines("| h |\n|---|\n| 日本語日本語 |", 9), ["h  日本語", "   日本語"]);
}

#[test]
fn emoji_sequences_stay_whole_when_a_word_breaks() {
	// Beside the name, 3 columns are left, and each emoji takes 2.
	assert_eq!(plain_lines("| h |\n|---|\n| xx\u{26A0}\u{FE0F}\u{26A0}\u{FE0F} |", 6), ["h  xx", "   \u{26A0}\u{FE0F}", "   \u{26A0}\u{FE0F}"]);
	let family = "\u{1F469}\u{200D}\u{1F469}\u{200D}\u{1F467}";
	assert_eq!(
		plain_lines(&format!("| h |\n|---|\n| xx{family}{family} |"), 6),
		["h  xx".to_string(), format!("   {family}"), format!("   {family}")]
	);
	// A combining accent stays with its letter.
	assert_eq!(plain_lines("| h |\n|---|\n| xxe\u{301}yy |", 6), ["h  xxe\u{301}", "   yy"]);
}

#[test]
fn a_record_line_is_wider_only_for_a_character_wider_than_its_room() {
	let letters = "| h | i |\n|---|---|\n| abc | de |";
	assert_eq!(plain_lines(letters, 1), ["h", "a", "b", "c", "i", "d", "e"]);
	assert_eq!(plain_lines(letters, 2), ["h", " a", " b", " c", "i", " d", " e"]);
	let wide = "| h | i |\n|---|---|\n| 日本 | x |";
	assert_eq!(plain_lines(wide, 1), ["h", "日", "本", "i", "x"]);
	assert_eq!(plain_lines(wide, 2), ["h", " 日", " 本", "i", " x"]);
}

#[test]
fn long_url_in_running_text_stays_whole() {
	assert_eq!(
		plain_lines("see [the policy](https://example.com/handbook/operations/retry-policy)", 40),
		["see the policy", "(https://example.com/handbook/operations/retry-policy)"]
	);
}

#[test]
fn wide_characters_in_a_cell_wrap_by_display_width() {
	// Width 10 leaves 6 columns for the cell, the width of `日本語`.
	assert_eq!(table_rows("| h |\n|---|\n| 日本語 ok |", 10), ["│   h    │", "│ 日本語 │", "│ ok     │"]);
}

// =========================================================
// Halfwidth sound marks
// =========================================================

// A halfwidth sound mark such as `ﾞ` takes a column of its own, as
// terminals draw it. Every character in these inputs takes one cell on
// screen, so counting characters measures a line as the screen shows it.

#[test]
fn halfwidth_sound_marks_count_when_text_wraps() {
	assert_eq!(plain_lines("ｶﾞｶﾞ ｶﾞｶﾞ", 8), ["ｶﾞｶﾞ", "ｶﾞｶﾞ"]);
}

#[test]
fn a_table_with_halfwidth_sound_marks_fits_its_width() {
	let lines = plain_lines("| a | b |\n|--|--|\n| x | ｶﾞｶﾞｶﾞｶﾞｶﾞｶﾞ |", 12);
	for line in &lines {
		assert!(line.chars().count() <= 12, "{lines:#?}");
	}
}

// =========================================================
// Graphemes
// =========================================================

// Widths count grapheme by grapheme, as terminals draw them: the letters
// of a Lam-Alef ligature take a column each.

#[test]
fn a_ligature_counts_a_column_per_letter_when_text_wraps() {
	assert_eq!(plain_lines("لا لا", 4), ["لا", "لا"]);
}

#[test]
fn a_flag_stays_whole_when_a_word_breaks() {
	// Beside the name, 3 columns are left, and each flag takes 2.
	assert_eq!(plain_lines("| h |\n|---|\n| xx🇩🇪🇩🇪 |", 6), ["h  xx", "   🇩🇪", "   🇩🇪"]);
}

// =========================================================
// Code blocks
// =========================================================

#[test]
fn code_block_basic() {
	assert_snapshot!(render("```\nfn main() {\n    println!(\"hello\");\n}\n```"));
}

#[test]
fn code_block_with_language() {
	assert_snapshot!(render("```rust\nlet x = 42;\n```"));
}

#[test]
fn code_block_no_wrap() {
	// A very long line in a code block must not be wrapped.
	let long_line = "x".repeat(120);
	let md = format!("```\n{long_line}\n```");
	// The long line should appear intact (not broken into multiple lines).
	assert_eq!(plain_lines(&md, 80), [format!("  {long_line}")]);
}

#[test]
fn code_block_between_paragraphs() {
	assert_snapshot!(render("Before.\n\n```\ncode\n```\n\nAfter."));
}

// ---------------------------------------------------------
// Code blocks — syntax highlighting
// ---------------------------------------------------------
//
// The snapshots below are the source of truth for which scopes in
// the `Ansi` theme map to which palette indices. Regenerate them
// deliberately (`cargo insta review`) if the upstream theme
// definition in `two-face` changes.

#[test]
fn code_block_highlights_rust() {
	assert_snapshot!(render("```rust\n// comment\nlet x: u32 = 42;\n```"));
}

#[test]
fn code_block_highlights_python() {
	assert_snapshot!(render("```python\n# comment\ndef greet(name):\n    return f\"hi {name}\"\n```"));
}

#[test]
fn code_block_highlights_javascript() {
	assert_snapshot!(render("```javascript\n// comment\nconst x = 42;\nlet s = \"hi\";\n```"));
}

#[test]
fn code_block_highlights_typescript() {
	assert_snapshot!(render("```typescript\n// comment\nconst n: number = 42;\n```"));
}

#[test]
fn code_block_highlights_go() {
	assert_snapshot!(render("```go\n// comment\nfunc greet(name string) string { return \"hi \" + name }\n```"));
}

#[test]
fn code_block_highlights_bash() {
	assert_snapshot!(render("```bash\n# comment\nfor i in 1 2 3; do echo \"$i\"; done\n```"));
}

#[test]
fn code_block_highlights_json() {
	assert_snapshot!(render("```json\n{\"id\": 42, \"name\": \"alice\"}\n```"));
}

#[test]
fn code_block_highlights_yaml() {
	assert_snapshot!(render("```yaml\nname: alice\nid: 42\ntags:\n  - admin\n  - user\n```"));
}

#[test]
fn code_block_highlights_toml() {
	assert_snapshot!(render("```toml\n[package]\nname = \"patine\"\nversion = \"1.0.0\"\n```"));
}

#[test]
fn code_block_highlights_markdown() {
	assert_snapshot!(render("```markdown\n# Heading\n\nSome *emphasis* and **strong** text.\n```"));
}

#[test]
fn code_block_unknown_language_falls_back_to_code_color() {
	// A nonsense language hint must render identically to no hint
	// at all: the whole block in flat CODE_COLOR.
	assert_snapshot!(render("```foobar\nsome text\n```"));
}

#[test]
fn code_block_case_variation_recognizes_language() {
	// `rust`, `Rust`, and `RUST` should all resolve to the same
	// syntax. Output must match byte-for-byte.
	let lower = render("```rust\nlet x = 1;\n```");
	let title = render("```Rust\nlet x = 1;\n```");
	let upper = render("```RUST\nlet x = 1;\n```");
	assert_eq!(lower, title);
	assert_eq!(lower, upper);
}

#[test]
fn code_block_preserves_multiline_string_state() {
	// A Rust string literal spanning multiple lines. The second
	// line must still be highlighted as "inside a string" — which
	// is only possible because we reuse one HighlightLines across
	// all lines of the block.
	assert_snapshot!(render("```rust\nlet s = \"first line\n  still inside string\";\n```"));
}

#[test]
fn highlighted_code_block_in_blockquote() {
	// Blockquote `│ ` bar prefix + per-token highlighting must
	// coexist: the bars keep their own style on every line and the
	// tokens stay highlighted per-token.
	assert_snapshot!(render("> Some intro text.\n>\n> ```rust\n> let x = 1;\n> ```"));
}

#[test]
fn highlighted_code_block_in_list_item() {
	assert_snapshot!(render("1. First step:\n\n   ```rust\n   let x = 1;\n   ```\n"));
}

#[test]
fn highlighted_code_block_never_wraps() {
	// A very long highlighted line in a narrow terminal must stay
	// on a single output line; only the terminal emulator itself
	// may soft-wrap it.
	let long = format!("let x = {};", "1 + ".repeat(40));
	let src = format!("```rust\n{long}\n```");
	assert_eq!(plain_lines(&src, 40), [format!("  {long}")]);
}

#[test]
fn highlighted_narrow_width_does_not_panic() {
	// Width smaller than the rendered content must not panic,
	// same guarantee as for the fallback path.
	assert_eq!(plain_lines("```rust\nlet x = 42;\n```", 6), ["  let x = 42;"]);
}

#[test]
fn highlighted_code_block_preserves_unicode() {
	// Multi-byte characters (combining accent, emoji) inside a
	// highlighted string literal must pass through verbatim and
	// not confuse column tracking.
	assert_snapshot!(render("```rust\nlet s = \"héllo 🎉\";\n```"));
}

#[test]
fn highlighted_empty_code_block_does_not_panic() {
	assert!(render_lines("```rust\n```", 80).is_empty());
}

// =========================================================
// Images
// =========================================================

#[test]
fn image_with_alt() {
	assert_snapshot!(render("![Architecture diagram](./docs/arch.png)"));
}

#[test]
fn image_without_alt() {
	assert_snapshot!(render("![](./screenshot.png)"));
}

#[test]
fn image_in_paragraph() {
	assert_snapshot!(render("See the diagram: ![overview](./overview.png) for context."));
}

// =========================================================
// Blockquotes
// =========================================================

#[test]
fn blockquote_simple() {
	assert_snapshot!(render("> This is a quote."));
}

#[test]
fn blockquote_multiple_paragraphs() {
	assert_snapshot!(render("> First paragraph.\n>\n> Second paragraph."));
}

#[test]
fn blockquote_nested() {
	assert_snapshot!(render("> Outer\n>\n>> Inner"));
}

#[test]
fn blockquote_with_emphasis() {
	assert_snapshot!(render("> This has **bold** in a quote."));
}

// =========================================================
// Regression: blockquote blank lines must have bar prefix
// =========================================================

#[test]
fn blockquote_blank_separator_has_bar() {
	// The blank line between two paragraphs inside a blockquote must render
	// with the │ bar prefix, so the visual bar column is unbroken.
	assert_eq!(plain_lines("> First\n>\n> Second", 80), ["│ First", "│ ", "│ Second"]);
}

// =========================================================
// Regression: list item text wrapping must indent past bullet
// =========================================================

#[test]
fn list_item_wrap_indentation() {
	// When a list item's text is long enough to wrap, the continuation line
	// must be indented past the bullet/number, not back at the left margin.
	let long_item = format!("- {}", "word ".repeat(30).trim());
	let lines = plain_lines(&long_item, 80);
	assert!(lines.len() >= 2, "expected wrapping to produce multiple lines");
	// The second line should have more indentation than just the 2-space
	// global indent — it should align with the text after "• ".
	let second_line = &lines[1];
	// "• " is 2 columns, so continuation should start with 2 spaces.
	assert!(second_line.starts_with("  "), "wrapped list continuation should be indented past the bullet.\nSecond line: '{second_line}'");
}

#[test]
fn ordered_list_item_wrap_indentation() {
	let long_item = format!("1. {}", "word ".repeat(30).trim());
	let lines = plain_lines(&long_item, 80);
	assert!(lines.len() >= 2, "expected wrapping to produce multiple lines");
	let second_line = &lines[1];
	// "1. " is 3 chars, so continuation should start with 3 spaces.
	assert!(second_line.starts_with("   "), "wrapped ordered list continuation should be indented past the number.\nSecond line: '{second_line}'");
}

// =========================================================
// List items use the full width
// =========================================================

// The bullet or number and the continuation indent are part of the line,
// so a list item's text fills the line up to the width, on its first line
// as on its wrapped ones. Each test picks the width at which three words
// fit exactly, and one column less, at which the third word wraps.

const FOUR_WORDS: &str = "aaaa bbbb cccc dddd eeee ffff gggg";

#[test]
fn bullet_item_fills_the_line_exactly() {
	let markdown = format!("- {FOUR_WORDS}");
	assert_eq!(plain_lines(&markdown, 16), ["• aaaa bbbb cccc", "  dddd eeee ffff", "  gggg"]);
	assert_eq!(plain_lines(&markdown, 15), ["• aaaa bbbb", "  cccc dddd", "  eeee ffff", "  gggg"]);
}

#[test]
fn numbered_items_fill_the_line_exactly() {
	assert_eq!(plain_lines(&format!("1. {FOUR_WORDS}"), 17), ["1. aaaa bbbb cccc", "   dddd eeee ffff", "   gggg"]);
	assert_eq!(plain_lines(&format!("10. {FOUR_WORDS}"), 18), ["10. aaaa bbbb cccc", "    dddd eeee ffff", "    gggg"]);
	assert_eq!(plain_lines(&format!("10. {FOUR_WORDS}"), 17), ["10. aaaa bbbb", "    cccc dddd", "    eeee ffff", "    gggg"]);
}

#[test]
fn nested_item_fills_the_line_exactly() {
	assert_eq!(plain_lines(&format!("- x\n  - {FOUR_WORDS}"), 18), ["• x", "  • aaaa bbbb cccc", "    dddd eeee ffff", "    gggg"]);
}

#[test]
fn item_in_a_quote_fills_the_line_exactly() {
	assert_eq!(plain_lines(&format!("> - {FOUR_WORDS}"), 18), ["│ • aaaa bbbb cccc", "│   dddd eeee ffff", "│   gggg"]);
}

#[test]
fn lines_after_breaks_in_an_item_fill_the_line_exactly() {
	let expected = ["• aaaa bbbb cccc", "  dddd eeee ffff", "  gggg"];
	assert_eq!(plain_lines("- aaaa bbbb cccc\\\ndddd eeee ffff gggg", 16), expected);
	assert_eq!(plain_lines_with("- aaaa bbbb cccc\ndddd eeee ffff gggg", 16, BREAK), expected);
}

#[test]
fn second_paragraph_of_an_item_fills_the_line_exactly() {
	assert_eq!(plain_lines(&format!("- x\n\n  {FOUR_WORDS}"), 16), ["• x", "", "  aaaa bbbb cccc", "  dddd eeee ffff", "  gggg"]);
}

#[test]
fn thematic_break_in_an_item_reaches_the_width() {
	// Blocks nested in an item sit one nesting indent in, and the rule
	// spans the rest of the line.
	let lines = plain_lines("- x\n\n  ***", 20);
	assert_eq!(lines[2], format!("  {}", "─".repeat(18)));
}

#[test]
fn table_in_an_item_reaches_the_width() {
	// The cell text is too long for one line, so the table takes all the
	// width it gets.
	let lines = plain_lines("- x\n\n  | aaaa bbbb cccc dddd eeee |\n  |---|", 20);
	let widths: Vec<usize> = lines[2..].iter().map(|line| line.chars().count()).collect();
	assert!(widths.iter().all(|&width| width == 20), "{lines:#?}");
}

#[test]
fn word_wider_than_the_room_after_the_bullet_stays_with_it() {
	// A line that holds only the bullet or the continuation indent is
	// empty for wrapping: a word too wide for it overflows there instead
	// of leaving the bullet alone on its line.
	assert_eq!(plain_lines("- abcdefghij", 8), ["• abcdefghij"]);
	assert_eq!(plain_lines("10. abcdefghij", 8), ["10. abcdefghij"]);
	assert_eq!(plain_lines("> - abcdefghij", 6), ["│ • abcdefghij"]);
	assert_eq!(plain_lines("- x\n\n  abcdefghij", 8), ["• x", "", "  abcdefghij"]);
	assert_eq!(plain_lines("- x abcdefghij", 8), ["• x", "  abcdefghij"]);
}

#[test]
fn blocks_nested_in_an_item_wrap_under_their_own_text() {
	// A quote or heading inside a list item sits one nesting indent in.
	// Its wrapped lines start where its first line does, not past the
	// item's bullet a second time.
	assert_eq!(plain_lines("- item\n\n  > aaaa bbbb cccc dddd", 18), ["• item", "", "  │ aaaa bbbb cccc", "  │ dddd"]);
	assert_eq!(plain_lines("10. item\n\n    > aaaa bbbb cccc dddd", 18), ["10. item", "", "  │ aaaa bbbb cccc", "  │ dddd"]);
	assert_eq!(plain_lines("- item\n\n  ### aaaa bbbb cccc dddd", 16), ["• item", "", "  aaaa bbbb cccc", "  dddd"]);
}

#[test]
fn block_first_in_an_item_starts_right_under_the_bullet() {
	// A line that holds only the bullet or number ends there, and the
	// block starts on the next line, one nesting indent in, with no blank
	// line between them.
	assert_eq!(plain_lines("- > quote", 80), ["• ", "  │ quote"]);
	assert_eq!(plain_lines("10. > quote", 80), ["10. ", "  │ quote"]);
	assert_eq!(plain_lines("- ```\n  code\n  ```", 80), ["• ", "    code"]);
	assert_eq!(plain_lines("10. ```\n    code\n    ```", 80), ["10. ", "    code"]);
	assert_eq!(plain_lines("- # Heading", 80), ["• ", "  Heading"]);
	assert_eq!(plain_lines("10. # Heading", 80), ["10. ", "  Heading"]);
	assert_eq!(plain_lines("> - > quote", 80), ["│ • ", "│   │ quote"]);
}

#[test]
fn nested_list_first_in_an_item_starts_right_under_the_bullet() {
	assert_eq!(plain_lines("- - nested", 80), ["• ", "  • nested"]);
	assert_eq!(plain_lines("10. - nested", 80), ["10. ", "  • nested"]);
}

#[test]
fn block_after_other_content_in_an_item_keeps_its_blank_line() {
	// Only the bullet's own line lets a block start without a blank line.
	// After a nested list the line is closed, and the block is spaced as
	// usual.
	assert_eq!(plain_lines("- - nested\n\n  > quote", 80), ["• ", "  • nested", "", "  │ quote"]);
}

#[test]
fn empty_block_first_in_an_item_keeps_spacing_after_the_list() {
	// A block that draws nothing still leaves the bullet as output above
	// the next block, which then gets its blank line.
	assert_eq!(plain_lines("- >\n\npara", 80), ["• ", "", "para"]);
}

// =========================================================
// Prefix of nested blocks
// =========================================================

// A line's prefix lists the blocks around it from the outside in: a
// quote's bar, a list item's indent, a code block's indent, in the order
// the blocks were entered.

#[test]
fn code_block_in_a_nested_item_sits_past_both_indents() {
	assert_eq!(plain_lines("- a\n  - b\n\n    ```\n    code\n    ```", 80), ["• a", "  • b", "", "      code"]);
}

#[test]
fn item_in_two_quotes_wraps_inside_both_bars() {
	assert_eq!(plain_lines("> > - aaaa bbbb cccc", 14), ["│ │ • aaaa", "│ │   bbbb", "│ │   cccc"]);
}

#[test]
fn table_in_an_item_in_a_quote_reaches_the_width() {
	let lines = plain_lines("> - x\n>\n>   | aaaa bbbb cccc dddd eeee |\n>   |---|", 20);
	let table = lines.iter().skip_while(|line| !line.contains('┌')).collect::<Vec<_>>();
	assert!(!table.is_empty(), "{lines:#?}");
	for line in table {
		assert_eq!(line.chars().count(), 20, "{lines:#?}");
	}
}

#[test]
fn block_after_a_break_that_drew_nothing_starts_on_its_own_line() {
	// The first paragraph draws nothing but breaks its line, which leaves
	// a line holding only the continuation indent. The quote starts on
	// the line after it, not appended to it.
	assert_eq!(plain_lines("- <b></b>\\\n  <i></i>\n\n  > q", 80), ["• ", "  ", "  │ q"]);
}

#[test]
fn code_block_in_a_quote_sits_inside_the_bar() {
	// Every line of the block carries the whole prefix, the empty one too.
	assert_eq!(plain_lines("> ```\n> code\n>\n> x\n> ```", 80), ["│   code", "│   ", "│   x"]);
}

#[test]
fn nested_list_in_a_quote_sits_inside_the_bar() {
	assert_eq!(plain_lines("> - a\n>   - aaaa bbbb cccc", 14), ["│ • a", "│   • aaaa", "│     bbbb", "│     cccc"]);
}

#[test]
fn blocks_in_an_item_in_a_quote_sit_inside_the_bar() {
	assert_eq!(plain_lines("> - aaa\n>\n>   ```\n>   code\n>   ```", 40), ["│ • aaa", "│ ", "│     code"]);
	assert_eq!(plain_lines("> - # heading", 40), ["│ • ", "│   heading"]);
	assert_eq!(plain_lines("> > - > x", 40), ["│ │ • ", "│ │   │ x"]);
	let rule = plain_lines("> - x\n>\n>   ***", 20);
	assert_eq!(rule[2], format!("│   {}", "─".repeat(16)));
}

#[test]
fn blank_line_in_an_item_in_a_quote_ends_at_the_bar() {
	assert_eq!(plain_lines("> - a\n>\n>   > q", 40), ["│ • a", "│ ", "│   │ q"]);
}

#[test]
fn prefix_spans_follow_the_nesting_order() {
	// The bars keep their own style wherever they sit, and the indent
	// between them is unstyled.
	let lines = render_lines("> - > quote", 80);
	let bar = styled_chars(&lines[0])[0].1;
	let prefix: Vec<(char, TextStyle)> = styled_chars(&lines[1])[..6].to_vec();
	assert_eq!(
		prefix,
		[('│', bar), (' ', bar), (' ', TextStyle::default()), (' ', TextStyle::default()), ('│', bar), (' ', bar),],
		"{}",
		serialize(&lines)
	);
}

#[test]
fn block_after_a_first_block_that_drew_nothing_starts_under_the_bullet() {
	// A block that draws nothing leaves the bullet alone on its line, so
	// the next block follows it as if it came first.
	assert_eq!(plain_lines("- <div>\n\n  > q", 80), ["• ", "  │ q"]);
	assert_eq!(plain_lines("- ```\n  ```\n\n  > q", 80), ["• ", "  │ q"]);
}

#[test]
fn paragraph_after_an_empty_block_follows_the_bullet() {
	// Nothing has been drawn after the bullet, so the paragraph starts on
	// the next line without a blank line, as a quote does.
	assert_eq!(plain_lines("- <div>\n\n  para", 80), ["• ", "  para"]);
}

#[test]
fn paragraph_after_an_empty_code_block_follows_the_bullet() {
	assert_eq!(plain_lines("- ```\n  ```\n\n  para", 80), ["• ", "  para"]);
}

#[test]
fn paragraph_after_an_empty_paragraph_follows_the_bullet() {
	// The first paragraph holds only inline HTML that is not drawn.
	assert_eq!(plain_lines("- <b></b>\n\n  para", 80), ["• ", "  para"]);
}

#[test]
fn second_paragraph_of_an_item_keeps_its_blank_line() {
	assert_eq!(plain_lines("- a\n\n  b", 80), ["• a", "", "  b"]);
}

// =========================================================
// Spacing of a list by its parent
// =========================================================

// A list nested directly in a list item follows the item's text without
// a blank line. Any other list gets the blank line before it, also when
// a quote or footnote around it sits in a list item.

#[test]
fn list_directly_in_an_item_follows_its_text() {
	assert_eq!(plain_lines("- a\n  - b", 80), ["• a", "  • b"]);
	assert_eq!(plain_lines("- a\n\n  - b", 80), ["• a", "  • b"]);
	assert_eq!(plain_lines("- > - c", 80), ["• ", "  │ • c"]);
}

#[test]
fn list_after_text_in_a_quote_in_an_item_gets_a_blank_line() {
	assert_eq!(plain_lines("- a\n\n  > b\n  >\n  > - c", 80), ["• a", "", "  │ b", "  │ ", "  │ • c"]);
}

#[test]
fn quote_in_an_item_spaces_a_list_like_a_top_level_quote() {
	let top_level = plain_lines("> b\n>\n> - c", 80);
	let in_item = plain_lines("- a\n\n  > b\n  >\n  > - c", 80);
	assert_eq!(in_item[2..], top_level.iter().map(|line| format!("  {line}")).collect::<Vec<_>>());
}

#[test]
fn list_in_a_footnote_definition_in_an_item_gets_a_blank_line() {
	// At the top level, `a\n\n[^1]: - x` already gets the blank line.
	assert_eq!(plain_lines("- a\n\n  [^1]: - x\n\nb[^1]", 80), ["• a", "", "  • x", "", "b"]);
}

// =========================================================
// Regression: blockquote must not emit bare styles before first bar
// =========================================================

#[test]
fn blockquote_no_bare_style_lines() {
	// The blockquote must not emit lines that are purely style escape codes
	// without the bar prefix. Every line with content or style tags should
	// also include the │ bar (except the final style-closing line).
	let lines = plain_lines("> Hello", 80);
	let output = lines.join("\n");
	// The first line must contain the bar.
	assert!(lines[0].contains('│'), "first line of blockquote must contain │ bar.\nFirst line: '{}'\nFull output:\n{output}", lines[0]);
}

// =========================================================
// Regression: no empty bar line at start of blockquote
// =========================================================

#[test]
fn blockquote_no_leading_empty_bar_line() {
	// When a blockquote follows another block (e.g., a heading), the blank
	// line separator must NOT contain a │ bar. The bar should only appear
	// on lines that are part of the blockquote content.
	let lines = plain_lines("# Title\n\n> Quote", 80);
	let output = lines.join("\n");
	// Find the first line that contains │. It must also contain the word "Quote".
	let first_bar_line = lines.iter().find(|l| l.contains('│'));
	assert!(
		first_bar_line.is_some_and(|l| l.contains("Quote")),
		"first line with │ bar must be a content line, not an empty bar.\nFull output:\n{output}"
	);
}

#[test]
fn heading_then_blockquote_has_blank_line() {
	// A blank line must separate the heading from the blockquote content.
	let lines = plain_lines("# Title\n\n> Quote", 80);
	let output = lines.join("\n");
	// There must be an empty line between the heading line and the blockquote.
	assert!(lines.len() >= 3, "expected at least 3 lines (heading, blank, blockquote)");
	// The second line (index 1) should be empty (the blank separator).
	assert!(
		lines[1].trim().is_empty() || !lines[1].contains('│'),
		"blank line between heading and blockquote must not have a bar.\nLine: '{}'\nFull output:\n{output}",
		lines[1]
	);
}

// =========================================================
// Horizontal rules
// =========================================================

#[test]
fn horizontal_rule() {
	assert_snapshot!(render("Above\n\n---\n\nBelow"));
}

#[test]
fn horizontal_rule_width() {
	// At 40-column terminal width, the rule should span all 40 columns.
	assert_eq!(plain_lines("---", 40), ["─".repeat(40)]);
}

// =========================================================
// Frontmatter
// =========================================================

#[test]
fn frontmatter_unclosed_fence_is_not_frontmatter() {
	// An opening `---` without a closing fence is not frontmatter. The
	// markdown parser falls back to normal parsing: `---` followed by
	// text becomes a thematic break and a paragraph.
	assert_snapshot!(render("---\ntitle: x"));
}

#[test]
fn frontmatter_yaml_basic() {
	// YAML frontmatter renders as a syntax-highlighted verbatim block,
	// fence lines included, followed by the document body.
	assert_snapshot!(render("---\ntitle: x\ntags: [a, b]\n---\n\n# Body"));
}

#[test]
fn frontmatter_toml_basic() {
	// TOML frontmatter (`+++` fences) gets the same treatment with the
	// TOML grammar.
	assert_snapshot!(render("+++\ntitle = \"x\"\n\n[owner]\nname = \"jakob\"\n+++\n\nBody"));
}

#[test]
fn frontmatter_invalid_yaml_still_renders() {
	// Highlighting is lexical, not parsing: syntactically broken YAML
	// renders verbatim without any error.
	assert_snapshot!(render("---\ntitle: [unclosed\n  ::: not yaml\n---\n\nBody"));
}

#[test]
fn frontmatter_empty_renders_nothing() {
	// Empty frontmatter carries no metadata: the output is identical to
	// rendering the body alone.
	assert_eq!(render("---\n---\n\nBody"), render("Body"));
}

#[test]
fn frontmatter_whitespace_only_renders_nothing() {
	assert_eq!(render("---\n   \n---\n\nBody"), render("Body"));
}

#[test]
fn frontmatter_only_document() {
	// A document that is nothing but frontmatter renders the block and
	// nothing after it: no trailing blank line.
	let lines = render_lines("---\ntitle: x\n---", 80);
	assert_snapshot!(serialize(&lines));
	assert!(lines.last().is_some_and(|line| !line.spans.is_empty()), "output must not end with a blank line");
}

#[test]
fn frontmatter_not_at_document_start_ignored() {
	// Frontmatter is only recognized at the very start of the document.
	// With a leading blank line, `---` pairs parse as ordinary markdown.
	assert_snapshot!(render("\n---\ntitle: x\n---\n\nBody"));
}

#[test]
fn frontmatter_long_lines_never_wrap() {
	// Frontmatter shares the code-block contract: verbatim lines, never
	// word-wrapped, even past the terminal width.
	let md = "---\ndescription: this value is far too long to fit into forty columns of output\n---";
	let lines = plain_lines(md, 40);
	assert!(lines.iter().any(|l| l.contains("forty columns")), "long value must stay on one verbatim line.\nOutput: {lines:#?}");
}

#[test]
fn frontmatter_unicode_content() {
	// Non-ASCII content passes through verbatim.
	assert_snapshot!(render("---\ntitle: 日本語のタイトル 🚀\n---\n\nBody"));
}

// =========================================================
// Block spacing
// =========================================================

#[test]
fn no_leading_blank_line() {
	let lines = plain_lines("# Hello", 80);
	assert_eq!(lines[0], "Hello", "output must not start with a blank line");
}

#[test]
fn no_trailing_blank_line() {
	assert_eq!(plain_lines("Hello\n\n\n", 80), ["Hello"]);
}

#[test]
fn heading_paragraph_heading_spacing() {
	assert_snapshot!(render("## First\n\nA paragraph.\n\n## Second"));
}

// =========================================================
// Effective width floor
// =========================================================

#[test]
fn effective_width_floored_thematic_break_not_empty() {
	// With a very narrow terminal and deep nesting, effective_width() can
	// reach 0 after subtracting indent widths. A thematic break rendered at
	// effective_width=0 produces an empty (invisible) rule because
	// "─".repeat(0) is "".
	//
	// Four levels of blockquote at width 8 consumes all 8 columns of indent
	// (4 × 2 = 8), leaving effective_width = 0 without the floor.
	let output = render_with_width("> > > > ---", 8);
	assert!(output.contains('─'), "thematic break must not disappear at zero effective width:\n{output}");
}

// =========================================================
// Edge case: very small terminal widths
// =========================================================

#[test]
fn very_narrow_width_zero_does_not_panic() {
	// Width 0 should not panic; the `effective_width().max(1)` floor
	// should be enough. Whatever we produce is allowed to overflow — but
	// the render call itself must succeed and produce non-empty output
	// for non-empty input.
	let output = render_with_width("hello world", 0);
	assert!(!output.trim().is_empty(), "empty output at width 0");
	assert!(output.contains("hello"));
	assert!(output.contains("world"));
}

#[test]
fn very_narrow_width_one_does_not_panic() {
	let output = render_with_width("hello world", 1);
	assert!(!output.trim().is_empty(), "empty output at width 1");
	assert!(output.contains("hello"));
	assert!(output.contains("world"));
}

#[test]
fn narrow_width_five_wraps_every_word() {
	// At width 5, "hello" fits exactly, so each word goes on its own line.
	let output = render_with_width("hello world foo bar", 5);
	// Every word should appear.
	for word in ["hello", "world", "foo", "bar"] {
		assert!(output.contains(word), "missing {word:?} in:\n{output}");
	}
	// Every word should be on its own line (because 5+1+5 > 5).
	let visible_lines: Vec<String> = plain_lines("hello world foo bar", 5).into_iter().filter(|l| !l.trim().is_empty()).collect();
	assert!(visible_lines.len() >= 4, "expected ≥4 lines (one per word), got {}:\n{output}", visible_lines.len());
}

#[test]
fn narrow_width_heading_does_not_panic() {
	// Headings have their own block spacing and underline logic; make
	// sure it survives narrow widths without panicking.
	let output = render_with_width("# a heading", 3);
	assert!(output.contains("heading") || output.contains("a"));
}

#[test]
fn narrow_width_code_block_does_not_panic() {
	let output = render_with_width("```\nlong code line here\n```", 6);
	// The content should still appear; code blocks don't wrap, so this
	// just confirms we don't panic and output is non-empty.
	assert!(!output.trim().is_empty());
	assert!(output.contains("long"));
}

// =========================================================
// Edge case: hard line breaks
// =========================================================

#[test]
fn hard_break_trailing_backslash() {
	// A `\` at the end of a line is a Markdown hard break. The two
	// parts must appear on separate visual lines (not joined by a
	// space).
	let lines = plain_lines("first line\\\nsecond line", 80);
	let output = lines.join("\n");
	// Check: "first line" and "second line" are on different lines.
	let first_idx = lines.iter().position(|l| l.contains("first line")).expect("first line present");
	let second_idx = lines.iter().position(|l| l.contains("second line")).expect("second line present");
	assert_ne!(first_idx, second_idx, "hard break must put the two halves on different visual lines:\n{output}");
}

#[test]
fn hard_break_two_trailing_spaces() {
	// Two trailing spaces before a newline is the classic Markdown
	// hard-break spelling. Same expected behavior as the backslash
	// form.
	let lines = plain_lines("first line  \nsecond line", 80);
	let output = lines.join("\n");
	let first_idx = lines.iter().position(|l| l.contains("first line")).expect("first line present");
	let second_idx = lines.iter().position(|l| l.contains("second line")).expect("second line present");
	assert_ne!(first_idx, second_idx, "hard break must put the two halves on different visual lines:\n{output}");
}

#[test]
fn hard_break_in_list_item_lines_up_with_item_text() {
	// The line after a hard break continues the item's text, so it
	// starts under that text like a wrapped line does, not under the
	// bullet.
	assert_eq!(plain_lines("- item\\\nmore\n- next", 80), ["• item", "  more", "• next"]);
	assert_eq!(plain_lines("10. item  \nmore", 80), ["10. item", "    more"]);
}

#[test]
fn hard_break_in_nested_list_item_lines_up_with_item_text() {
	assert_eq!(plain_lines("- outer\n  - inner\\\n    more", 80), ["• outer", "  • inner", "    more"]);
}

#[test]
fn hard_break_in_list_item_inside_blockquote_keeps_the_bar() {
	assert_eq!(plain_lines("> - item\\\n>   more", 80), ["│ • item", "│   more"]);
}

#[test]
fn hard_break_after_a_space_adds_no_space_to_the_next_line() {
	// `item \` leaves a trailing space in the text before the break. The
	// space must not reappear after the continuation indent.
	assert_eq!(plain_lines("- item \\\nmore", 80), ["• item", "  more"]);
}

#[test]
fn hard_break_in_list_item_snapshot() {
	// The continuation indent is unstyled layout; the bold text on both
	// sides of the break keeps its style.
	assert_snapshot!(render("- **bold\\\nmore** plain\n  - nested\\\n    line"));
}

// =========================================================
// Edge case: multi-paragraph list items
// =========================================================

#[test]
fn list_item_with_multiple_paragraphs() {
	// A single list item containing two paragraphs. Both paragraphs
	// must render inside the item (indented past the bullet), with a
	// blank line between them — not glued together inline.
	let md = "- first paragraph\n\n  second paragraph\n- next item";
	let output = render(md);
	assert!(output.contains("first paragraph"), "missing first paragraph:\n{output}");
	assert!(output.contains("second paragraph"), "missing second paragraph:\n{output}");
	assert!(output.contains("next item"), "missing next item:\n{output}");
	// The second paragraph must appear after the first and before
	// "next item" in the output.
	let first_pos = output.find("first paragraph").unwrap();
	let second_pos = output.find("second paragraph").unwrap();
	let next_pos = output.find("next item").unwrap();
	assert!(
		first_pos < second_pos && second_pos < next_pos,
		"paragraphs out of order:\nfirst@{first_pos} second@{second_pos} next@{next_pos}\n{output}"
	);
	// There must be at least one newline between the two paragraphs
	// of the same list item — they cannot be glued together inline.
	let between = &output[first_pos + "first paragraph".len()..second_pos];
	assert!(between.contains('\n'), "expected newline between the two paragraphs, got {between:?}\nfull:\n{output}");
	// Pin the exact visual shape so regressions are loud.
	assert_snapshot!(output);
}

// =========================================================
// Edge case: empty code blocks
// =========================================================

#[test]
fn empty_code_block_does_not_panic() {
	// A fenced code block with no body must render without panicking
	// and must produce no lines at all (empty → empty).
	assert_eq!(render_lines("```\n```", 80), []);
}

#[test]
fn empty_code_block_with_language_does_not_panic() {
	assert_eq!(render_lines("```rust\n```", 80), []);
}

#[test]
fn empty_code_block_then_following_content_still_renders() {
	// An empty code block followed by a paragraph should not swallow
	// or mangle the following content.
	let output = render("```\n```\n\ntext after empty block");
	assert!(output.contains("text after empty block"), "content after empty code block must still render:\n{output}");
}

// =========================================================
// Line endings in verbatim blocks
// =========================================================

// CommonMark ends a line at `\n`, `\r\n` or a lone `\r`, and markdown-rs
// keeps whichever the source used in the value of a code block or
// frontmatter. Every one of them starts a new output line.

#[test]
fn code_block_lines_end_at_every_line_ending() {
	for fence in ["```", "```rust"] {
		for ending in ["\n", "\r\n", "\r"] {
			let markdown = format!("{fence}{ending}let a;{ending}let b;{ending}```");
			assert_eq!(plain_lines(&markdown, 80), ["  let a;", "  let b;"], "{markdown:?}");
		}
	}
}

#[test]
fn lone_carriage_return_inside_a_code_block_ends_the_line() {
	for fence in ["```", "```rust"] {
		let markdown = format!("{fence}\nlet a;\rlet b;\n```");
		assert_eq!(plain_lines(&markdown, 80), ["  let a;", "  let b;"], "{markdown:?}");
	}
}

#[test]
fn frontmatter_lines_end_at_every_line_ending() {
	assert_eq!(plain_lines("---\ntitle: a\rdraft: true\n---\n", 80), ["  ---", "  title: a", "  draft: true", "  ---"]);
}

#[test]
fn no_span_holds_a_carriage_return() {
	let documents = [
		"---\ra: 1\r---\r\r# Title\r\rText\rmore\r\r- item\r  more\r\r> quote\r> more\r\r```\ra\rb\r```\r\r```rust\rlet a;\rlet b;\r```\r\r<div>\rblock\r</div>\r\r| a |\r|---|\r| b |\r\r`co\rde` [li\rnk](u) ![a\rb](u)",
		"---\r\na: 1\r\n---\r\n\r\nText\r\nmore\r\n\r\n```rust\r\nlet a;\r\n```\r\n",
	];
	for markdown in documents {
		for options in [RenderOptions::default(), BREAK_AND_LITERAL] {
			for line in render_lines_with(markdown, 80, &options) {
				for span in &line {
					assert!(!span.text.contains('\r'), "{span:?} in {markdown:?} with {options:?}");
				}
			}
		}
	}
}

// =========================================================
// End-to-end showcase: a realistic document using every feature
// =========================================================

/// Fixture: a realistic document that exercises every Markdown feature
/// patine supports — h1/h2/h3, paragraphs with soft breaks,
/// bold/italic/bold+italic/strikethrough, inline code, fenced code
/// blocks (with and without a language hint), ordered and unordered
/// lists, nested lists, multi-paragraph list items, simple and nested
/// blockquotes, tables, thematic breaks, inline links, autolinks,
/// images, and hard breaks.
///
/// The snapshot locks in the full end-to-end visual shape. Any change
/// that alters how ANY feature renders will show up here in a single
/// diff, making regressions trivially visible during review.
const SHOWCASE_MD: &str = include_str!("fixtures/showcase.md");

#[test]
fn showcase_document_renders() {
	assert_snapshot!(render(SHOWCASE_MD));
}

// =========================================================
// Line prefixes are unstyled layout
// =========================================================

#[test]
fn wrapped_continuation_indent_is_unstyled() {
	// A link or strikethrough that wraps inside a list item continues on
	// a line that starts with the continuation indent. Those spaces are
	// layout, not content, so they carry no underline or strikethrough.
	let inputs = [
		"- see [a link whose text wraps over lines](https://x.io)",
		"- see ~~a struck phrase whose text wraps over lines~~",
		"- outer\n  - inner ~~struck words that wrap around~~",
	];
	for input in inputs {
		let lines = render_lines(input, 16);
		for line in lines.iter().skip(1) {
			for (c, style) in styled_chars(line).into_iter().take_while(|&(c, _)| c == ' ') {
				assert_eq!(style, TextStyle::default(), "indent {c:?} of {line:?} is styled\nfull:\n{}", serialize(&lines));
			}
		}
	}
}

/// The style of every bar character at the start of each line of
/// `markdown`, which must all lie inside a quote.
fn bar_styles(lines: &[StyledLine]) -> Vec<(char, TextStyle)> {
	let mut bars = Vec::new();
	for line in lines {
		let prefix: Vec<(char, TextStyle)> = styled_chars(line).into_iter().take_while(|&(c, _)| c == '│' || c == ' ').collect();
		assert!(!prefix.is_empty(), "line {line} lacks a bar");
		bars.extend(prefix);
	}
	bars
}

/// A quote with emphasis, a wrapping heading, a blank separator line and
/// a nested quote: every place a bar is drawn.
const QUOTE_WITH_EVERY_BAR: &str = "> quoted *text*\n>\n> ## A heading that wraps\n>\n> > nested";

#[test]
fn blockquote_bars_are_cyan_only() {
	// The bars are cyan whatever the quoted content looks like, including
	// on the wrapped lines of a heading inside the quote.
	let lines = render_lines(QUOTE_WITH_EVERY_BAR, 16);
	let mut cyan = TextStyle::default();
	cyan.foreground = Some(AnsiColor::Cyan);
	for (c, style) in bar_styles(&lines) {
		assert_eq!(style, cyan, "{c:?}\nfull:\n{}", serialize(&lines));
	}
}

#[test]
fn quote_bars_take_the_color_of_the_option() {
	let options = RenderOptions::new().with_quote_style(QuoteStyle::Bar(AnsiColor::Blue));
	let lines = render_lines_with(QUOTE_WITH_EVERY_BAR, 16, &options);
	let mut blue = TextStyle::default();
	blue.foreground = Some(AnsiColor::Blue);
	for (c, style) in bar_styles(&lines) {
		assert_eq!(style, blue, "{c:?}\nfull:\n{}", serialize(&lines));
	}
}

#[test]
fn dim_bar_quote_style_draws_dim_bars_and_upright_text() {
	let options = RenderOptions::new().with_quote_style(QuoteStyle::DimBar);
	let lines = render_lines_with(QUOTE_WITH_EVERY_BAR, 16, &options);
	for (c, style) in bar_styles(&lines) {
		assert_eq!(style, dim(), "{c:?}\nfull:\n{}", serialize(&lines));
	}
	assert_eq!(serialize(&lines[..1]), "[dim]│ []quoted [italic]text[]\n");
}

#[test]
fn emphasis_stays_visible_inside_a_quote() {
	assert_eq!(render("> a *b* c"), "[cyan]│ []a [italic]b[] c\n");
}

#[test]
fn table_in_a_quote_keeps_its_own_style() {
	// Only the header text is bold; borders and cells are plain.
	let lines = render_lines("> | col1 | col2 |\n> |------|------|\n> | a    | b    |", 80);
	assert_eq!(lines.len(), 5, "full:\n{}", serialize(&lines));
	for line in &lines {
		let chars = styled_chars(line);
		let (_, table) = chars.split_at(2);
		let in_header_row = line.to_string().contains("col");
		for &(c, style) in table {
			let mut expected = TextStyle::default();
			expected.bold = in_header_row && c.is_alphanumeric();
			assert_eq!(style, expected, "{c:?} in {line}\nfull:\n{}", serialize(&lines));
		}
	}
}

#[test]
fn code_in_a_quote_keeps_its_own_style() {
	let lines = render_lines("> ```rust\n> let x = 1;\n> ```", 80);
	assert_eq!(lines.len(), 1);
	for (c, style) in styled_chars(&lines[0]).into_iter().skip(2) {
		assert!(!style.dim && !style.italic, "{c:?} is dim or italic");
	}
}

// =========================================================
// Render options
// =========================================================

const BREAK: RenderOptions = RenderOptions::new().with_soft_breaks(SoftBreaks::Break);
const LITERAL: RenderOptions = RenderOptions::new().with_html(Html::Literal);
const BREAK_AND_LITERAL: RenderOptions = BREAK.with_html(Html::Literal);

/// Like [`render_with_width`], with render options.
fn render_with_options(markdown: &str, width: usize, options: RenderOptions) -> String {
	serialize(&render_lines_with(markdown, width, &options))
}

/// Like [`plain_lines`], with render options.
fn plain_lines_with(markdown: &str, width: usize, options: RenderOptions) -> Vec<String> {
	render_lines_with(markdown, width, &options).iter().map(ToString::to_string).collect()
}

/// Documents that use soft breaks and raw HTML in every place the options
/// look at, plus the showcase.
fn option_documents() -> Vec<&'static str> {
	vec![
		SHOWCASE_MD,
		"first\nsecond\r\nthird\rfourth",
		"- item\nmore\n  - nested\n    line\n\n> quoted\n> more",
		"Use a Vec<String> or <code>x</code> <!-- note -->\n\n<div>\n  block\n</div>",
		"| a <b>x</b> |\n|---|\n| <br> |",
		"Foo\nbar\n===\n\n[li\nnk](https://example.com) `co\nde` ![a\nb](u.png)",
	]
}

#[test]
fn default_options_render_like_render_lines() {
	// Every snapshot above goes through `render_lines`; this pins that it
	// and the defaults are one and the same rendering.
	for markdown in option_documents() {
		for width in [80, 20] {
			assert_eq!(render_lines_with(markdown, width, &RenderOptions::default()), render_lines(markdown, width), "{markdown:?} at {width}");
		}
	}
}

#[test]
fn options_matrix_snapshot() {
	// One document under all four combinations, so a single snapshot
	// shows what each option changes and that they combine independently.
	let markdown = "Pick **one\nof these**:\n\n- `Vec<String>` or Vec<String>\n  for the <em>list</em>\n- none\n\n<details>\n  <summary>More</summary>\n</details>";
	let mut out = String::new();
	for (name, options) in
		[("join, drop", RenderOptions::default()), ("break, drop", BREAK), ("join, literal", LITERAL), ("break, literal", BREAK_AND_LITERAL)]
	{
		out.push_str(&format!("== {name}\n"));
		out.push_str(&render_with_options(markdown, 80, options));
	}
	assert_snapshot!(out);
}

// ---------------------------------------------------------
// Soft breaks
// ---------------------------------------------------------

#[test]
fn soft_breaks_join_by_default() {
	assert_eq!(plain_lines_with("first\nsecond", 80, RenderOptions::default()), ["first second"]);
}

#[test]
fn soft_break_breaks_the_line() {
	assert_eq!(plain_lines_with("first\nsecond\nthird", 80, BREAK), ["first", "second", "third"]);
}

#[test]
fn soft_break_accepts_every_line_ending() {
	assert_eq!(plain_lines_with("a\r\nb\rc\nd", 80, BREAK), ["a", "b", "c", "d"]);
	// A text node that ends in the line ending, right before an inline
	// element.
	for ending in ["\n", "\r\n", "\r"] {
		assert_eq!(plain_lines_with(&format!("a{ending}**b**"), 80, BREAK), ["a", "b"], "{ending:?}");
	}
}

#[test]
fn soft_break_keeps_the_styles_across_the_break() {
	assert_snapshot!(render_with_options("**bold\nstill bold** plain\n*italic* ~~struck\nstruck~~", 80, BREAK));
}

#[test]
fn soft_break_in_list_items_lines_up_with_item_text() {
	assert_eq!(plain_lines_with("- item\nmore\n  - nested\n    line\n- next", 80, BREAK), ["• item", "  more", "  • nested", "    line", "• next"]);
}

#[test]
fn soft_break_in_blockquote_keeps_the_bars() {
	assert_snapshot!(render_with_options("> quoted\n> more\n>\n> - item\n>   line", 80, BREAK));
}

#[test]
fn soft_break_lines_still_wrap() {
	assert_eq!(plain_lines_with("one two three four five\nsix", 14, BREAK), ["one two three", "four five", "six"]);
}

#[test]
fn soft_and_hard_breaks_mix() {
	assert_eq!(plain_lines_with("a\\\nb\nc  \nd", 80, BREAK), ["a", "b", "c", "d"]);
}

#[test]
fn soft_break_in_link_text_and_setext_heading() {
	// Link text and a multi-line setext heading are running text too.
	assert_snapshot!(render_with_options("Foo\nbar\n===\n\n[link\ntext](https://example.com)", 80, BREAK));
}

#[test]
fn soft_break_leaves_code_alt_text_and_frontmatter_alone() {
	// Inline code and image alt text turn a line ending into a space, and
	// code blocks and frontmatter keep their lines whatever the option.
	let markdown = "---\ntitle: x\nlist: [a]\n---\n\n`co\nde` ![a\nb](u.png)\n\n```\nline one\nline two\n```";
	assert_eq!(render_with_options(markdown, 80, BREAK), render_with_options(markdown, 80, RenderOptions::default()));
	assert!(plain_lines_with(markdown, 80, BREAK).contains(&"co de [image: a b] (u.png)".to_string()));
}

// ---------------------------------------------------------
// HTML
// ---------------------------------------------------------

#[test]
fn html_is_dropped_by_default() {
	assert_eq!(
		plain_lines_with("Use a Vec<String> here.\n\n<div>\nblock\n</div>\n\nafter", 80, RenderOptions::default()),
		["Use a Vec here.", "", "after"]
	);
}

#[test]
fn inline_html_shows_as_written() {
	assert_eq!(
		plain_lines_with("Use a Vec<String> or <code>x</code> <!-- note --> here.", 80, LITERAL),
		["Use a Vec<String> or <code>x</code> <!-- note --> here."]
	);
}

#[test]
fn inline_html_takes_the_surrounding_style() {
	assert_snapshot!(render_with_options("**a <b>bold</b> tag**, [a <i>link</i>](https://example.com)\n\n> quoted <span>", 80, LITERAL));
}

#[test]
fn inline_html_wraps_like_words() {
	assert_eq!(plain_lines_with("one <span class=\"x\"> two </span> three", 16, LITERAL), ["one <span", "class=\"x\"> two", "</span> three"]);
}

#[test]
fn line_ending_inside_inline_html_follows_soft_breaks() {
	let markdown = "a <span\nclass=\"x\"> b";
	assert_eq!(plain_lines_with(markdown, 80, LITERAL), ["a <span class=\"x\"> b"]);
	assert_eq!(plain_lines_with(markdown, 80, BREAK_AND_LITERAL), ["a <span", "class=\"x\"> b"]);
}

#[test]
fn html_block_shows_its_lines_verbatim_without_wrapping() {
	let markdown = "before\n\n<div>\n  <p>a line far longer than the width</p>\n</div>\n\nafter";
	assert_eq!(plain_lines_with(markdown, 20, LITERAL), ["before", "", "<div>", "  <p>a line far longer than the width</p>", "</div>", "", "after"]);
}

#[test]
fn html_block_accepts_every_line_ending() {
	assert_eq!(plain_lines_with("<div>\r\nx\ry\r\n</div>", 80, LITERAL), ["<div>", "x", "y", "</div>"]);
}

#[test]
fn html_block_in_blockquote_and_list() {
	assert_snapshot!(render_with_options("> <div>\n> quoted\n> </div>\n\n- item\n\n  <div>\n  nested\n  </div>", 80, LITERAL));
}

#[test]
fn html_comments_show_as_written() {
	assert_eq!(
		plain_lines_with("<!-- block --> trailing\n\ntext <!-- inline --> more", 80, LITERAL),
		["<!-- block --> trailing", "", "text <!-- inline --> more"]
	);
}

#[test]
fn autolinks_stay_links_with_literal_html() {
	let markdown = "See <https://example.com> now.";
	assert_eq!(render_with_options(markdown, 80, LITERAL), render_with_options(markdown, 80, RenderOptions::default()));
	assert_snapshot!(render_with_options(markdown, 80, LITERAL), @"See [underline]https://example.com[] now.");
}

#[test]
fn inline_html_in_table_cells() {
	let markdown = "| a <b>x</b> |\n|---|\n| <br> |";
	assert_snapshot!(render_with_options(markdown, 80, LITERAL));
	assert_snapshot!(render_with_options(markdown, 80, RenderOptions::default()));
}

#[test]
fn dropped_html_leaves_its_line_empty_when_soft_breaks_break() {
	// The line endings around a line of dropped HTML still break, so the
	// line it stood on stays, empty.
	assert_eq!(plain_lines_with("a\n<span>\nb", 80, BREAK), ["a", "", "b"]);
	assert_eq!(plain_lines_with("a\n<span>\nb", 80, BREAK_AND_LITERAL), ["a", "<span>", "b"]);
}

// =========================================================
// Glued inline pieces wrap as one word
// =========================================================

// markdown-rs makes each inline element its own node, so `foo**bar**` is
// two nodes without whitespace between them. A line may only break where
// the source has whitespace or a line break. Each test below picks the
// width at which "aaaa" plus the first glued piece fits exactly, so the
// next piece is the first one that does not fit.

#[test]
fn text_glued_to_bold_wraps_as_one_word() {
	assert_eq!(render_with_width("aaaa foo**bar**", 8), "aaaa\nfoo[bold]bar[]\n");
}

#[test]
fn bold_glued_to_text_wraps_as_one_word() {
	assert_eq!(render_with_width("aaaa **foo**bar", 8), "aaaa\n[bold]foo[]bar\n");
}

#[test]
fn text_glued_to_inline_code_wraps_as_one_word() {
	assert_eq!(render_with_width("aaaa x`code`", 6), "aaaa\nx[yellow]code[]\n");
}

#[test]
fn link_url_suffix_glued_to_text_wraps_as_one_word() {
	// The space in front of `(u)` is the renderer's, not the source's, so
	// the line may still break there; it may not break after `(u)`.
	assert_eq!(render_with_width("aaaa [link](u)s", 13), "aaaa [underline]link[]\n[dim](u)[]s\n");
}

#[test]
fn text_glued_to_inline_html_wraps_as_one_word() {
	assert_eq!(plain_lines_with("Use a Vec<String>", 11, LITERAL), ["Use a", "Vec<String>"]);
}

#[test]
fn glued_word_wider_than_the_line_overflows_on_its_own_line() {
	// Like a single long word: it moves off a line that has content and
	// is never broken inside, so it overflows the width.
	assert_eq!(render_with_width("aaaa foo**barbazqux**", 8), "aaaa\nfoo[bold]barbazqux[]\n");
	assert_eq!(render_with_width("foo**barbazqux** b", 8), "foo[bold]barbazqux[]\nb\n");
}

#[test]
fn glued_words_never_split_at_any_width() {
	// Each document glues pieces into the word on the right; whatever the
	// width, that word shows up whole on one line.
	let cases = [
		("one two foo**bar** three", "foobar", RenderOptions::default()),
		("one **two**three four", "twothree", RenderOptions::default()),
		("one a`b`c two", "abc", RenderOptions::default()),
		("one ~~x~~*y*z two", "xyz", RenderOptions::default()),
		("one [link](u)s two", "(u)s", RenderOptions::default()),
		("one a![alt](u)", "a[image:", RenderOptions::default()),
		("one Vec<String> two", "Vec<String>", LITERAL),
		("- one foo**bar** two", "foobar", RenderOptions::default()),
		("> one foo**bar** two", "foobar", RenderOptions::default()),
		("# one foo**bar** two", "foobar", RenderOptions::default()),
	];
	for (markdown, word, options) in cases {
		for width in 1..=30 {
			let lines = plain_lines_with(markdown, width, options);
			assert!(lines.iter().any(|line| line.contains(word)), "{word:?} split in {markdown:?} at width {width}:\n{}", lines.join("\n"));
		}
	}
}

#[test]
fn glued_pieces_keep_their_own_styles() {
	assert_eq!(render("x **a b**c d ~~e~~`f`[g](u)`h`"), "x [bold]a b[]c d [strike]e[][yellow]f[][underline]g[][dim] (u)[][yellow]h[]\n");
}

#[test]
fn url_suffix_inside_a_glued_run_moves_with_its_tail() {
	assert_eq!(render_with_width("`a`[b](u)`c`", 4), "[yellow]a[][underline]b[]\n[dim](u)[][yellow]c[]\n");
	assert_eq!(plain_lines("x![alt](u)y", 8), ["x[image:", "alt]", "(u)y"]);
}

#[test]
fn whitespace_between_inline_pieces_still_breaks_the_line() {
	assert_eq!(render_with_width("aaaa foo **bar** baz", 8), "aaaa foo\n[bold]bar[] baz\n");
	assert_eq!(render_with_width("aaaa **foo** bar", 8), "aaaa [bold]foo[]\nbar\n");
}

#[test]
fn line_breaks_end_a_glued_run() {
	assert_eq!(render("a**b**\\\nc"), "a[bold]b[]\nc\n");
	assert_eq!(render_with_options("a**b**\nc", 80, BREAK), "a[bold]b[]\nc\n");
}

#[test]
fn glued_run_in_a_list_item_wraps_under_the_item_text() {
	assert_eq!(plain_lines("- aaaa foo**bar** baz", 10), ["• aaaa", "  foobar", "  baz"]);
}
