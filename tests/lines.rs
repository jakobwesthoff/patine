// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! The public line API as a consumer such as a ratatui application sees
//! it: `render_lines`, `highlight` and the style-neutral types they
//! return. Layout details are pinned by the snapshots in `rendering.rs`;
//! these tests cover the contract of the API itself.

use patine::{AnsiColor, Html, RenderOptions, SoftBreaks, StyledLine, StyledSpan, TextStyle, Underline, highlight, render_lines, render_lines_with};

fn style(edit: impl FnOnce(&mut TextStyle)) -> TextStyle {
	let mut style = TextStyle::default();
	edit(&mut style);
	style
}

fn texts(lines: &[StyledLine]) -> Vec<String> {
	lines.iter().map(ToString::to_string).collect()
}

// =========================================================
// render_lines
// =========================================================

#[test]
fn empty_markdown_has_no_lines() {
	assert!(render_lines("", 80).is_empty());
	assert!(render_lines("  \n\n ", 80).is_empty());
}

#[test]
fn lines_carry_no_newlines_and_blank_lines_have_no_spans() {
	let lines = render_lines("First.\n\nSecond.", 80);
	assert_eq!(texts(&lines), ["First.", "", "Second."]);
	assert!(lines[1].spans.is_empty());
}

#[test]
fn heading_spans_carry_their_complete_style() {
	let lines = render_lines("# Title with **bold**", 80);
	let heading = style(|s| {
		s.bold = true;
		s.italic = true;
		s.underline = Underline::Double;
	});
	assert_eq!(lines, [StyledLine::new(vec![StyledSpan::new("Title with bold", heading)])]);
}

#[test]
fn inline_styles_become_separate_spans() {
	let lines = render_lines("Run `cargo` or ~~not~~ [this](https://x.io).", 80);
	let code = style(|s| s.foreground = Some(AnsiColor::Yellow));
	let struck = style(|s| s.strikethrough = true);
	let link = style(|s| s.underline = Underline::Single);
	let url = style(|s| s.dim = true);
	let plain = TextStyle::default();
	assert_eq!(
		lines[0],
		StyledLine::new(vec![
			// The space before an inline element belongs to the text
			// around it, so it stays plain.
			StyledSpan::new("Run ", plain),
			StyledSpan::new("cargo", code),
			StyledSpan::new(" or ", plain),
			StyledSpan::new("not", struck),
			StyledSpan::new(" ", plain),
			StyledSpan::new("this", link),
			StyledSpan::new(" (https://x.io)", url),
			StyledSpan::new(".", plain),
		])
	);
}

#[test]
fn blockquote_bars_are_cyan_spans_before_upright_text() {
	let lines = render_lines("> quoted", 80);
	assert_eq!(
		lines[0].spans,
		[StyledSpan::new("│ ", style(|s| s.foreground = Some(AnsiColor::Cyan))), StyledSpan::new("quoted", TextStyle::default()),]
	);
}

#[test]
fn wrapped_lines_fit_the_width_and_keep_their_style() {
	let lines = render_lines(&format!("**{}**", "word ".repeat(12).trim()), 20);
	assert!(lines.len() > 1);
	for line in &lines {
		assert!(line.width() <= 20, "{line} is wider than 20 columns");
		for span in line {
			assert!(span.style.bold, "{span:?} lost its bold");
		}
	}
}

#[test]
fn code_blocks_are_indented_by_an_unstyled_span() {
	let lines = render_lines("```rust\nlet x = 1;\n```", 80);
	assert_eq!(lines.len(), 1);
	assert_eq!(lines[0].spans[0], StyledSpan::new("  ", TextStyle::default()));
	assert_eq!(lines[0].to_string(), "  let x = 1;");
	assert!(lines[0].spans.iter().any(|span| span.style.foreground.is_some()), "highlighted tokens carry palette colors: {:?}", lines[0]);
}

#[test]
fn table_header_cells_are_bold() {
	let lines = render_lines("| Name |\n|---|\n| value |", 80);
	let header = &lines[1];
	let bold: Vec<&StyledSpan> = header.spans.iter().filter(|s| s.style.bold).collect();
	assert_eq!(bold.len(), 1, "{header:?}");
	assert_eq!(bold[0].text.trim(), "Name");
}

#[test]
fn width_counts_display_columns_not_bytes() {
	let lines = render_lines("日本語 🎉", 80);
	assert_eq!(lines[0].width(), 9);
	assert_eq!(lines[0].spans[0].width(), 9);
}

#[test]
fn width_counts_each_grapheme_as_terminals_draw_it() {
	let lines = render_lines("لا", 80);
	assert_eq!(lines[0].width(), 2);
}

#[test]
fn width_counts_a_halfwidth_sound_mark_as_terminals_draw_it() {
	let lines = render_lines("ｶﾞ", 80);
	assert_eq!(lines[0].width(), 2);
}

// =========================================================
// Render options
// =========================================================

/// Options for short text fields whose author means every newline, such
/// as JSON strings written by a program. Built in a constant to prove the
/// builders work at compile time.
const FIELD_OPTIONS: RenderOptions = RenderOptions::new().with_soft_breaks(SoftBreaks::Break).with_html(Html::Literal);

#[test]
fn default_options_follow_commonmark_and_drop_html() {
	let options = RenderOptions::default();
	assert_eq!(options.soft_breaks, SoftBreaks::Join);
	assert_eq!(options.html, Html::Drop);
	assert_eq!(RenderOptions::new(), options);
	assert_eq!(SoftBreaks::default(), SoftBreaks::Join);
	assert_eq!(Html::default(), Html::Drop);
}

#[test]
fn builders_set_one_option_each() {
	let breaks = RenderOptions::new().with_soft_breaks(SoftBreaks::Break);
	assert_eq!(breaks.soft_breaks, SoftBreaks::Break);
	assert_eq!(breaks.html, Html::Drop);

	let html = RenderOptions::new().with_html(Html::Literal);
	assert_eq!(html.soft_breaks, SoftBreaks::Join);
	assert_eq!(html.html, Html::Literal);

	assert_eq!(FIELD_OPTIONS.soft_breaks, SoftBreaks::Break);
	assert_eq!(FIELD_OPTIONS.html, Html::Literal);
}

#[test]
fn options_fields_can_be_set_directly() {
	let mut options = RenderOptions::default();
	options.soft_breaks = SoftBreaks::Break;
	options.html = Html::Literal;
	assert_eq!(options, FIELD_OPTIONS);
}

#[test]
fn render_lines_renders_with_the_default_options() {
	let markdown = "one\ntwo <b>three</b>\n\n<div>\nblock\n</div>";
	assert_eq!(render_lines(markdown, 80), render_lines_with(markdown, 80, &RenderOptions::default()));
	assert_eq!(texts(&render_lines(markdown, 80)), ["one two three"]);
}

#[test]
fn field_options_break_lines_and_show_html() {
	let lines = render_lines_with("Use a Vec<String>\nhere.", 80, &FIELD_OPTIONS);
	assert_eq!(texts(&lines), ["Use a Vec<String>", "here."]);
}

#[test]
fn render_with_writes_the_lines_of_render_lines_with() {
	let markdown = "**one**\ntwo <b>";
	let mut output = Vec::new();
	patine::render_with(markdown, &mut output, 80, &FIELD_OPTIONS).expect("writing to a Vec cannot fail");
	assert_eq!(output, b"\x1b[1mone\x1b[22m\ntwo <b>\n");

	let mut joined = Vec::new();
	patine::render(markdown, &mut joined, 80).expect("writing to a Vec cannot fail");
	assert_eq!(joined, b"\x1b[1mone\x1b[22m two\n");
}

// =========================================================
// highlight
// =========================================================

#[test]
fn highlight_colors_known_languages_without_indent() {
	let lines = highlight("fn main() {}\n// done\n", Some("rust"));
	assert_eq!(texts(&lines), ["fn main() {}", "// done"]);
	assert!(lines[0].spans[0].style.foreground.is_some(), "{:?}", lines[0]);
}

#[test]
fn highlight_accepts_extensions_and_names() {
	for language in ["rs", "Rust", "RUST"] {
		assert_eq!(highlight("let x = 1;", Some(language)), highlight("let x = 1;", Some("rust")), "{language}");
	}
}

#[test]
fn highlight_without_a_known_language_returns_unstyled_lines() {
	let code = "first line\n\nthird line";
	let plain = vec![
		StyledLine::new(vec![StyledSpan::new("first line", TextStyle::default())]),
		StyledLine::default(),
		StyledLine::new(vec![StyledSpan::new("third line", TextStyle::default())]),
	];
	for language in [None, Some(""), Some("  "), Some("no-such-language")] {
		assert_eq!(highlight(code, language), plain, "{language:?}");
	}
}

#[test]
fn highlight_of_empty_code_has_no_lines() {
	assert!(highlight("", Some("rust")).is_empty());
	assert!(highlight("", None).is_empty());
}

#[test]
fn highlight_splits_at_every_line_ending() {
	// `\n`, `\r\n` and a lone `\r` each end a line, as in CommonMark. A
	// line ending after the last line adds no empty line, and no line
	// keeps a `\r`.
	for language in [Some("rust"), None] {
		for code in ["let a = 1;\nlet b = 2;\n", "let a = 1;\r\nlet b = 2;\r\n", "let a = 1;\rlet b = 2;\r"] {
			let lines = highlight(code, language);
			assert_eq!(texts(&lines), ["let a = 1;", "let b = 2;"], "{code:?} as {language:?}");
		}
	}
	assert_eq!(texts(&highlight("a\r\rb", None)), ["a", "", "b"]);
}

// =========================================================
// Types
// =========================================================

#[test]
fn default_style_is_plain_text() {
	let plain = TextStyle::default();
	assert!(!plain.bold && !plain.italic && !plain.dim && !plain.strikethrough);
	assert_eq!(plain.underline, Underline::None);
	assert_eq!(plain.foreground, None);
}

#[test]
fn palette_colors_map_to_their_sgr_index() {
	let colors = [
		AnsiColor::Black,
		AnsiColor::Red,
		AnsiColor::Green,
		AnsiColor::Yellow,
		AnsiColor::Blue,
		AnsiColor::Magenta,
		AnsiColor::Cyan,
		AnsiColor::White,
	];
	for (index, color) in (0u8..).zip(colors) {
		assert_eq!(color.index(), index);
		assert_eq!(u8::from(color), index);
	}
}

#[test]
fn lines_build_from_spans_and_iterate_over_them() {
	let bold = style(|s| s.bold = true);
	let spans = vec![StyledSpan::new("one ", TextStyle::default()), StyledSpan::new("two", bold)];
	let line = StyledLine::from(spans.clone());
	assert_eq!(line, StyledLine::new(spans.clone()));
	assert_eq!(line, spans.iter().cloned().collect::<StyledLine>());
	assert_eq!((&line).into_iter().collect::<Vec<_>>(), spans.iter().collect::<Vec<_>>());
	assert_eq!(line.clone().into_iter().collect::<Vec<_>>(), spans);
	assert_eq!(line.to_string(), "one two");
	assert_eq!(spans[1].to_string(), "two");
	assert_eq!(line.width(), 7);
}

#[test]
fn types_are_send_and_sync() {
	fn assert_send_sync<T: Send + Sync>() {}
	assert_send_sync::<StyledLine>();
	assert_send_sync::<StyledSpan>();
	assert_send_sync::<TextStyle>();
	assert_send_sync::<Underline>();
	assert_send_sync::<AnsiColor>();
	assert_send_sync::<RenderOptions>();
	assert_send_sync::<SoftBreaks>();
	assert_send_sync::<Html>();
}
