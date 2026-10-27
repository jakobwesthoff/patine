// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Options that change how a document's source turns into lines and how
//! some of it looks. The defaults render Markdown documents. The soft
//! break and HTML alternatives suit short text fields whose author means
//! every newline and every `<`; the quote style suits a front end with a
//! color scheme of its own, for documents and text fields alike.
//! Design rationale lives in
//! `docs/adr/0007-render-options-for-soft-breaks-and-html.md` and
//! `docs/adr/0010-quote-style-option.md`.

use crate::styled::AnsiColor;

/// How [`render_lines_with`](crate::render_lines_with) and
/// [`render_with`](crate::render_with) treat parts of the source that
/// Markdown documents and short text fields need differently.
///
/// The default renders Markdown documents as CommonMark describes them,
/// exactly like [`render_lines`](crate::render_lines). Options may be
/// added in minor releases, so outside patine a value starts from
/// [`RenderOptions::new`] or [`RenderOptions::default`]. The builders are
/// `const`, so options can live in a constant:
///
/// ```
/// use patine::{Html, RenderOptions, SoftBreaks};
///
/// const QUESTION_TEXT: RenderOptions = RenderOptions::new()
///     .with_soft_breaks(SoftBreaks::Break)
///     .with_html(Html::Literal);
///
/// let mut options = RenderOptions::default();
/// options.html = Html::Literal;
/// assert_eq!(options.with_soft_breaks(SoftBreaks::Break), QUESTION_TEXT);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct RenderOptions {
	/// What a single newline inside running text does.
	pub soft_breaks: SoftBreaks,
	/// What happens to raw HTML.
	pub html: Html,
	/// How blockquotes look.
	pub quote_style: QuoteStyle,
	/// Force ASCII-only output: accented letters are transliterated to
	/// their base form (`é` → `e`, `ß` → `ss`), common symbols map to
	/// ASCII (`…` → `.`, `—` → `-`), and anything without an ASCII
	/// equivalent becomes `?`. The `patine` binary exposes this as
	/// `--ascii`.
	pub ascii: bool,
}

impl RenderOptions {
	/// The default options, usable in a `const`.
	pub const fn new() -> Self {
		Self { soft_breaks: SoftBreaks::Join, html: Html::Drop, quote_style: QuoteStyle::DEFAULT, ascii: false }
	}

	/// These options with `ascii` replaced.
	#[must_use]
	pub const fn with_ascii(mut self, ascii: bool) -> Self {
		self.ascii = ascii;
		self
	}

	/// These options with `soft_breaks` replaced.
	#[must_use]
	pub const fn with_soft_breaks(mut self, soft_breaks: SoftBreaks) -> Self {
		self.soft_breaks = soft_breaks;
		self
	}

	/// These options with `html` replaced.
	#[must_use]
	pub const fn with_html(mut self, html: Html) -> Self {
		self.html = html;
		self
	}

	/// These options with `quote_style` replaced.
	#[must_use]
	pub const fn with_quote_style(mut self, quote_style: QuoteStyle) -> Self {
		self.quote_style = quote_style;
		self
	}
}

/// What a single newline inside running text does: a soft line ending in
/// CommonMark's terms, as opposed to a hard break (a trailing backslash or
/// two spaces), which always breaks the line.
///
/// Running text is the text of paragraphs, list items, blockquotes,
/// headings and link text. Code blocks, inline code, image alt text and
/// frontmatter keep their own handling under both variants. A table cell
/// cannot hold a line ending, since each table row is one source line.
///
/// ```
/// use patine::{RenderOptions, SoftBreaks, render_lines_with};
///
/// let text = |lines: Vec<patine::StyledLine>| -> Vec<String> {
///     lines.iter().map(ToString::to_string).collect()
/// };
/// let joined = render_lines_with("one\ntwo", 80, &RenderOptions::new());
/// assert_eq!(text(joined), ["one two"]);
///
/// let options = RenderOptions::new().with_soft_breaks(SoftBreaks::Break);
/// let broken = render_lines_with("one\ntwo", 80, &options);
/// assert_eq!(text(broken), ["one", "two"]);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SoftBreaks {
	/// The lines join into one, separated by a space, and wrap as one
	/// paragraph. This is CommonMark's behavior.
	#[default]
	Join,
	/// The line breaks exactly where a hard break would break it: the next
	/// line starts under the text above, past any list bullet, inside any
	/// blockquote bars. GitHub renders comments this way.
	Break,
}

/// What happens to raw HTML in the source.
///
/// [Autolinks](https://spec.commonmark.org/0.31.2/#autolinks) such as
/// `<https://example.com>` are links, not HTML, and render as links under
/// both variants.
///
/// ```
/// use patine::{Html, RenderOptions, render_lines_with};
///
/// let markdown = "Use a Vec<String> here.";
/// let dropped = render_lines_with(markdown, 80, &RenderOptions::new());
/// assert_eq!(dropped[0].to_string(), "Use a Vec here.");
///
/// let options = RenderOptions::new().with_html(Html::Literal);
/// let literal = render_lines_with(markdown, 80, &options);
/// assert_eq!(literal[0].to_string(), "Use a Vec<String> here.");
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Html {
	/// HTML is left out: inline HTML disappears from its line, and an
	/// HTML block renders nothing.
	#[default]
	Drop,
	/// HTML shows as its source text, comments included. Inline HTML is
	/// drawn in the style around it and wraps like words; a line ending
	/// inside it follows [`SoftBreaks`]. An HTML block keeps its source
	/// lines verbatim and unwrapped, unstyled apart from its context,
	/// without the extra indent of a code block.
	Literal,
}

/// How a blockquote looks: its `│ ` bars and the text inside it.
///
/// Under both variants the quoted text keeps the style around it, so
/// emphasis, bold and inline code look as they do outside a quote. Only
/// the bars differ.
///
/// ```
/// use patine::{AnsiColor, QuoteStyle, RenderOptions, render_lines_with};
///
/// let options = RenderOptions::new().with_quote_style(QuoteStyle::Bar(AnsiColor::Blue));
/// let lines = render_lines_with("> quoted", 80, &options);
/// assert_eq!(lines[0].spans[0].text, "│ ");
/// assert_eq!(lines[0].spans[0].style.foreground, Some(AnsiColor::Blue));
/// assert!(!lines[0].spans[1].style.italic);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum QuoteStyle {
	/// The bars in this palette color and no other attribute. The
	/// default is `Bar(AnsiColor::Cyan)`.
	Bar(AnsiColor),
	/// The bars dim and without a color, for a front end that shows no
	/// colors.
	DimBar,
}

impl QuoteStyle {
	/// The default, usable in a `const` such as [`RenderOptions::new`].
	const DEFAULT: Self = Self::Bar(AnsiColor::Cyan);
}

impl Default for QuoteStyle {
	/// Cyan bars.
	fn default() -> Self {
		Self::DEFAULT
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn new_and_default_options_are_the_same() {
		assert_eq!(RenderOptions::new(), RenderOptions::default());
		assert_eq!(RenderOptions::new().quote_style, QuoteStyle::Bar(AnsiColor::Cyan));
	}
}
