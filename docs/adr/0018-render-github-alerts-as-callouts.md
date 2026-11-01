# 18. Render GitHub alerts as callouts

Date: 2026-11-01

## Status

Accepted

Amends [16. Use comrak for Markdown parsing](0016-use-comrak-for-markdown-parsing.md)

Amends [7. Render options for soft breaks and HTML](0007-render-options-for-soft-breaks-and-html.md)

## Context

GitHub's alert syntax marks a quote as a callout with its first line:
`> [!NOTE]`, `> [!TIP]`, `> [!IMPORTANT]`, `> [!WARNING]` and
`> [!CAUTION]`. GitHub draws these as boxes with a colored title. patine
2.1.0 drew them as plain quotes with the marker as text, so `ntropy show`
showed `│ [!NOTE] …` for every callout in a note.

comrak 0.59.0 has two extensions for the syntax. `alerts` parses the five
kinds; its title is the rest of the marker line as a plain string, and
any other type stays a quote. `ofm_callouts`, Obsidian's callouts, parses
any type up to the first `]` after `[!`, an optional fold marker `+` or
`-`, metadata after a `|`, and the rest of the marker line as Markdown
blocks in a title node. It lowercases the type and derives a default
title from it (`Note`, `Foo`, `My kind`).

## Decision

patine turns on `ofm_callouts` and leaves `alerts` off. comrak tries
`ofm_callouts` first, and `alerts` would still take the markers it
leaves, such as `> [!NOTE]text`: two rules for one syntax.

A callout draws a heavy bar `┃ ` on every line, 2 columns wide like a
quote's bar, in the color of its kind. Its first line is the kind's
symbol and the title, bold and in that color. The body starts on the
line under the title.

| Kind      | Symbol | Color   |
|-----------|--------|---------|
| Note      | `●`    | Blue    |
| Tip       | `◆`    | Green   |
| Important | `■`    | Magenta |
| Warning   | `▲`    | Yellow  |
| Caution   | `✕`    | Red     |
| any other | `○`    | none    |

Any type other than the five is a neutral callout in the terminal's
default foreground. The five kinds and the quote bar's cyan take every
palette color but black and white. Each symbol is one column wide.

The title is the text after the marker, rendered as inline Markdown, in
place of the type's name; otherwise it is comrak's default title. A type
whose default title is blank, such as `[!-]`, shows the symbol alone.
The symbol is placed like a list bullet, so the title's first word stays
on its line and a wrapped title continues under the title's text.

comrak parses the text after the marker as blocks. The title is the text
of the first leaf block below the title node: a paragraph's or heading's
inline content, a code block's text as inline code, an HTML block as
inline HTML under `Html`. A callout in the title without a title of its
own gives its default title; one with a title is looked into. A callout
written in a title thus gives only its text and draws no bar or symbol.
When that block draws nothing (a thematic break, HTML that is not shown,
an empty title node left by a footnote definition on the marker line),
the default title shows.

Fold markers and metadata draw nothing; a callout is never folded. A
marker with nothing under it shows its title line alone.

Callouts nest wherever comrak finds them, and count against the 64
levels that ADR 16 sets for block quotes, list items and footnote
definitions. A callout below that depth renders flat like the other
containers, but keeps its title as a line of its own, followed by its
body after a blank line.

### Options

`RenderOptions` gets two fields with `const` builders,
`with_callout_style` and `with_callout_spacing`:

```rust
#[non_exhaustive]
pub enum CalloutStyle {
    Colored(CalloutColors), // default Colored(CalloutColors::new())
    DimBar,
}

#[non_exhaustive]
pub struct CalloutColors {
    pub note: Option<AnsiColor>,      // Blue
    pub tip: Option<AnsiColor>,       // Green
    pub important: Option<AnsiColor>, // Magenta
    pub warning: Option<AnsiColor>,   // Yellow
    pub caution: Option<AnsiColor>,   // Red
    pub other: Option<AnsiColor>,     // None
}

#[non_exhaustive]
pub enum CalloutSpacing {
    Attached, // default
    BlankLine,
}
```

`Colored` draws each kind's bars, symbol and title in its color, `None`
in the default foreground. `DimBar` draws the bars dim and the symbol and
title bold without a color, for a front end that shows no colors.
`CalloutColors` has `const` builders for each field. `BlankLine` puts a
blank line carrying the bar between the title and the body.

`CalloutStyle` is independent of `QuoteStyle`: two settings without a
hidden coupling. A front end that sets `QuoteStyle::DimBar` for output
without colors, as ntropy and asqr do under `NO_COLOR`, sets
`CalloutStyle::DimBar` as well.

The command-line interface has no flag for either option.

## Consequences

- Quotes whose first line is a marker render as callouts, beyond what
  GitHub draws as alerts: other types, a title after the marker, fold
  markers, metadata, empty callouts and nested callouts are quotes on
  GitHub. `>[!NOTE]` and a body line without `>` are callouts on both.
- GitHub draws `>  [!NOTE]`, `>\t[!NOTE]` and `> \[!NOTE]` as alerts;
  patine keeps them quotes, as comrak parses them.
- A type before a custom title is not drawn: `> [!NOTE see [docs] here`
  shows `○ here`.
- Inline code and unhighlighted code blocks are yellow, so in a Warning
  they have the color of the bar.
- A front end that removes colors and leaves the callout style at its
  default gets the bars and titles in its default foreground, not dim;
  the heavy bar and the symbol still keep a callout apart from a quote.
- ADR 7's options gain the callout style and spacing; they apply to
  Markdown documents and short text fields alike.
