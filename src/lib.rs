// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Render Markdown for the terminal.
//!
//! patine lays a Markdown document out for a given width: headings,
//! wrapped paragraphs, lists, blockquotes, tables and syntax-highlighted
//! code blocks. It styles text with bold, italic, dim, underline,
//! strikethrough and the 8 colors of the ANSI palette, so the user's
//! terminal theme decides the actual colors.
//!
//! There are two ways to use the result:
//!
//! - [`render`] writes the document as text with ANSI escape sequences,
//!   ready for a terminal. This is what the `patine` binary does.
//! - [`render_lines`] returns the document as [`StyledLine`]s, lines of
//!   [`StyledSpan`]s that each carry a complete [`TextStyle`]. An
//!   application that draws its own interface, such as a ratatui
//!   application, maps these onto its own types. [`highlight`] does the
//!   same for source code outside Markdown.
//!
//! [`render_lines_with`] and [`render_with`] take [`RenderOptions`] for
//! text that is not a Markdown document, where a single newline should
//! break the line ([`SoftBreaks`]) and raw HTML should show as written
//! ([`Html`]).
//!
//! ```
//! use patine::{AnsiColor, render_lines};
//!
//! let lines = render_lines("Run `cargo test` **now**.", 80);
//! for span in &lines[0] {
//!     if span.style.foreground == Some(AnsiColor::Yellow) {
//!         assert_eq!(span.text, "cargo test");
//!     }
//! }
//! assert_eq!(lines[0].to_string(), "Run cargo test now.");
//! ```
//!
//! # Features
//!
//! - `cli` (default): builds the `patine` binary. Library users can turn
//!   it off with `default-features = false`, which leaves out `clap` and
//!   `crossterm`.

#![warn(missing_docs)]

mod ansi;
mod ascii;
mod highlight;
mod line_endings;
mod options;
mod render;
mod styled;
mod table;

use std::io::Write;

use anyhow::Result;

pub use crate::options::{Html, QuoteStyle, RenderOptions, SoftBreaks};
pub use crate::styled::{AnsiColor, StyledLine, StyledSpan, TextStyle, Underline};

/// Parse and render a Markdown string as styled terminal output, wrapping
/// text at `terminal_width` columns.
///
/// Writes the lines of [`render_lines`] to `writer`, each followed by a
/// newline, with ANSI SGR escape sequences for their styles. Every span
/// switches its attributes off again, so no style stays active across a
/// line break or after the output ends.
///
/// # Errors
///
/// Fails only when writing to `writer` fails.
///
/// # Examples
///
/// ```
/// let mut output = Vec::new();
/// patine::render("**bold**", &mut output, 80)?;
/// assert_eq!(output, b"\x1b[1mbold\x1b[22m\n");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn render(input: &str, writer: &mut impl Write, terminal_width: usize) -> Result<()> {
	render_with(input, writer, terminal_width, &RenderOptions::default())
}

/// Like [`render`], with [`RenderOptions`] that change how soft line
/// breaks and raw HTML render.
///
/// Writes the lines of [`render_lines_with`].
///
/// # Errors
///
/// Fails only when writing to `writer` fails.
///
/// # Examples
///
/// ```
/// use patine::{RenderOptions, SoftBreaks};
///
/// let options = RenderOptions::new().with_soft_breaks(SoftBreaks::Break);
/// let mut output = Vec::new();
/// patine::render_with("one\ntwo", &mut output, 80, &options)?;
/// assert_eq!(output, b"one\ntwo\n");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn render_with(input: &str, writer: &mut impl Write, terminal_width: usize, options: &RenderOptions) -> Result<()> {
	ansi::write_lines(&render_lines_with(input, terminal_width, options), writer)
}

/// Parse a Markdown string (CommonMark with GitHub extensions and YAML or
/// TOML frontmatter) and lay it out as styled lines for a display `width`
/// columns wide.
///
/// Text wraps at `width`. Lines may still be wider: in running text,
/// words and URLs longer than the width stay whole; code blocks and
/// frontmatter never wrap. A table whose columns would have to break a
/// word is laid out as records, one line per column; a value there
/// breaks a word wider than its room between graphemes, and its line is
/// wider only where one grapheme, such as a wide character or an emoji,
/// is wider than the room it gets. Widths count grapheme by grapheme, as
/// terminals draw text.
/// Line prefixes are spans of their own: indents are unstyled spaces and
/// blockquote bars are `│ ` spans in the style
/// [`RenderOptions::quote_style`] asks for. Blank lines have no
/// spans, and empty input yields no lines.
///
/// A code block whose language syntect fails to highlight is shown in the
/// flat code color, like a block without a language.
///
/// # Examples
///
/// ```
/// use patine::{Underline, render_lines};
///
/// let lines = render_lines("# Title\n\nSome text.", 80);
/// assert_eq!(lines.len(), 3);
/// assert_eq!(lines[0].spans[0].text, "Title");
/// assert_eq!(lines[0].spans[0].style.underline, Underline::Double);
/// assert!(lines[1].spans.is_empty());
/// ```
pub fn render_lines(input: &str, width: usize) -> Vec<StyledLine> {
	render_lines_with(input, width, &RenderOptions::default())
}

/// Like [`render_lines`], with [`RenderOptions`] that change how soft
/// line breaks and raw HTML render.
///
/// The defaults suit Markdown documents. Short text fields, such as text
/// a program writes into a JSON string, often mean a newline as a line
/// break and a `<` as a literal character:
///
/// ```
/// use patine::{Html, RenderOptions, SoftBreaks, render_lines_with};
///
/// let options = RenderOptions::new()
///     .with_soft_breaks(SoftBreaks::Break)
///     .with_html(Html::Literal);
/// let lines = render_lines_with("Returns a Vec<String>.\nSee **below**.", 80, &options);
/// assert_eq!(lines[0].to_string(), "Returns a Vec<String>.");
/// assert_eq!(lines[1].to_string(), "See below.");
/// ```
pub fn render_lines_with(input: &str, width: usize, options: &RenderOptions) -> Vec<StyledLine> {
	// GFM plus frontmatter: `Constructs::gfm()` does not enable the
	// frontmatter construct, so a `---`/`+++` block at the document start
	// would otherwise be parsed as ordinary markdown (setext heading,
	// thematic break) and mangle the metadata.
	let parse_options = markdown::ParseOptions {
		constructs: markdown::Constructs { frontmatter: true, ..markdown::Constructs::gfm() },
		..markdown::ParseOptions::gfm()
	};
	// markdown-rs reports syntax errors only for MDX constructs, which
	// these options leave disabled.
	let tree = markdown::to_mdast(input, &parse_options).expect("Markdown without MDX has no syntax errors");

	let mut renderer = render::Renderer::new(width, *options);
	renderer.render_node(&tree);
	let mut lines = renderer.finish();
	if options.ascii {
		for line in &mut lines {
			ascii::asciify_line(line);
		}
	}
	lines
}

/// Syntax-highlight source code the way patine highlights fenced code
/// blocks, one [`StyledLine`] per line of `code`. A line ends at `\n`,
/// `\r\n` or a lone `\r`, as in CommonMark; a line ending after the last
/// line adds no empty line.
///
/// `language` is a name or file extension as used after a Markdown code
/// fence, such as `rust`, `rs`, `Python` or `py`, matched without regard
/// to case. Tokens get palette colors, bold and italic from the same
/// 8-color theme as code blocks. Lines are neither indented nor wrapped.
///
/// With `None`, a blank or an unknown language, or when syntect fails to
/// highlight the code, every line is returned as unstyled text.
///
/// # Examples
///
/// ```
/// use patine::highlight;
///
/// let lines = highlight("fn main() {}\n", Some("rust"));
/// assert_eq!(lines.len(), 1);
/// assert!(lines[0].spans.iter().any(|span| span.style.foreground.is_some()));
///
/// let plain = highlight("fn main() {}\n", None);
/// assert_eq!(plain[0].spans[0].style, patine::TextStyle::default());
/// ```
pub fn highlight(code: &str, language: Option<&str>) -> Vec<StyledLine> {
	highlight::try_highlight(code, language).unwrap_or_else(|| {
		line_endings::verbatim_lines(code)
			.map(|line| {
				let mut styled = StyledLine::default();
				styled.push_str(line, TextStyle::default());
				styled
			})
			.collect()
	})
}
