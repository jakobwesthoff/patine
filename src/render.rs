// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use markdown::mdast::Node;
use unicode_segmentation::UnicodeSegmentation;

use crate::line_endings::{split_line_endings, verbatim_lines};
use crate::options::{Html, QuoteStyle, RenderOptions, SoftBreaks};
use crate::styled::{AnsiColor, StyledLine, TextStyle, Underline, text_width};

/// Columns of indent for one level of nesting: a code block, or a block
/// nested in a list item.
const NEST_INDENT_WIDTH: usize = 2;

/// Foreground color used for inline code and code blocks.
const CODE_COLOR: AnsiColor = AnsiColor::Yellow;

/// Prefix for each blockquote nesting level.
const BLOCKQUOTE_BAR: &str = "│ ";

/// The marker of an unordered list item.
const BULLET: &str = "•";

/// The marker of an open task list item, in place of the bullet.
const TASK_OPEN: &str = "☐";

/// The marker of a done task list item, in place of the bullet.
const TASK_DONE: &str = "☑";

// =========================================================
// Line prefix
// =========================================================

/// One level of the line prefix. The renderer keeps them in the order the
/// blocks were entered, which is the order they appear on a line: a code
/// block in a quote sits inside the bar, a quote in a list item after the
/// item's indent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Segment {
	/// Unstyled indent of this many columns: a code block's own indent,
	/// or the indent of a block nested in a list item.
	Nest(usize),
	/// A blockquote's `│ ` bar.
	Quote,
	/// A list item. It adds nothing to the prefix: its bullet is drawn
	/// once, and the blocks nested in it push a `Nest` of their own.
	/// While it is the innermost segment, wrapped lines start `hang`
	/// columns in, past the bullet. `bare` holds until anything is drawn
	/// after the bullet, so a block can follow a bullet that stands alone
	/// without a blank line.
	Item { hang: usize, bare: bool },
}

// =========================================================
// Style tracking
// =========================================================

/// Text attributes that Markdown nodes push onto the style stack while
/// their content renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Style {
	Bold,
	Italic,
	Underline,
	DoubleUnderline,
	Dim,
	Strikethrough,
	ForegroundColor(AnsiColor),
}

// =========================================================
// Inline content on its own
// =========================================================

/// What the renderer does with a word wider than the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LongWords {
	/// The word overflows a line of its own, as in running text.
	Overflow,
	/// The word is broken between characters, as in a value of a table
	/// laid out as records.
	Break,
}

/// Lay out inline content, such as the content of a table cell, as
/// running text `width` columns wide, with `long_words` deciding what
/// happens to a word wider than that. Content that draws nothing yields
/// no lines.
pub(crate) fn render_inline_lines(children: &[Node], width: usize, options: RenderOptions, long_words: LongWords) -> Vec<StyledLine> {
	let mut renderer = Renderer::new(width, options);
	renderer.long_words = long_words;
	renderer.render_inline_content(children);
	renderer.finish()
}

// =========================================================
// Renderer
// =========================================================

/// Walks the Markdown tree and lays it out as styled lines.
///
/// Text goes into the line under construction, each piece styled from the
/// style stack at the time it is written. `write_newline` closes that
/// line. Because every span carries its complete style, a line wrap needs
/// no re-application of styles: the next span is styled from the same
/// stack.
pub struct Renderer {
	/// Finished lines, in output order.
	lines: Vec<StyledLine>,

	/// The line under construction.
	line: StyledLine,

	/// Available content width in columns, excluding the global indent.
	/// Derived from the terminal width at construction time.
	content_width: usize,

	/// Current column position within the content area, 0-based. Tracks
	/// content position *after* the indent prefix (nesting indent +
	/// blockquote bars), so a list item's bullet or number and its
	/// continuation indent count. Wrapping decisions compare `column`
	/// against [`effective_width()`](Self::effective_width), both of which
	/// exclude indent width. This is why setting `column` to a line's
	/// display width (e.g. in `render_code_block`) is correct despite the
	/// indent having just been written.
	column: usize,

	/// The column where the text of the current line starts: past a list
	/// item's bullet or number on its first line, past the continuation
	/// indent on its later lines, 0 everywhere else. A line whose
	/// `column` is still here holds only its prefix and is empty for
	/// wrapping: the next word goes on it whatever its width, without a
	/// space in front.
	text_start: usize,

	/// Number of consecutive newlines written since the last non-newline
	/// output. Used to avoid emitting duplicate blank lines.
	consecutive_newlines: usize,

	/// True until the first visible content is written, which keeps a
	/// block at the top of the output from starting with a blank line.
	/// Also true on the line under a list item's bullet or number when a
	/// block starts there, so that the block follows the bullet without a
	/// blank line.
	at_start: bool,

	/// The word under construction, not yet placed on a line. markdown-rs
	/// makes every inline element its own node, so `foo**bar**`, `Vec`
	/// followed by inline HTML `<String>` or link text followed by its
	/// `(url)` suffix arrive as separate pieces. They collect here, each in
	/// its own style, until whitespace or the end of the line closes the
	/// word, and then wrap as one: a line breaks only where the source has
	/// whitespace or a line break.
	word: StyledLine,

	/// When set, the buffered [`word`](Self::word) is preceded by a space
	/// (unless it falls at the start of a line), drawn in the held style.
	/// Set by whitespace in text nodes and by soft breaks, so that spacing
	/// between adjacent inline nodes is preserved. The style is the one
	/// active where the whitespace occurs: a space in front of `**bold**`
	/// stays plain, a space between two struck words stays struck.
	pending_space: Option<TextStyle>,

	/// Stack of active inline styles. Styles are pushed when entering a
	/// styled node and popped when leaving. Content written in between is
	/// styled from the whole stack (see `current_style`).
	style_stack: Vec<Style>,

	/// The blocks around the current position, outermost first. They make
	/// up the prefix of every line, its width and the continuation indent
	/// of wrapped list item text.
	prefix: Vec<Segment>,

	/// How soft line breaks and raw HTML render.
	options: RenderOptions,

	/// True while the inline content of a paragraph or heading renders.
	/// markdown-rs represents inline and block HTML as the same node
	/// type, so only the place in the tree tells them apart.
	in_inline_content: bool,

	/// What happens to a word wider than the line.
	long_words: LongWords,
}

impl Renderer {
	pub fn new(terminal_width: usize, options: RenderOptions) -> Self {
		let content_width = terminal_width;
		Self {
			lines: Vec::new(),
			line: StyledLine::default(),
			content_width,
			column: 0,
			text_start: 0,
			consecutive_newlines: 0,
			at_start: true,
			word: StyledLine::default(),
			pending_space: None,
			style_stack: Vec::new(),
			prefix: Vec::new(),
			options,
			in_inline_content: false,
			long_words: LongWords::Overflow,
		}
	}

	/// Finalize the output and return its lines. Closes a line still under
	/// construction; empty input produces no lines.
	pub fn finish(mut self) -> Vec<StyledLine> {
		if !self.at_start && self.consecutive_newlines == 0 {
			self.write_newline();
		}
		self.lines
	}

	// =========================================================
	// Node dispatch
	// =========================================================

	pub fn render_node(&mut self, node: &Node) {
		match node {
			Node::Root(root) => self.render_children(&root.children),
			Node::Heading(h) => self.render_heading(h),
			Node::Paragraph(p) => self.render_paragraph(p),
			Node::Text(t) => self.render_running_text(&t.value),
			Node::Strong(s) => self.render_strong(&s.children),
			Node::Emphasis(e) => self.render_emphasis(&e.children),
			Node::Break(_) => self.write_hard_break(),
			Node::InlineCode(c) => self.render_inline_code(&c.value),
			Node::List(l) => self.render_list(l, false),
			Node::Blockquote(bq) => self.render_blockquote(&bq.children),
			Node::Code(c) => self.render_fenced_text(&c.value, c.lang.as_deref()),
			Node::Yaml(y) => self.render_frontmatter(&y.value, "---", "yaml"),
			Node::Toml(t) => self.render_frontmatter(&t.value, "+++", "toml"),
			Node::Table(t) => self.render_table(t),
			Node::ThematicBreak(_) => self.render_thematic_break(),
			Node::Link(l) => self.render_link(l),
			Node::Delete(d) => self.render_strikethrough(&d.children),
			Node::Image(img) => self.render_image(img),
			Node::Html(html) => self.render_html(&html.value),
			// Fallback: render children if the node has any, silently skip
			// leaf nodes we don't handle yet.
			other => {
				if let Some(children) = other.children() {
					self.render_children(children);
				}
			}
		}
	}

	fn render_children(&mut self, children: &[Node]) {
		for child in children {
			self.render_node(child);
		}
	}

	// =========================================================
	// Block-level rendering
	// =========================================================

	fn render_heading(&mut self, heading: &markdown::mdast::Heading) {
		self.ensure_block_spacing();
		self.write_indent();

		// Heading hierarchy: every level is bold (consistent baseline),
		// and underline decorations create the H1/H2/H3+ visual ladder.
		// H1 additionally uses italic so it remains distinct from H2 even
		// on terminals that fall back from `4:2` (double underline) to a
		// plain single underline.
		let styles: &[Style] = match heading.depth {
			1 => &[Style::Bold, Style::Italic, Style::DoubleUnderline],
			2 => &[Style::Bold, Style::Underline],
			_ => &[Style::Bold],
		};
		for &style in styles {
			self.push_style(style);
		}
		self.render_inline_content(&heading.children);
		for _ in styles {
			self.pop_style();
		}

		self.write_newline();
	}

	/// Render verbatim text as a code-block-shaped element. Serves both
	/// fenced/indented code blocks and frontmatter blocks.
	///
	/// When `lang` names a recognized language identifier
	/// (e.g. `` ```rust ``), tokens are syntax-highlighted using the
	/// bat-authored `Ansi` theme (ANSI palette indices 0–7) so that
	/// colors resolve against the user's terminal theme. When the
	/// language is missing or unknown, or when highlighting fails, the
	/// block falls back to the flat `CODE_COLOR` rendering. In both
	/// cases, content is written verbatim with each line indented one
	/// level and never word-wrapped.
	fn render_fenced_text(&mut self, value: &str, lang: Option<&str>) {
		self.ensure_block_spacing();

		// Code blocks get an extra 2-space indent beyond the global
		// indent to visually separate them from surrounding prose.
		// The indent is applied around whichever rendering branch
		// runs, so the structural layout stays identical between
		// highlighted and fallback modes.
		self.with_segment(Segment::Nest(NEST_INDENT_WIDTH), |renderer| {
			// A block that syntect fails to highlight renders like one
			// without a language rather than failing the whole render.
			match crate::highlight::try_highlight(value, lang) {
				Some(lines) => renderer.render_code_block_highlighted(&lines),
				None => renderer.render_code_block_plain(value),
			}
		});
	}

	/// Render a YAML (`---`) or TOML (`+++`) frontmatter block as a
	/// syntax-highlighted verbatim block, visually identical to a fenced
	/// code block of that language.
	///
	/// The mdast node value carries only the content between the fences;
	/// the fence lines are reconstructed around it so the rendered block
	/// mirrors the source document. Empty frontmatter carries no metadata
	/// and renders nothing at all.
	fn render_frontmatter(&mut self, value: &str, fence: &str, lang: &str) {
		if value.trim().is_empty() {
			return;
		}
		let fenced = format!("{fence}\n{value}\n{fence}");
		self.render_fenced_text(&fenced, Some(lang));
	}

	/// Flat-color code block rendering used for fences without a
	/// recognized language identifier.
	fn render_code_block_plain(&mut self, value: &str) {
		self.push_style(Style::ForegroundColor(CODE_COLOR));
		for line in verbatim_lines(value) {
			self.write_indent();
			self.emit(line);
			self.column = text_width(line);
			self.mark_drawn();
			self.write_newline();
		}
		self.pop_style();
	}

	/// Syntax-highlighted code block rendering. Each token keeps the
	/// surrounding context style and adds its own color, bold and italic
	/// on top.
	fn render_code_block_highlighted(&mut self, lines: &[StyledLine]) {
		let base = self.current_style();
		for line in lines {
			self.write_indent();
			let mut column: usize = 0;
			for span in &line.spans {
				self.line.push_str(&span.text, base.patch(span.style));
				column += text_width(&span.text);
			}
			self.column = column;
			self.mark_drawn();
			self.write_newline();
		}
	}

	fn render_table(&mut self, table: &markdown::mdast::Table) {
		self.ensure_block_spacing();

		// The table module lays out the table as a list of styled content
		// lines (box-drawing characters, cell text, bold headers, or the
		// lines of records) without any per-line prefix. Each cell is laid out as running text by a
		// renderer of its own, so it keeps its inline styles and wraps by
		// the same rules. We walk the lines here so that each one receives
		// the renderer's normal line prefix — nesting indent *and*
		// blockquote bars — via `write_indent`, and the context style
		// under its own.
		let base = self.current_style();
		let options = self.options;
		let lines = crate::table::layout_table(table, self.effective_width(), |cell, width, long_words| {
			render_inline_lines(cell, width, options, long_words)
		});
		for line in lines {
			self.write_indent();
			for span in &line.spans {
				self.line.push_str(&span.text, base.patch(span.style));
			}
			// Table rows occupy full content lines; once the line is
			// emitted, the next call must treat us as if we're mid-line
			// so `write_newline` can terminate the row cleanly. The
			// subsequent `write_newline` resets `column` back to 0.
			self.column = 0;
			self.mark_drawn();
			self.write_newline();
		}
	}

	fn render_blockquote(&mut self, children: &[Node]) {
		// Block spacing must be ensured BEFORE entering the blockquote
		// context. Otherwise the blank separator line would get the │ bar
		// prefix, which visually belongs inside the blockquote, not between
		// the preceding block and the blockquote.
		self.ensure_block_spacing();
		// The quoted text keeps the style around it; only the bars mark
		// the quote.
		self.with_segment(Segment::Quote, |renderer| {
			renderer.render_children(children);
		});
	}

	/// Render a list. A list `directly_in_item`, nested in a list item
	/// without a block between them, follows the item's text without a
	/// blank line: the item's content already ended with a line break.
	/// Every other list, such as one in a quote or footnote that itself
	/// sits in a list item, is spaced from what comes before it.
	fn render_list(&mut self, list: &markdown::mdast::List, directly_in_item: bool) {
		if !directly_in_item {
			self.ensure_block_spacing();
		}
		let mut number = list.start.unwrap_or(1);

		for child in &list.children {
			if let Node::ListItem(item) = child {
				self.render_list_item(item, list.ordered, number);
				number += 1;
			}
		}
	}

	fn render_list_item(&mut self, item: &markdown::mdast::ListItem, ordered: bool, number: u32) {
		self.ensure_line_break();
		self.write_indent();

		// Write the bullet or number prefix. A task item shows its box in
		// place of the bullet, or after the number of an ordered item.
		let task_box = item.checked.map(|done| if done { TASK_DONE } else { TASK_OPEN });
		let prefix = match (ordered, task_box) {
			(false, None) => format!("{BULLET} "),
			(false, Some(task_box)) => format!("{task_box} "),
			(true, None) => format!("{number}. "),
			(true, Some(task_box)) => format!("{number}. {task_box} "),
		};

		// A done task recedes: its whole line, number and box included,
		// is dim.
		let done = item.checked == Some(true);
		if done {
			self.push_style(Style::Dim);
		}
		self.emit(&prefix);
		if done {
			self.pop_style();
		}
		let prefix_width = text_width(&prefix);
		self.column += prefix_width;
		self.text_start = self.column;
		self.mark_drawn();

		// While the item is the innermost segment, wrapped lines align
		// past the bullet/number prefix.
		let segment = Segment::Item { hang: prefix_width, bare: true };
		self.with_segment(segment, |renderer| {
			renderer.render_list_item_children(&item.children, done);
		});
		self.ensure_line_break();
	}

	/// Render the blocks of a list item. Everything in a `done` task is
	/// dim except its nested lists: each item in them shows its own state,
	/// so an open sub-task does not look done.
	fn render_list_item_children(&mut self, children: &[Node], done: bool) {
		// Render item content. ListItem children are typically Paragraph
		// nodes. We render the first paragraph's inline children
		// directly so that the text starts immediately after the
		// bullet — bypassing the paragraph's own block spacing keeps
		// the bullet and its first line on the same visual row.
		//
		// Subsequent paragraphs (loose lists with multiple paragraphs
		// per item) need a blank-line separator AND must re-emit the
		// continuation indent so they sit past the bullet, aligned
		// with the wrapped lines of the first paragraph. We do this
		// ourselves rather than calling `render_paragraph`, because
		// `render_paragraph`'s `ensure_block_spacing` alone leaves the
		// caret at column 0 — without re-emitting the continuation
		// indent, the second paragraph would render back at the left
		// margin instead of aligned under the bullet. While nothing has
		// been drawn after the bullet (the blocks before drew nothing),
		// the paragraph follows the bullet on the next line without a
		// blank line, as a block in that place does.
		let mut is_first = true;
		for child in children {
			let dim = done && !matches!(child, Node::List(_));
			if dim {
				self.push_style(Style::Dim);
			}
			match child {
				Node::Paragraph(p) => {
					if !is_first {
						if self.innermost_item_is_bare() {
							self.ensure_line_break();
						} else {
							self.ensure_block_spacing();
						}
						self.write_indent();
						if self.continuation_indent() > 0 {
							self.write_continuation_indent();
							self.consecutive_newlines = 0;
							self.at_start = false;
						}
					}
					self.render_inline_content(&p.children);
				}
				// Nested lists and other block content get their own indent.
				// Ensure we're on a new line before starting nested content.
				// The nesting indent is all of their prefix: their wrapped
				// lines start under their own text, so the item's
				// continuation indent is off while they render. A nested
				// list sets one for its own items.
				//
				// A block can't share the bullet's line, as its lines sit
				// at the nesting indent and not past the bullet. While
				// nothing has been drawn after the bullet, the block starts
				// on the next line without a blank line in between, so the
				// bullet stays attached to it: `at_start` makes the block's
				// spacing stand down, as at the top of the output. Whatever
				// the line holds then (the bullet, or the continuation
				// indent after a break in text that drew nothing) is closed
				// first, so the block's prefix starts a line of its own.
				other => {
					if self.innermost_item_is_bare() {
						if !self.line.spans.is_empty() {
							self.write_newline();
						}
						self.at_start = true;
					} else {
						self.ensure_line_break();
					}
					self.with_segment(Segment::Nest(NEST_INDENT_WIDTH), |renderer| match other {
						Node::List(list) => renderer.render_list(list, true),
						_ => renderer.render_node(other),
					});
					// A block that draws nothing leaves `at_start` set,
					// but the bullet above is output that a later block
					// after the list needs its blank line from.
					self.at_start = false;
				}
			}
			if dim {
				self.pop_style();
			}
			is_first = false;
		}
	}

	fn render_thematic_break(&mut self) {
		self.ensure_block_spacing();
		self.write_indent();
		let width = self.effective_width();
		self.emit(&"─".repeat(width));
		self.column = width;
		self.mark_drawn();
		self.write_newline();
	}

	fn render_paragraph(&mut self, paragraph: &markdown::mdast::Paragraph) {
		self.ensure_block_spacing();
		self.write_indent();
		self.render_inline_content(&paragraph.children);
		self.write_newline();
	}

	/// Render an HTML block in [`Html::Literal`] mode: its source lines
	/// verbatim and never wrapped. HTML is part of what the author wrote,
	/// so it sits at the indent and in the style of the text around it.
	fn render_html_block(&mut self, html: &str) {
		self.ensure_block_spacing();
		for line in split_line_endings(html) {
			self.write_indent();
			self.emit(line);
			self.column = text_width(line);
			self.mark_drawn();
			self.write_newline();
		}
	}

	// =========================================================
	// Inline rendering
	// =========================================================

	/// Render the inline children of a paragraph or heading. Paragraphs
	/// and headings never nest inside inline content, so the flag is
	/// simply off again afterwards.
	///
	/// Inline content is the only place words are written, so its end
	/// places the last buffered word. Everything after it (closing the
	/// line, block spacing, the next list bullet) reads `column` and the
	/// newline count, which are only right once that word is on the line.
	fn render_inline_content(&mut self, children: &[Node]) {
		self.in_inline_content = true;
		self.render_children(children);
		self.flush_word();
		self.in_inline_content = false;
	}

	/// Render running text, from a text node or inline HTML. A line
	/// ending in it is a soft break, which [`SoftBreaks`] turns into a
	/// space or a line break.
	fn render_running_text(&mut self, text: &str) {
		match self.options.soft_breaks {
			SoftBreaks::Join => self.render_text(text),
			SoftBreaks::Break => {
				for (index, line) in split_line_endings(text).enumerate() {
					if index > 0 {
						self.write_hard_break();
					}
					self.render_text(line);
				}
			}
		}
	}

	fn render_html(&mut self, html: &str) {
		match self.options.html {
			Html::Drop => {}
			Html::Literal if self.in_inline_content => self.render_running_text(html),
			Html::Literal => self.render_html_block(html),
		}
	}

	/// Render a text node, splitting on whitespace for word wrapping. Leading
	/// and trailing whitespace in the text is tracked via `pending_space` so
	/// that spacing between adjacent inline nodes is preserved.
	fn render_text(&mut self, text: &str) {
		if text.is_empty() {
			return;
		}

		if text.starts_with(char::is_whitespace) {
			self.request_space();
		}

		let mut first = true;
		for word in text.split_whitespace() {
			if first {
				first = false;
			} else {
				self.request_space();
			}
			self.write_word(word);
		}

		if text.ends_with(char::is_whitespace) {
			self.request_space();
		}
	}

	/// Ask for a space before the next word, styled like the text here.
	/// When whitespace on both sides of an element boundary collapses into
	/// one space, the earlier side wins.
	///
	/// Whitespace ends the word under construction, so it is placed first;
	/// the space then belongs to the word that follows.
	fn request_space(&mut self) {
		self.flush_word();
		let style = self.current_style();
		self.pending_space.get_or_insert(style);
	}

	fn render_strong(&mut self, children: &[Node]) {
		self.push_style(Style::Bold);
		self.render_children(children);
		self.pop_style();
	}

	fn render_emphasis(&mut self, children: &[Node]) {
		self.push_style(Style::Italic);
		self.render_children(children);
		self.pop_style();
	}

	fn render_strikethrough(&mut self, children: &[Node]) {
		self.push_style(Style::Strikethrough);
		self.render_children(children);
		self.pop_style();
	}

	fn render_image(&mut self, image: &markdown::mdast::Image) {
		let marker = if image.alt.is_empty() { "[image]".to_string() } else { format!("[image: {}]", image.alt) };
		self.render_text(&marker);
		self.write_dimmed_url_suffix(&image.url);
	}

	fn render_inline_code(&mut self, code: &str) {
		self.push_style(Style::ForegroundColor(CODE_COLOR));
		self.render_text(code);
		self.pop_style();
	}

	/// Render a link as its underlined text followed by its URL suffix.
	///
	/// A link whose text is its URL shows the URL already and gets no
	/// suffix. Autolinks are such links: `<https://example.com>` and a GFM
	/// literal `https://example.com` have the URL as their text, and an
	/// email autolink `<someone@example.com>` has it behind `mailto:`. A
	/// `www.example.com` literal gets `http://` in front of its URL, so its
	/// text differs and it keeps the suffix. The text compared is the text
	/// the link draws, without its styling: raw HTML counts with
	/// [`Html::Literal`] and not with [`Html::Drop`].
	fn render_link(&mut self, link: &markdown::mdast::Link) {
		self.push_style(Style::Underline);
		self.render_children(&link.children);
		self.pop_style();

		let mut text = String::new();
		self.push_drawn_text(&link.children, &mut text);
		let shows_its_url = text == link.url || link.url.strip_prefix("mailto:") == Some(text.as_str());
		if !shows_its_url {
			self.write_dimmed_url_suffix(&link.url);
		}
	}

	/// Append the source text of `nodes` that rendering draws to `text`:
	/// the value of every text, inline code and raw HTML node, except raw
	/// HTML that [`Html::Drop`] leaves out.
	fn push_drawn_text(&self, nodes: &[Node], text: &mut String) {
		for node in nodes {
			match node {
				Node::Html(_) if self.options.html == Html::Drop => {}
				_ => match node.children() {
					Some(children) => self.push_drawn_text(children, text),
					None => text.push_str(&node.to_string()),
				},
			}
		}
	}

	/// Write a URL in dimmed parentheses as a suffix to the previous inline
	/// content (link text or image marker), separated from it by a dimmed
	/// space.
	///
	/// That space is the renderer's, not the source's, so the line may
	/// break there: if the suffix doesn't fit on the current line alongside
	/// the preceding content, it moves to a fresh line. The suffix starts a
	/// word of its own, so text glued to the end of a link (`[link](u)s`)
	/// stays with it. In running text the URL itself is never broken
	/// mid-string — on a narrow terminal it may overflow its own line,
	/// which is preferable to rendering it uncopyable. In a value of a
	/// table laid out as records it breaks like any word wider than its
	/// room (see [`LongWords`]).
	fn write_dimmed_url_suffix(&mut self, url: &str) {
		self.flush_word();

		// The dim separating space replaces whatever whitespace the link
		// text ended with.
		self.push_style(Style::Dim);
		self.pending_space = Some(self.current_style());
		self.write_word(&format!("({url})"));
		self.pop_style();
	}

	// =========================================================
	// Word output with wrapping
	// =========================================================

	/// Add a piece of text without whitespace to the word under
	/// construction, in the style of the style stack.
	fn write_word(&mut self, piece: &str) {
		let style = self.current_style();
		self.word.push_str(piece, style);
	}

	/// Place the word under construction on the line, preceded by a space
	/// if `pending_space` is set. Wraps to a new line first when the whole
	/// word would exceed the available content width. A word wider than
	/// the line still moves off a line that has content. In running text
	/// it then overflows its own line and is never broken inside; with
	/// [`LongWords::Break`] it is broken across lines by
	/// [`place_broken_word`](Self::place_broken_word).
	fn flush_word(&mut self) {
		if self.word.spans.is_empty() {
			return;
		}
		let word = std::mem::take(&mut self.word);
		let width = word.width();

		let line_has_text = self.column > self.text_start;
		if let Some(space_style) = self.pending_space
			&& line_has_text
		{
			if self.column + 1 + width > self.effective_width() {
				self.wrap_line();
			} else {
				self.line.push_str(" ", space_style);
				self.column += 1;
			}
		} else if line_has_text && self.column + width > self.effective_width() {
			// No space requested but the word still doesn't fit.
			self.wrap_line();
		}

		self.pending_space = None;
		if self.long_words == LongWords::Break && self.column + width > self.effective_width() {
			self.place_broken_word(word);
		} else {
			for piece in word {
				self.line.push_str(&piece.text, piece.style);
			}
			self.column += width;
		}
		self.mark_drawn();
	}

	/// Place `word`, which is wider than the rest of the line, across as
	/// many lines as it needs. The word is cut into units (see
	/// [`break_units`]), and a unit that would pass the line's end starts
	/// the next line; a unit wider than a whole line takes a line of its
	/// own and overflows it. Each unit keeps the style of its piece.
	///
	/// The part of the word on a line is measured span by span, as
	/// [`StyledLine::width`] measures it and the table layout with it, so
	/// a line never comes out wider than the check let it be.
	fn place_broken_word(&mut self, word: StyledLine) {
		let limit = self.effective_width();
		let mut start = self.column;
		let mut on_line = StyledLine::default();
		for piece in &word {
			for unit in break_units(&piece.text) {
				let mut extended = on_line.clone();
				extended.push_str(unit, piece.style);
				if start + extended.width() > limit && !on_line.spans.is_empty() {
					self.column = start + on_line.width();
					self.wrap_line();
					start = self.column;
					extended = StyledLine::default();
					extended.push_str(unit, piece.style);
				}
				self.line.push_str(unit, piece.style);
				on_line = extended;
			}
		}
		self.column = start + on_line.width();
	}

	/// Break to a new line and indent it so that wrapped text lines up
	/// with the text above it.
	fn wrap_line(&mut self) {
		self.write_newline();
		self.write_indent();
		// Add continuation indent to align past list bullet/number.
		if self.continuation_indent() > 0 {
			self.write_continuation_indent();
		}
	}

	/// End the line inside a block (a trailing backslash or two spaces in
	/// the source). The text after it continues the same block, so its
	/// line starts where a wrapped line would: under the text of a list
	/// item, not under its bullet.
	fn write_hard_break(&mut self) {
		self.flush_word();
		self.wrap_line();
	}

	// =========================================================
	// Low-level output helpers
	// =========================================================

	/// Content width available after the line prefix (nesting indent and
	/// blockquote bars). A list item's bullet and continuation indent are
	/// not subtracted, since `column` already counts them. Always returns
	/// at least 1 to prevent degenerate behavior (e.g. invisible thematic
	/// breaks, every word on its own line).
	fn effective_width(&self) -> usize {
		let prefix_width: usize = self
			.prefix
			.iter()
			.map(|segment| match segment {
				Segment::Nest(width) => *width,
				Segment::Quote => text_width(BLOCKQUOTE_BAR),
				Segment::Item { .. } => 0,
			})
			.sum();
		self.content_width.saturating_sub(prefix_width).max(1)
	}

	/// Render with `segment` added to the line prefix, and remove it again
	/// afterwards. Every segment enters and leaves the prefix here, so the
	/// stack always matches the blocks being rendered.
	fn with_segment(&mut self, segment: Segment, render: impl FnOnce(&mut Self)) {
		self.prefix.push(segment);
		render(self);
		self.prefix.pop().expect("with_segment pops the segment it pushed");
	}

	/// How far a wrapped line of list item text starts past the prefix:
	/// the item's bullet width while the item itself is rendering, 0 inside
	/// any block nested in it, which has a `Nest` of its own on top.
	fn continuation_indent(&self) -> usize {
		match self.prefix.last() {
			Some(Segment::Item { hang, .. }) => *hang,
			_ => 0,
		}
	}

	/// Whether nothing has been drawn since the bullet of the list item
	/// being rendered.
	fn innermost_item_is_bare(&self) -> bool {
		let innermost_item = self.prefix.iter().rev().find(|segment| matches!(segment, Segment::Item { .. }));
		matches!(innermost_item, Some(Segment::Item { bare: true, .. }))
	}

	/// Record that visible content went onto the line: it ends a run of
	/// blank lines, the start of the output, and the bare bullet of every
	/// list item around it.
	fn mark_drawn(&mut self) {
		self.consecutive_newlines = 0;
		self.at_start = false;
		for segment in &mut self.prefix {
			if let Segment::Item { bare, .. } = segment {
				*bare = false;
			}
		}
	}

	/// Append `text` to the current line in the style of the style stack.
	/// Column and spacing bookkeeping stays with the caller.
	fn emit(&mut self, text: &str) {
		let style = self.current_style();
		self.line.push_str(text, style);
	}

	fn write_newline(&mut self) {
		self.lines.push(std::mem::take(&mut self.line));
		self.column = 0;
		self.text_start = 0;
		self.consecutive_newlines += 1;
	}

	/// Write the whole line prefix.
	fn write_indent(&mut self) {
		self.write_prefix(self.prefix.len());
	}

	/// Write the first `segments` levels of the line prefix, outermost
	/// first: an unstyled indent for a `Nest`, a `│ ` bar for a `Quote`,
	/// nothing for a list item. The prefix is layout, so it never takes on
	/// the style of the content around it.
	fn write_prefix(&mut self, segments: usize) {
		let mut wrote = false;
		for index in 0..segments {
			match self.prefix[index] {
				Segment::Nest(width) => {
					self.line.push_str(&" ".repeat(width), TextStyle::default());
					wrote = true;
				}
				Segment::Quote => {
					self.line.push_str(BLOCKQUOTE_BAR, self.bar_style());
					wrote = true;
				}
				Segment::Item { .. } => {}
			}
		}

		// Only update output-tracking state when we actually wrote something,
		// so that an empty prefix doesn't silently claim output occurred.
		if wrote {
			self.consecutive_newlines = 0;
			self.at_start = false;
			self.pending_space = None;
		}
	}

	/// The style of a blockquote bar, whatever the quoted content looks
	/// like.
	fn bar_style(&self) -> TextStyle {
		let mut style = TextStyle::default();
		match self.options.quote_style {
			QuoteStyle::Bar(color) => style.foreground = Some(color),
			QuoteStyle::DimBar => style.dim = true,
		}
		style
	}

	/// Write the spaces that align a wrapped or follow-up line of a list
	/// item past its bullet or number. Like the line prefix, they are
	/// unstyled.
	fn write_continuation_indent(&mut self) {
		let indent = self.continuation_indent();
		self.line.push_str(&" ".repeat(indent), TextStyle::default());
		self.column += indent;
		self.text_start = self.column;
	}

	/// Insert blank-line separation before a block element. Writes newlines
	/// until there are at least two consecutive ones (i.e., one visible blank
	/// line). Skipped at the very start of the output.
	///
	/// When inside a blockquote, the blank line carries the prefix up to
	/// the innermost `│` bar so that the visual bar column is unbroken.
	/// Indents after that bar would only be trailing blanks.
	fn ensure_block_spacing(&mut self) {
		if self.at_start {
			return;
		}
		if self.consecutive_newlines >= 2 {
			// Block spacing is already satisfied.
			return;
		}
		let innermost_quote = self.prefix.iter().rposition(|segment| *segment == Segment::Quote);
		if let Some(innermost_quote) = innermost_quote {
			if self.consecutive_newlines == 0 {
				self.write_newline();
			}
			// Write a blank line that carries the blockquote bar prefix.
			self.write_prefix(innermost_quote + 1);
			self.write_newline();
			// write_indent resets consecutive_newlines to 0, and the
			// subsequent write_newline sets it to 1. Force it to 2 so that
			// callers see the block spacing as satisfied.
			self.consecutive_newlines = 2;
		} else {
			while self.consecutive_newlines < 2 {
				self.write_newline();
			}
		}
	}

	/// Ensure we're on a fresh line, but don't require a full blank-line gap.
	/// Used between list items which should be visually tight.
	fn ensure_line_break(&mut self) {
		if self.at_start {
			return;
		}
		if self.consecutive_newlines == 0 {
			self.write_newline();
		}
	}

	// =========================================================
	// Style management
	// =========================================================

	fn push_style(&mut self, style: Style) {
		self.style_stack.push(style);
	}

	fn pop_style(&mut self) {
		self.style_stack.pop().expect("style stack is never empty when pop_style is called");
	}

	/// The style for content written now: the union of every attribute on
	/// the stack. Underline and color have one value each, so the innermost
	/// entry wins: a link (single underline) inside an H1 (double underline)
	/// is single-underlined.
	fn current_style(&self) -> TextStyle {
		let mut style = TextStyle::default();
		for entry in &self.style_stack {
			match *entry {
				Style::Bold => style.bold = true,
				Style::Italic => style.italic = true,
				Style::Underline => style.underline = Underline::Single,
				Style::DoubleUnderline => style.underline = Underline::Double,
				Style::Dim => style.dim = true,
				Style::Strikethrough => style.strikethrough = true,
				Style::ForegroundColor(color) => style.foreground = Some(color),
			}
		}
		style
	}
}

// =========================================================
// Breaking words
// =========================================================

/// Cut `text` into the units a word wider than its line breaks into: its
/// extended grapheme clusters, the units a terminal and ratatui draw
/// whole. A letter keeps its combining marks and a halfwidth sound mark,
/// and an emoji its variation selector, skin tone and the emoji joined to
/// it, so `⚠️`, a family emoji or a flag is never cut.
fn break_units(text: &str) -> Vec<&str> {
	text.graphemes(true).collect()
}

// =========================================================
// Unit tests
// =========================================================

#[cfg(test)]
mod tests {
	use markdown::mdast::{Strong, Text};

	use super::*;

	fn text(value: &str) -> Node {
		Node::Text(Text { value: value.to_string(), position: None })
	}

	#[test]
	fn a_word_breaks_between_the_letters_of_a_ligature() {
		let lines = render_inline_lines(&[text("لا")], 1, RenderOptions::default(), LongWords::Break);
		let texts: Vec<String> = lines.iter().map(ToString::to_string).collect();
		assert_eq!(texts, ["ل", "ا"]);
	}

	#[test]
	fn broken_word_stays_within_the_line_across_styles() {
		// A word broken across styles is measured span by span, as its
		// lines are drawn: each span on its own, grapheme by grapheme.
		let word = [Node::Strong(Strong { children: vec![text("ل")], position: None }), text("اب")];
		let lines = render_inline_lines(&word, 2, RenderOptions::default(), LongWords::Break);
		assert!(lines.iter().all(|line| line.width() <= 2), "{lines:?}");
	}
}
