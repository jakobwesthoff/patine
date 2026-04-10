// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::Write;

use anyhow::{Context, Result};
use crossterm::{
    queue,
    style::{Attribute, Color, SetAttribute, SetForegroundColor},
};
use markdown::mdast::Node;
use unicode_width::UnicodeWidthStr;

/// Indentation unit used for nesting (lists, code blocks, etc.).
const NEST_INDENT: &str = "  ";

/// Width of one nesting indent level in columns. Uses `len()` (byte count)
/// rather than `UnicodeWidthStr::width()` because the latter is not `const`.
/// The compile-time assertion below ensures the indent stays ASCII, where
/// byte count and display width are identical.
const NEST_INDENT_WIDTH: usize = NEST_INDENT.len();
const _: () = assert!(NEST_INDENT.is_ascii(), "NEST_INDENT must be ASCII so that len() equals display width");

/// Foreground color used for inline code and code blocks.
const CODE_COLOR: Color = Color::DarkYellow;

// =========================================================
// Style tracking
// =========================================================

/// Terminal text attributes that can be pushed onto the style stack and
/// re-applied after line wraps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Style {
    Bold,
    Italic,
    Underline,
    DoubleUnderline,
    Dim,
    Strikethrough,
    ForegroundColor(Color),
}

// =========================================================
// Renderer
// =========================================================

pub struct Renderer<'w, W: Write> {
    writer: &'w mut W,

    /// Available content width in columns, excluding the global indent.
    /// Derived from the terminal width at construction time.
    content_width: usize,

    /// Current column position within the content area, 0-based. Tracks
    /// content position *after* the indent prefix (nesting indent +
    /// blockquote bars). Wrapping decisions compare `column` against
    /// [`effective_width()`](Self::effective_width), both of which exclude
    /// indent width — this is why setting `column` to a line's display
    /// width (e.g. in `render_code_block`) is correct despite the indent
    /// having just been written.
    column: usize,

    /// Number of consecutive newlines written since the last non-newline
    /// output. Used to avoid emitting duplicate blank lines.
    consecutive_newlines: usize,

    /// True until the first visible content is written. Prevents a leading
    /// blank line at the top of the output.
    at_start: bool,

    /// When true, the next word written will be preceded by a space (unless
    /// it falls at the start of a line). Set by trailing whitespace in text
    /// nodes and by soft breaks, so that spacing between adjacent inline
    /// nodes is preserved.
    pending_space: bool,

    /// Stack of active inline styles. Styles are pushed when entering a
    /// styled node and popped when leaving. The full stack is re-applied
    /// after a line wrap so that styling continues on the new line.
    style_stack: Vec<Style>,

    /// Extra indentation depth (in units of 2 spaces) for nested structures
    /// like lists and blockquotes.
    extra_indent: usize,

    /// Extra spaces to write on continuation lines after a line wrap, used
    /// to align wrapped list item text past the bullet/number prefix.
    continuation_indent: usize,

    /// Current blockquote nesting depth. Each level adds a `│ ` prefix to
    /// every line.
    blockquote_depth: usize,
}

impl<'w, W: Write> Renderer<'w, W> {
    pub fn new(writer: &'w mut W, terminal_width: usize) -> Self {
        let content_width = terminal_width;
        Self {
            writer,
            content_width,
            column: 0,
            consecutive_newlines: 0,
            at_start: true,
            pending_space: false,
            style_stack: Vec::new(),
            extra_indent: 0,
            continuation_indent: 0,
            blockquote_depth: 0,
        }
    }

    /// Finalize the output. Ensures non-empty output ends with exactly one
    /// trailing newline. Empty input produces empty output.
    pub fn finish(&mut self) -> Result<()> {
        if !self.at_start && self.consecutive_newlines == 0 {
            self.write_newline()?;
        }
        Ok(())
    }

    // =========================================================
    // Node dispatch
    // =========================================================

    pub fn render_node(&mut self, node: &Node) -> Result<()> {
        match node {
            Node::Root(root) => self.render_children(&root.children),
            Node::Heading(h) => self.render_heading(h),
            Node::Paragraph(p) => self.render_paragraph(p),
            Node::Text(t) => self.render_text(&t.value),
            Node::Strong(s) => self.render_strong(&s.children),
            Node::Emphasis(e) => self.render_emphasis(&e.children),
            // Hard break (trailing backslash or two spaces): emit a
            // newline within the current block.
            Node::Break(_) => {
                self.write_newline()?;
                self.write_indent()?;
                self.reapply_styles()?;
                Ok(())
            }
            Node::InlineCode(c) => self.render_inline_code(&c.value),
            Node::List(l) => self.render_list(l),
            Node::Blockquote(bq) => self.render_blockquote(&bq.children),
            Node::Code(c) => self.render_code_block(c),
            Node::Table(t) => self.render_table(t),
            Node::ThematicBreak(_) => self.render_thematic_break(),
            Node::Link(l) => self.render_link(l),
            Node::Delete(d) => self.render_strikethrough(&d.children),
            Node::Image(img) => self.render_image(img),
            // Fallback: render children if the node has any, silently skip
            // leaf nodes we don't handle yet.
            other => {
                if let Some(children) = other.children() {
                    self.render_children(children)?;
                }
                Ok(())
            }
        }
    }

    fn render_children(&mut self, children: &[Node]) -> Result<()> {
        for child in children {
            self.render_node(child)?;
        }
        Ok(())
    }

    // =========================================================
    // Block-level rendering
    // =========================================================

    fn render_heading(&mut self, heading: &markdown::mdast::Heading) -> Result<()> {
        self.ensure_block_spacing()?;
        self.write_indent()?;

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
            self.push_style(style)?;
        }
        self.render_children(&heading.children)?;
        for _ in styles {
            self.pop_style()?;
        }

        self.write_newline()?;
        Ok(())
    }

    /// Render a fenced or indented code block. The content is written
    /// verbatim (no word wrapping) with each line individually indented.
    /// The language hint from the fence is preserved internally but not
    /// displayed — it will be used for syntax highlighting in a later
    /// version.
    fn render_code_block(&mut self, code: &markdown::mdast::Code) -> Result<()> {
        self.ensure_block_spacing()?;
        self.push_style(Style::ForegroundColor(CODE_COLOR))?;

        // Code blocks get an extra 2-space indent beyond the global indent
        // to visually separate them from surrounding prose.
        self.extra_indent += 1;
        for line in code.value.lines() {
            self.write_indent()?;
            write!(self.writer, "{line}").context("write code line")?;
            self.column = UnicodeWidthStr::width(line);
            self.consecutive_newlines = 0;
            self.at_start = false;
            self.write_newline()?;
        }
        self.extra_indent -= 1;

        self.pop_style()?;
        Ok(())
    }

    fn render_table(&mut self, table: &markdown::mdast::Table) -> Result<()> {
        self.ensure_block_spacing()?;

        // The table module is pure: it lays out the table as a list of
        // pre-formatted content lines (box-drawing characters, cell text,
        // and any SGR escapes for bold headers) without any per-line
        // prefix. We walk the lines here so that each one receives the
        // renderer's normal line prefix — nesting indent *and* blockquote
        // bars — via `write_indent`. This matches the pattern used by
        // `render_code_block`.
        for line in crate::table::layout_table(table, self.effective_width()) {
            self.write_indent()?;
            write!(self.writer, "{line}").context("write table line")?;
            // Table rows occupy full content lines; once the line is
            // emitted, the next call must treat us as if we're mid-line
            // so `write_newline` can terminate the row cleanly. The
            // subsequent `write_newline` resets `column` back to 0.
            self.column = 0;
            self.consecutive_newlines = 0;
            self.at_start = false;
            self.write_newline()?;
        }
        Ok(())
    }

    fn render_blockquote(&mut self, children: &[Node]) -> Result<()> {
        // Block spacing must be ensured BEFORE entering the blockquote
        // context. Otherwise the blank separator line would get the │ bar
        // prefix, which visually belongs inside the blockquote, not between
        // the preceding block and the blockquote.
        self.ensure_block_spacing()?;
        self.blockquote_depth += 1;
        self.push_style(Style::Dim)?;
        self.push_style(Style::Italic)?;
        self.render_children(children)?;
        self.pop_style()?;
        self.pop_style()?;
        self.blockquote_depth -= 1;
        Ok(())
    }

    fn render_list(&mut self, list: &markdown::mdast::List) -> Result<()> {
        // Top-level lists need block spacing from surrounding content.
        // Nested lists (inside list items) only need a line break — the
        // parent item's content already ended with one.
        if self.extra_indent == 0 {
            self.ensure_block_spacing()?;
        }
        let mut number = list.start.unwrap_or(1);

        for child in &list.children {
            if let Node::ListItem(item) = child {
                self.render_list_item(item, list.ordered, number)?;
                number += 1;
            }
        }
        Ok(())
    }

    fn render_list_item(
        &mut self,
        item: &markdown::mdast::ListItem,
        ordered: bool,
        number: u32,
    ) -> Result<()> {
        self.ensure_line_break()?;
        self.write_indent()?;

        // Write the bullet or number prefix.
        let prefix = if ordered {
            format!("{number}. ")
        } else {
            "• ".to_string()
        };
        write!(self.writer, "{prefix}").context("write list marker")?;
        let prefix_width = UnicodeWidthStr::width(prefix.as_str());
        self.column += prefix_width;
        self.consecutive_newlines = 0;
        self.at_start = false;

        // Set continuation indent so that wrapped lines align past the
        // bullet/number prefix.
        let prev_continuation = self.continuation_indent;
        self.continuation_indent = prefix_width;

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
        // margin instead of aligned under the bullet.
        let mut is_first = true;
        for child in &item.children {
            match child {
                Node::Paragraph(p) => {
                    if !is_first {
                        self.ensure_block_spacing()?;
                        self.write_indent()?;
                        if self.continuation_indent > 0 {
                            let spaces = " ".repeat(self.continuation_indent);
                            write!(self.writer, "{spaces}")
                                .context("write list continuation indent")?;
                            self.column += self.continuation_indent;
                            self.consecutive_newlines = 0;
                            self.at_start = false;
                        }
                        self.reapply_styles()?;
                    }
                    self.render_children(&p.children)?;
                }
                // Nested lists and other block content get their own indent.
                // Ensure we're on a new line before starting nested content.
                other => {
                    self.ensure_line_break()?;
                    self.extra_indent += 1;
                    self.render_node(other)?;
                    self.extra_indent -= 1;
                }
            }
            is_first = false;
        }

        self.continuation_indent = prev_continuation;
        self.ensure_line_break()?;
        Ok(())
    }

    fn render_thematic_break(&mut self) -> Result<()> {
        self.ensure_block_spacing()?;
        self.write_indent()?;
        let width = self.effective_width();
        write!(self.writer, "{}", "─".repeat(width)).context("write thematic break")?;
        self.column = width;
        self.consecutive_newlines = 0;
        self.at_start = false;
        self.write_newline()?;
        Ok(())
    }

    fn render_paragraph(&mut self, paragraph: &markdown::mdast::Paragraph) -> Result<()> {
        self.ensure_block_spacing()?;
        self.write_indent()?;
        self.render_children(&paragraph.children)?;
        self.write_newline()?;
        Ok(())
    }

    // =========================================================
    // Inline rendering
    // =========================================================

    /// Render a text node, splitting on whitespace for word wrapping. Leading
    /// and trailing whitespace in the text is tracked via `pending_space` so
    /// that spacing between adjacent inline nodes is preserved.
    fn render_text(&mut self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        if text.starts_with(char::is_whitespace) {
            self.pending_space = true;
        }

        let mut first = true;
        for word in text.split_whitespace() {
            if first {
                first = false;
            } else {
                self.pending_space = true;
            }
            self.write_word(word)?;
        }

        if text.ends_with(char::is_whitespace) {
            self.pending_space = true;
        }

        Ok(())
    }

    fn render_strong(&mut self, children: &[Node]) -> Result<()> {
        self.push_style(Style::Bold)?;
        self.render_children(children)?;
        self.pop_style()
    }

    fn render_emphasis(&mut self, children: &[Node]) -> Result<()> {
        self.push_style(Style::Italic)?;
        self.render_children(children)?;
        self.pop_style()
    }

    fn render_strikethrough(&mut self, children: &[Node]) -> Result<()> {
        self.push_style(Style::Strikethrough)?;
        self.render_children(children)?;
        self.pop_style()
    }

    fn render_image(&mut self, image: &markdown::mdast::Image) -> Result<()> {
        let marker = if image.alt.is_empty() {
            "[image]".to_string()
        } else {
            format!("[image: {}]", image.alt)
        };
        self.render_text(&marker)?;
        self.write_dimmed_url_suffix(&image.url)
    }

    fn render_inline_code(&mut self, code: &str) -> Result<()> {
        self.push_style(Style::ForegroundColor(CODE_COLOR))?;
        self.render_text(code)?;
        self.pop_style()
    }

    fn render_link(&mut self, link: &markdown::mdast::Link) -> Result<()> {
        // Flush any pending space before enabling underline, so the space
        // between the preceding word and the link text is not underlined.
        if self.pending_space && self.column > 0 {
            write!(self.writer, " ").context("write space")?;
            self.column += 1;
            self.pending_space = false;
        }
        self.push_style(Style::Underline)?;
        self.render_children(&link.children)?;
        self.pop_style()?;
        self.write_dimmed_url_suffix(&link.url)
    }

    /// Write a URL in dimmed parentheses as a suffix to the previous inline
    /// content (link text or image marker). The `(url)` block is treated as
    /// one atomic unit for wrapping purposes: if it doesn't fit on the
    /// current line alongside the preceding content, it moves to a fresh
    /// line. The URL itself is never broken mid-string — on a narrow
    /// terminal it may overflow its own line, which is preferable to
    /// rendering it uncopyable.
    fn write_dimmed_url_suffix(&mut self, url: &str) -> Result<()> {
        let url_width = UnicodeWidthStr::width(url);
        let suffix_width = url_width + 2; // "(" + url + ")"

        // If we are mid-line and the suffix (including a leading space)
        // would push past the available width, wrap before writing so the
        // suffix starts on a fresh line.
        let mut need_space = self.column > 0;
        if need_space && self.column + 1 + suffix_width > self.effective_width() {
            self.wrap_line()?;
            // After wrap_line we are at the start of the content area for
            // the new line (column may still be > 0 due to continuation
            // indent, but conceptually we're "at the start" and should not
            // prefix another space).
            need_space = false;
        }
        self.pending_space = false;

        self.push_style(Style::Dim)?;
        if need_space {
            write!(self.writer, " ").context("write space before url")?;
            self.column += 1;
        }
        write!(self.writer, "({url})").context("write url suffix")?;
        self.column += suffix_width;
        self.consecutive_newlines = 0;
        self.at_start = false;
        self.pop_style()
    }

    // =========================================================
    // Word output with wrapping
    // =========================================================

    /// Write a single word, preceded by a space if `pending_space` is set.
    /// Wraps to a new line when the word would exceed the available content
    /// width.
    fn write_word(&mut self, word: &str) -> Result<()> {
        let width = UnicodeWidthStr::width(word);

        if self.pending_space && self.column > 0 {
            if self.column + 1 + width > self.effective_width() {
                self.wrap_line()?;
            } else {
                write!(self.writer, " ").context("write space")?;
                self.column += 1;
            }
        } else if self.column > 0 && self.column + width > self.effective_width() {
            // No space requested but the word still doesn't fit.
            self.wrap_line()?;
        }

        self.pending_space = false;
        write!(self.writer, "{word}").context("write word")?;
        self.column += width;
        self.consecutive_newlines = 0;
        self.at_start = false;
        Ok(())
    }

    /// Break to a new line and re-apply the current style stack, so that
    /// inline formatting continues seamlessly on the wrapped line.
    fn wrap_line(&mut self) -> Result<()> {
        self.write_newline()?;
        self.write_indent()?;
        // Add continuation indent to align past list bullet/number.
        if self.continuation_indent > 0 {
            let spaces = " ".repeat(self.continuation_indent);
            write!(self.writer, "{spaces}").context("write continuation indent")?;
            self.column += self.continuation_indent;
        }
        self.reapply_styles()?;
        Ok(())
    }

    // =========================================================
    // Low-level output helpers
    // =========================================================

    /// Content width available for text, accounting for global indent and any
    /// extra nesting indent. Always returns at least 1 to prevent degenerate
    /// behavior (e.g. invisible thematic breaks, every word on its own line).
    fn effective_width(&self) -> usize {
        self.content_width
            .saturating_sub(self.extra_indent * NEST_INDENT_WIDTH)
            .saturating_sub(self.blockquote_depth * 2)
            .saturating_sub(self.continuation_indent)
            .max(1)
    }

    fn write_newline(&mut self) -> Result<()> {
        writeln!(self.writer).context("write newline")?;
        self.column = 0;
        self.consecutive_newlines += 1;
        Ok(())
    }

    fn write_indent(&mut self) -> Result<()> {
        let has_indent = self.extra_indent > 0 || self.blockquote_depth > 0;

        for _ in 0..self.extra_indent {
            write!(self.writer, "{NEST_INDENT}").context("write nesting indent")?;
        }
        // Blockquote bars: each nesting level adds "│ ". The bars themselves
        // are always dim, independently of the content style. We apply Dim
        // directly (not via the style stack) and then use NormalIntensity to
        // clear it — which has the SGR 22 side-effect of also clearing Bold
        // (see `disable_style` for details). The subsequent `reapply_styles`
        // restores the correct content styling.
        if self.blockquote_depth > 0 {
            queue!(self.writer, SetAttribute(Attribute::Dim))?;
            for _ in 0..self.blockquote_depth {
                write!(self.writer, "│ ").context("write blockquote bar")?;
            }
            queue!(self.writer, SetAttribute(Attribute::NormalIntensity))?;
            self.reapply_styles()?;
        }

        // Only update output-tracking state when we actually wrote something,
        // so that a zero-indent call doesn't silently claim output occurred.
        if has_indent {
            self.consecutive_newlines = 0;
            self.at_start = false;
            self.pending_space = false;
        }
        Ok(())
    }

    /// Insert blank-line separation before a block element. Writes newlines
    /// until there are at least two consecutive ones (i.e., one visible blank
    /// line). Skipped at the very start of the output.
    ///
    /// When inside a blockquote, the blank line includes the indent and `│`
    /// bar prefix so that the visual bar column is unbroken.
    fn ensure_block_spacing(&mut self) -> Result<()> {
        if self.at_start {
            return Ok(());
        }
        if self.consecutive_newlines >= 2 {
            // Block spacing is already satisfied.
            return Ok(());
        }
        if self.blockquote_depth > 0 {
            if self.consecutive_newlines == 0 {
                self.write_newline()?;
            }
            // Write a blank line that carries the blockquote bar prefix.
            self.write_indent()?;
            self.write_newline()?;
            // write_indent resets consecutive_newlines to 0, and the
            // subsequent write_newline sets it to 1. Force it to 2 so that
            // callers see the block spacing as satisfied.
            self.consecutive_newlines = 2;
        } else {
            while self.consecutive_newlines < 2 {
                self.write_newline()?;
            }
        }
        Ok(())
    }

    /// Ensure we're on a fresh line, but don't require a full blank-line gap.
    /// Used between list items which should be visually tight.
    fn ensure_line_break(&mut self) -> Result<()> {
        if self.at_start {
            return Ok(());
        }
        if self.consecutive_newlines == 0 {
            self.write_newline()?;
        }
        Ok(())
    }

    // =========================================================
    // Style management
    // =========================================================

    fn push_style(&mut self, style: Style) -> Result<()> {
        self.style_stack.push(style);
        self.enable_style(style)
    }

    fn pop_style(&mut self) -> Result<()> {
        let style = self
            .style_stack
            .pop()
            .expect("style stack is never empty when pop_style is called");
        self.disable_style(style)
    }

    fn enable_style(&mut self, style: Style) -> Result<()> {
        match style {
            Style::Bold => queue!(self.writer, SetAttribute(Attribute::Bold))?,
            Style::Italic => queue!(self.writer, SetAttribute(Attribute::Italic))?,
            Style::Underline => queue!(self.writer, SetAttribute(Attribute::Underlined))?,
            Style::DoubleUnderline => {
                queue!(self.writer, SetAttribute(Attribute::DoubleUnderlined))?;
            }
            Style::Dim => queue!(self.writer, SetAttribute(Attribute::Dim))?,
            Style::Strikethrough => queue!(self.writer, SetAttribute(Attribute::CrossedOut))?,
            Style::ForegroundColor(c) => queue!(self.writer, SetForegroundColor(c))?,
        }
        Ok(())
    }

    /// Emit the escape sequence that disables a style.
    ///
    /// ANSI SGR has shared reset codes that affect multiple attributes at
    /// once. Two cases require compensating re-application after the reset:
    ///
    ///   - **Dim (SGR 2)**: There is no dedicated "NoDim" SGR code.
    ///     Crossterm uses `NormalIntensity` (SGR 22), which resets *both*
    ///     Bold and Dim. If Bold is still active on the style stack we must
    ///     re-emit SGR 1 immediately after.
    ///
    ///   - **ForegroundColor**: `SetForegroundColor(Color::Reset)` (SGR 39)
    ///     clears the foreground to the terminal default. If a color from an
    ///     outer scope is still on the stack (e.g., code color inside a
    ///     blockquote that also sets a color) we must re-apply it.
    ///
    /// These workarounds are inherent to the SGR model and cannot be avoided
    /// without a full "recompute all active attributes from scratch" reset,
    /// which would be more expensive.
    fn disable_style(&mut self, style: Style) -> Result<()> {
        match style {
            // NoBold (SGR 21) is interpreted as "doubly underlined" by many
            // terminals. Use NormalIntensity (SGR 22) instead, which clears
            // both Bold and Dim. Re-apply Bold itself (when an outer Bold
            // is still on the stack — e.g. inline `**bold**` inside a bold
            // heading) and Dim (when an outer Dim is still on the stack).
            Style::Bold => {
                queue!(self.writer, SetAttribute(Attribute::NormalIntensity))?;
                if self.style_stack.contains(&Style::Bold) {
                    queue!(self.writer, SetAttribute(Attribute::Bold))?;
                }
                if self.style_stack.contains(&Style::Dim) {
                    queue!(self.writer, SetAttribute(Attribute::Dim))?;
                }
            }
            // Re-emit Italic if an outer Italic is still on the stack
            // (e.g. inline `*italic*` inside an italic-bearing H1 or
            // blockquote).
            Style::Italic => {
                queue!(self.writer, SetAttribute(Attribute::NoItalic))?;
                if self.style_stack.contains(&Style::Italic) {
                    queue!(self.writer, SetAttribute(Attribute::Italic))?;
                }
            }
            // NoUnderline (SGR 24) clears both single and double underline.
            // After disabling, re-emit whichever underline variants are
            // still on the stack so outer scopes are preserved.
            Style::Underline => {
                queue!(self.writer, SetAttribute(Attribute::NoUnderline))?;
                if self.style_stack.contains(&Style::DoubleUnderline) {
                    queue!(self.writer, SetAttribute(Attribute::DoubleUnderlined))?;
                }
                if self.style_stack.contains(&Style::Underline) {
                    queue!(self.writer, SetAttribute(Attribute::Underlined))?;
                }
            }
            Style::DoubleUnderline => {
                queue!(self.writer, SetAttribute(Attribute::NoUnderline))?;
                if self.style_stack.contains(&Style::DoubleUnderline) {
                    queue!(self.writer, SetAttribute(Attribute::DoubleUnderlined))?;
                }
                if self.style_stack.contains(&Style::Underline) {
                    queue!(self.writer, SetAttribute(Attribute::Underlined))?;
                }
            }
            Style::Strikethrough => {
                queue!(self.writer, SetAttribute(Attribute::NotCrossedOut))?;
                if self.style_stack.contains(&Style::Strikethrough) {
                    queue!(self.writer, SetAttribute(Attribute::CrossedOut))?;
                }
            }
            // Symmetric to the Bold case above: NormalIntensity clears
            // both Bold and Dim, so re-emit Dim and/or Bold if the outer
            // scope still wants them.
            Style::Dim => {
                queue!(self.writer, SetAttribute(Attribute::NormalIntensity))?;
                if self.style_stack.contains(&Style::Dim) {
                    queue!(self.writer, SetAttribute(Attribute::Dim))?;
                }
                if self.style_stack.contains(&Style::Bold) {
                    queue!(self.writer, SetAttribute(Attribute::Bold))?;
                }
            }
            Style::ForegroundColor(_) => {
                queue!(self.writer, SetForegroundColor(Color::Reset))?;
                for &s in self.style_stack.iter().rev() {
                    if let Style::ForegroundColor(c) = s {
                        queue!(self.writer, SetForegroundColor(c))?;
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    /// Re-emit enable escapes for every style currently on the stack. Called
    /// after a line wrap so that formatting carries over to the new line.
    fn reapply_styles(&mut self) -> Result<()> {
        let styles: Vec<Style> = self.style_stack.clone();
        for style in styles {
            self.enable_style(style)?;
        }
        Ok(())
    }
}
