// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! The style-neutral output model: lines of text spans, each carrying the
//! complete style it is drawn with. The renderer builds these lines; the
//! ANSI writer in `ansi.rs` turns them into escape sequences, and other
//! front ends (a ratatui application) map them onto their own types.
//! Design rationale lives in
//! `docs/adr/0006-render-to-style-neutral-lines.md`.

use std::fmt;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// =========================================================
// Colors and underline
// =========================================================

/// A color from the 8-color ANSI palette, named after its standard SGR
/// meaning.
///
/// patine never emits absolute RGB colors: each color is a palette slot
/// that the user's terminal theme resolves. A front end keeps that
/// property by drawing the slot rather than a fixed RGB value, for
/// example with ratatui's `Color::Indexed(color.index())`.
///
/// The enum is exhaustive, so a front end can match on every color. A new
/// variant is a breaking change and only comes in a major release.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnsiColor {
	/// Palette index 0.
	Black,
	/// Palette index 1.
	Red,
	/// Palette index 2.
	Green,
	/// Palette index 3, used for inline code and unhighlighted code blocks.
	Yellow,
	/// Palette index 4.
	Blue,
	/// Palette index 5.
	Magenta,
	/// Palette index 6.
	Cyan,
	/// Palette index 7, which ratatui calls `Color::Gray`.
	White,
}

impl AnsiColor {
	/// The palette index of this color, from 0 (`Black`) to 7 (`White`).
	///
	/// ```
	/// use patine::AnsiColor;
	///
	/// assert_eq!(AnsiColor::Yellow.index(), 3);
	/// ```
	pub const fn index(self) -> u8 {
		self as u8
	}

	/// The color at palette index `index`, or `None` outside 0 to 7.
	pub(crate) const fn from_index(index: u8) -> Option<Self> {
		Some(match index {
			0 => Self::Black,
			1 => Self::Red,
			2 => Self::Green,
			3 => Self::Yellow,
			4 => Self::Blue,
			5 => Self::Magenta,
			6 => Self::Cyan,
			7 => Self::White,
			_ => return None,
		})
	}
}

impl From<AnsiColor> for u8 {
	/// The palette index, as returned by [`AnsiColor::index`].
	fn from(color: AnsiColor) -> u8 {
		color.index()
	}
}

/// How text is underlined.
///
/// The enum is exhaustive, so a front end can match on every kind. A new
/// variant is a breaking change and only comes in a major release.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Underline {
	/// Not underlined.
	#[default]
	None,
	/// A single underline, used for links and level-two headings.
	Single,
	/// A double underline, used for level-one headings. Terminals without
	/// double underline support draw a single one instead, so a level-one
	/// heading is also italic to stay distinct from a level-two one.
	Double,
}

// =========================================================
// Text style
// =========================================================

/// The complete style of a piece of text.
///
/// The default is the terminal's plain text: no attributes and the
/// default foreground color. New fields may be added in minor releases,
/// so outside patine a style is built from the default:
///
/// ```
/// use patine::{AnsiColor, TextStyle};
///
/// let mut keyword = TextStyle::default();
/// keyword.bold = true;
/// keyword.foreground = Some(AnsiColor::Magenta);
/// ```
///
/// The values the existing fields can take change only in a major
/// release: a new variant of [`Underline`] or [`AnsiColor`] is a breaking
/// change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct TextStyle {
	/// Bold, or increased intensity.
	pub bold: bool,
	/// Italic.
	pub italic: bool,
	/// Dim, or decreased intensity. Can be combined with `bold`.
	pub dim: bool,
	/// Crossed out.
	pub strikethrough: bool,
	/// Underline kind, [`Underline::None`] for none.
	pub underline: Underline,
	/// Foreground color; `None` keeps the terminal's default foreground.
	/// patine never sets a background color.
	pub foreground: Option<AnsiColor>,
}

impl TextStyle {
	/// This style with every attribute that `top` sets laid over it.
	/// Attributes `top` leaves unset keep their value from `self`, so an
	/// outer context (a done task's dim) survives inside an inner one (a
	/// bold table header).
	pub(crate) fn patch(self, top: TextStyle) -> TextStyle {
		TextStyle {
			bold: self.bold || top.bold,
			italic: self.italic || top.italic,
			dim: self.dim || top.dim,
			strikethrough: self.strikethrough || top.strikethrough,
			underline: match top.underline {
				Underline::None => self.underline,
				underline => underline,
			},
			foreground: top.foreground.or(self.foreground),
		}
	}
}

// =========================================================
// Width
// =========================================================

/// Halfwidth Katakana Voiced Sound Mark, `ﾞ`.
const HALFWIDTH_VOICED_SOUND_MARK: char = '\u{FF9E}';
/// Halfwidth Katakana Semi-Voiced Sound Mark, `ﾟ`.
const HALFWIDTH_SEMI_VOICED_SOUND_MARK: char = '\u{FF9F}';

/// The columns `text` takes in a terminal. Every width patine lays out is
/// measured here.
///
/// Terminals and ratatui draw text grapheme by grapheme, so this is the
/// sum of the Unicode widths of the extended grapheme clusters of `text`.
/// unicode-width gives some sequences of graphemes a width of their own:
/// the Lam-Alef ligature `لا` is one column as a string, but its two
/// graphemes are drawn in a column each.
///
/// Each halfwidth sound mark, `ﾞ` or `ﾟ`, takes one column more.
/// unicode-width gives them none, as they extend the grapheme before
/// them, but terminals and ratatui draw each in a cell of its own. The
/// combining sound marks U+3099 and U+309A keep their Unicode width.
///
/// Control characters keep their Unicode width; a front end decides how
/// to draw them.
pub(crate) fn text_width(text: &str) -> usize {
	text.graphemes(true).map(grapheme_width).sum()
}

/// The columns one extended grapheme cluster takes, as [`text_width`]
/// counts them.
fn grapheme_width(grapheme: &str) -> usize {
	let sound_marks = grapheme.chars().filter(|&c| c == HALFWIDTH_VOICED_SOUND_MARK || c == HALFWIDTH_SEMI_VOICED_SOUND_MARK).count();
	UnicodeWidthStr::width(grapheme) + sound_marks
}

// =========================================================
// Spans and lines
// =========================================================

/// A run of text drawn in one style.
///
/// The text never contains a line ending, `\n` or `\r`. It may contain
/// tab characters from code blocks, which patine passes through verbatim.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct StyledSpan {
	/// The text of the span.
	pub text: String,
	/// The style the whole text is drawn in.
	pub style: TextStyle,
}

impl StyledSpan {
	/// A span of `text` drawn in `style`.
	pub fn new(text: impl Into<String>, style: TextStyle) -> Self {
		Self { text: text.into(), style }
	}

	/// Display width of the text in terminal columns, counted grapheme by
	/// grapheme as terminals and ratatui draw it: the letters of a
	/// Lam-Alef `لا` take a column each, and a halfwidth sound mark, `ﾞ`
	/// or `ﾟ`, takes one column. ratatui's own `Span::width` measures the
	/// whole string with unicode-width and can differ.
	pub fn width(&self) -> usize {
		text_width(&self.text)
	}
}

impl fmt::Display for StyledSpan {
	/// Writes the plain text, without any styling.
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.text)
	}
}

/// One output line: its spans in reading order, without a trailing
/// newline. A blank line has no spans.
///
/// Lines that patine produces never hold two adjacent spans of the same
/// style.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct StyledLine {
	/// The spans of the line, in reading order.
	pub spans: Vec<StyledSpan>,
}

impl StyledLine {
	/// A line made of `spans`, kept exactly as given.
	pub fn new(spans: Vec<StyledSpan>) -> Self {
		Self { spans }
	}

	/// Display width of the line in terminal columns, the sum of its
	/// spans' widths.
	pub fn width(&self) -> usize {
		self.spans.iter().map(StyledSpan::width).sum()
	}

	/// Append `text` in `style`. Text that continues the style of the last
	/// span extends that span, so a line holds one span per style change
	/// rather than one per word.
	pub(crate) fn push_str(&mut self, text: &str, style: TextStyle) {
		if text.is_empty() {
			return;
		}
		match self.spans.last_mut() {
			Some(last) if last.style == style => last.text.push_str(text),
			_ => self.spans.push(StyledSpan::new(text, style)),
		}
	}
}

impl fmt::Display for StyledLine {
	/// Writes the plain text of all spans, without any styling.
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.spans.iter().try_for_each(|span| f.write_str(&span.text))
	}
}

impl From<Vec<StyledSpan>> for StyledLine {
	fn from(spans: Vec<StyledSpan>) -> Self {
		Self::new(spans)
	}
}

impl FromIterator<StyledSpan> for StyledLine {
	fn from_iter<I: IntoIterator<Item = StyledSpan>>(spans: I) -> Self {
		Self::new(spans.into_iter().collect())
	}
}

impl IntoIterator for StyledLine {
	type Item = StyledSpan;
	type IntoIter = std::vec::IntoIter<StyledSpan>;

	fn into_iter(self) -> Self::IntoIter {
		self.spans.into_iter()
	}
}

impl<'a> IntoIterator for &'a StyledLine {
	type Item = &'a StyledSpan;
	type IntoIter = std::slice::Iter<'a, StyledSpan>;

	fn into_iter(self) -> Self::IntoIter {
		self.spans.iter()
	}
}

// =========================================================
// Unit tests
// =========================================================

#[cfg(test)]
mod tests {
	use super::*;

	const BOLD: TextStyle = TextStyle { bold: true, italic: false, dim: false, strikethrough: false, underline: Underline::None, foreground: None };

	#[test]
	fn palette_index_round_trips() {
		for index in 0..=7 {
			let color = AnsiColor::from_index(index).expect("0 to 7 are palette colors");
			assert_eq!(color.index(), index);
		}
	}

	#[test]
	fn indices_beyond_the_eight_color_palette_have_no_color() {
		assert_eq!(AnsiColor::from_index(8), None);
		assert_eq!(AnsiColor::from_index(255), None);
	}

	#[test]
	fn palette_order_follows_sgr_numbering() {
		assert_eq!(AnsiColor::Black.index(), 0);
		assert_eq!(AnsiColor::Yellow.index(), 3);
		assert_eq!(AnsiColor::White.index(), 7);
	}

	#[test]
	fn patch_adds_attributes_and_keeps_the_rest() {
		let quote = TextStyle { dim: true, italic: true, foreground: Some(AnsiColor::Yellow), underline: Underline::Double, ..TextStyle::default() };
		let patched = quote.patch(BOLD);
		assert_eq!(patched, TextStyle { bold: true, ..quote });
	}

	#[test]
	fn patch_overrides_color_and_underline_when_set() {
		let base = TextStyle { foreground: Some(AnsiColor::Yellow), underline: Underline::Double, strikethrough: true, ..TextStyle::default() };
		let top = TextStyle { foreground: Some(AnsiColor::Blue), underline: Underline::Single, ..TextStyle::default() };
		let patched = base.patch(top);
		assert_eq!(patched.foreground, Some(AnsiColor::Blue));
		assert_eq!(patched.underline, Underline::Single);
		assert!(patched.strikethrough);
	}

	#[test]
	fn push_str_merges_text_of_the_same_style() {
		let mut line = StyledLine::default();
		line.push_str("one", BOLD);
		line.push_str(" two", BOLD);
		line.push_str(" three", TextStyle::default());
		assert_eq!(
			line.spans,
			vec![StyledSpan { text: "one two".to_string(), style: BOLD }, StyledSpan { text: " three".to_string(), style: TextStyle::default() },]
		);
	}

	// ── Widths ──────────────────────────────────────────────────────────

	#[test]
	fn a_halfwidth_sound_mark_takes_a_column() {
		assert_eq!(StyledSpan::new("ｶﾞ", TextStyle::default()).width(), 2);
	}

	#[test]
	fn a_halfwidth_semi_voiced_sound_mark_takes_a_column() {
		assert_eq!(StyledSpan::new("ﾟ", TextStyle::default()).width(), 1);
	}

	#[test]
	fn a_combining_sound_mark_takes_no_column() {
		assert_eq!(StyledSpan::new("ｶ\u{3099}", TextStyle::default()).width(), 1);
	}

	#[test]
	fn a_ligature_takes_a_column_per_grapheme() {
		assert_eq!(StyledSpan::new("لا", TextStyle::default()).width(), 2);
	}

	#[test]
	fn a_joined_ligature_takes_a_column_per_grapheme() {
		assert_eq!(StyledSpan::new("א\u{200D}ל", TextStyle::default()).width(), 2);
	}

	#[test]
	fn push_str_ignores_empty_text() {
		let mut line = StyledLine::default();
		line.push_str("", BOLD);
		assert!(line.spans.is_empty());
	}
}
