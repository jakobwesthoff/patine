// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Options that change how a document's source turns into lines and how
//! some of it looks. The defaults render Markdown documents. The soft
//! break and HTML alternatives suit short text fields whose author means
//! every newline and every `<`; the quote style suits a front end with a
//! color scheme of its own, for documents and text fields alike; the
//! first line width suits a front end that draws something at the end of
//! the first line; the callout style and spacing suit a front end with
//! colors or a layout of its own. Design rationale lives in
//! `docs/adr/0007-render-options-for-soft-breaks-and-html.md`,
//! `docs/adr/0010-quote-style-option.md`,
//! `docs/adr/0015-first-line-width-option.md` and
//! `docs/adr/0018-render-github-alerts-as-callouts.md`.

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
    /// The most columns the first line of the output takes, where that is
    /// fewer than the width; every later line takes the full width. `None`
    /// lays out every line at the full width. See
    /// [`with_first_line_width`](Self::with_first_line_width).
    pub first_line_width: Option<usize>,
    /// How callouts look: the colors of their bars, symbols and titles.
    pub callout_style: CalloutStyle,
    /// Whether a blank line separates a callout's title from its body.
    pub callout_spacing: CalloutSpacing,
}

impl RenderOptions {
    /// The default options, usable in a `const`.
    pub const fn new() -> Self {
        Self {
            soft_breaks: SoftBreaks::Join,
            html: Html::Drop,
            quote_style: QuoteStyle::DEFAULT,
            first_line_width: None,
            callout_style: CalloutStyle::DEFAULT,
            callout_spacing: CalloutSpacing::Attached,
        }
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

    /// These options with the first line of the output at most `width`
    /// columns wide, for a front end that draws something at the end of
    /// that line. Every later line takes the full width.
    ///
    /// Running text wraps its first line there, and a list item's bullet
    /// and a quote's bars count against it. A thematic break on the first
    /// line is as long. A table is laid out once, so a table that starts
    /// the output keeps the first line width on all its rows. Code blocks,
    /// frontmatter and literal HTML blocks are never wrapped, so their
    /// first line may be wider, as any of their lines may be wider than
    /// the width. A word wider than the first line width overflows the
    /// first line, as a word wider than the width overflows any line. A
    /// first line width at or above the width changes nothing.
    ///
    /// ```
    /// use patine::{RenderOptions, render_lines_with};
    ///
    /// let text = "one two three four five six seven";
    /// let options = RenderOptions::new().with_first_line_width(10);
    /// let lines = render_lines_with(text, 20, &options);
    /// assert_eq!(lines[0].to_string(), "one two");
    /// assert_eq!(lines[1].to_string(), "three four five six");
    /// assert_eq!(lines[2].to_string(), "seven");
    /// ```
    #[must_use]
    pub const fn with_first_line_width(mut self, width: usize) -> Self {
        self.first_line_width = Some(width);
        self
    }

    /// These options with `callout_style` replaced.
    #[must_use]
    pub const fn with_callout_style(mut self, callout_style: CalloutStyle) -> Self {
        self.callout_style = callout_style;
        self
    }

    /// These options with `callout_spacing` replaced.
    #[must_use]
    pub const fn with_callout_spacing(mut self, callout_spacing: CalloutSpacing) -> Self {
        self.callout_spacing = callout_spacing;
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

/// How a callout looks: its `┃ ` bars, the symbol of its kind and its
/// title.
///
/// Under both variants the title is bold and the body keeps the style
/// around it; only the colors differ. The style is independent of
/// [`QuoteStyle`]: a front end that shows no colors sets
/// [`CalloutStyle::DimBar`] next to [`QuoteStyle::DimBar`].
///
/// ```
/// use patine::{AnsiColor, CalloutColors, CalloutStyle, RenderOptions, render_lines_with};
///
/// let colors = CalloutColors::new().with_warning(Some(AnsiColor::Red));
/// let options = RenderOptions::new().with_callout_style(CalloutStyle::Colored(colors));
/// let lines = render_lines_with("> [!WARNING]\n> Back up first.", 80, &options);
/// assert_eq!(lines[0].to_string(), "┃ ▲ Warning");
/// assert_eq!(lines[0].spans[0].style.foreground, Some(AnsiColor::Red));
///
/// let options = RenderOptions::new().with_callout_style(CalloutStyle::DimBar);
/// let lines = render_lines_with("> [!WARNING]\n> Back up first.", 80, &options);
/// assert!(lines[0].spans[0].style.dim);
/// assert_eq!(lines[0].spans[1].style.foreground, None);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CalloutStyle {
    /// Each kind's bars, symbol and title in its color. The default is
    /// `Colored(CalloutColors::new())`.
    Colored(CalloutColors),
    /// The bars dim and the symbol and title bold, all without a color,
    /// for a front end that shows no colors.
    DimBar,
}

impl CalloutStyle {
    /// The default, usable in a `const` such as [`RenderOptions::new`].
    const DEFAULT: Self = Self::Colored(CalloutColors::new());
}

impl Default for CalloutStyle {
    /// Every kind in its color from [`CalloutColors::new`].
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The color of each kind of callout, for its bars, symbol and title.
/// `None` keeps the terminal's default foreground.
///
/// Fields may be added in minor releases, so outside patine a value starts
/// from [`CalloutColors::new`] or [`CalloutColors::default`]; the builders
/// are `const`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct CalloutColors {
    /// A `[!NOTE]` callout, blue by default.
    pub note: Option<AnsiColor>,
    /// A `[!TIP]` callout, green by default.
    pub tip: Option<AnsiColor>,
    /// An `[!IMPORTANT]` callout, magenta by default.
    pub important: Option<AnsiColor>,
    /// A `[!WARNING]` callout, yellow by default.
    pub warning: Option<AnsiColor>,
    /// A `[!CAUTION]` callout, red by default.
    pub caution: Option<AnsiColor>,
    /// A callout of any other type, such as `[!FOO]`, without a color by
    /// default.
    pub other: Option<AnsiColor>,
}

impl CalloutColors {
    /// The default colors, usable in a `const`. The five kinds take five
    /// palette colors that are not cyan, the default quote bar's.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            note: Some(AnsiColor::Blue),
            tip: Some(AnsiColor::Green),
            important: Some(AnsiColor::Magenta),
            warning: Some(AnsiColor::Yellow),
            caution: Some(AnsiColor::Red),
            other: None,
        }
    }

    /// These colors with `note` replaced.
    #[must_use]
    pub const fn with_note(mut self, color: Option<AnsiColor>) -> Self {
        self.note = color;
        self
    }

    /// These colors with `tip` replaced.
    #[must_use]
    pub const fn with_tip(mut self, color: Option<AnsiColor>) -> Self {
        self.tip = color;
        self
    }

    /// These colors with `important` replaced.
    #[must_use]
    pub const fn with_important(mut self, color: Option<AnsiColor>) -> Self {
        self.important = color;
        self
    }

    /// These colors with `warning` replaced.
    #[must_use]
    pub const fn with_warning(mut self, color: Option<AnsiColor>) -> Self {
        self.warning = color;
        self
    }

    /// These colors with `caution` replaced.
    #[must_use]
    pub const fn with_caution(mut self, color: Option<AnsiColor>) -> Self {
        self.caution = color;
        self
    }

    /// These colors with `other` replaced.
    #[must_use]
    pub const fn with_other(mut self, color: Option<AnsiColor>) -> Self {
        self.other = color;
        self
    }
}

impl Default for CalloutColors {
    /// The colors of [`CalloutColors::new`].
    fn default() -> Self {
        Self::new()
    }
}

/// Whether a blank line separates a callout's title from its body.
///
/// ```
/// use patine::{CalloutSpacing, RenderOptions, render_lines_with};
///
/// let text = |lines: Vec<patine::StyledLine>| -> Vec<String> {
///     lines.iter().map(ToString::to_string).collect()
/// };
/// let attached = render_lines_with("> [!TIP]\n> Run it twice.", 80, &RenderOptions::new());
/// assert_eq!(text(attached), ["┃ ◆ Tip", "┃ Run it twice."]);
///
/// let options = RenderOptions::new().with_callout_spacing(CalloutSpacing::BlankLine);
/// let spaced = render_lines_with("> [!TIP]\n> Run it twice.", 80, &options);
/// assert_eq!(text(spaced), ["┃ ◆ Tip", "┃ ", "┃ Run it twice."]);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CalloutSpacing {
    /// The body starts on the line under the title.
    #[default]
    Attached,
    /// A blank line, carrying the bar, separates the title from the body.
    /// A callout below the nesting limit, drawn without its bar, has the
    /// blank line under either variant.
    BlankLine,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_default_options_are_the_same() {
        assert_eq!(RenderOptions::new(), RenderOptions::default());
        assert_eq!(
            RenderOptions::new().quote_style,
            QuoteStyle::Bar(AnsiColor::Cyan)
        );
        assert_eq!(
            RenderOptions::new().callout_style,
            CalloutStyle::Colored(CalloutColors::default())
        );
        assert_eq!(CalloutStyle::default(), RenderOptions::new().callout_style);
        assert_eq!(
            RenderOptions::new().callout_spacing,
            CalloutSpacing::Attached
        );
    }

    #[test]
    fn default_callout_colors_follow_the_kinds() {
        let colors = CalloutColors::new();
        assert_eq!(colors, CalloutColors::default());
        assert_eq!(colors.note, Some(AnsiColor::Blue));
        assert_eq!(colors.tip, Some(AnsiColor::Green));
        assert_eq!(colors.important, Some(AnsiColor::Magenta));
        assert_eq!(colors.warning, Some(AnsiColor::Yellow));
        assert_eq!(colors.caution, Some(AnsiColor::Red));
        assert_eq!(colors.other, None);
    }

    #[test]
    fn each_color_builder_sets_its_kind_alone() {
        let red = Some(AnsiColor::Red);
        let base = CalloutColors::new();
        assert_eq!(base.with_note(red).note, red);
        assert_eq!(base.with_tip(red).tip, red);
        assert_eq!(base.with_important(red).important, red);
        assert_eq!(base.with_warning(None).warning, None);
        assert_eq!(base.with_caution(None).caution, None);
        assert_eq!(base.with_other(red).other, red);
        assert_eq!(base.with_note(red).tip, base.tip);
    }
}
