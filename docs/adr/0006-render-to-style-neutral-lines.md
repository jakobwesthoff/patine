# 6. Render to style-neutral lines

Date: 2026-10-14

## Status

Accepted

Amends [3. Direct-write rendering architecture](0003-direct-write-rendering-architecture.md)

Amended by [7. Render options for soft breaks and HTML](0007-render-options-for-soft-breaks-and-html.md)

Amended by [10. Quote style option](0010-quote-style-option.md)

Amended by [11. Table cells render as inline content](0011-table-cells-render-as-inline-content.md)

Amended by [17. Offer the ANSI encoding as a Display adapter](0017-offer-the-ansi-encoding-as-a-display-adapter.md)

## Context

ADR 0003 has the renderer write ANSI escape sequences straight into an
`impl Write` and names an intermediate representation as the way out
once a second output backend appears. That second backend is asqr, a
terminal user interface built on ratatui, which shows Markdown and
highlighted source code and should look exactly like patine. A ratatui
application draws styled spans into a buffer; it cannot use escape
sequences without parsing them back into styles.

The toggle model of ADR 0003 also leaked styles into places they did not
belong. A style stayed active until something switched it off, across
newlines too, so line prefixes were drawn in the content's style:
blockquote bars were italic, and on the wrapped lines of a heading in a
quote also bold and underlined; the continuation indent of a wrapped
link or strikethrough in a list item was underlined or struck through.
SGR 22, which switches off bold, also switches off dim, so a bold table
header cell inside a quote dropped the quote's dim for the rest of its
row. The renderer carried re-application code for each of these resets.

## Decision

patine's library renders to style-neutral lines. asqr uses the library
directly; there is no separate crate.

### Output model

- `StyledLine` holds the spans of one line in reading order, without a
  newline. A blank line has no spans.
- `StyledSpan` is a text and the `TextStyle` it is drawn in.
- `TextStyle` is the complete style: `bold`, `italic`, `dim`,
  `strikethrough`, `underline` and `foreground`.
- `Underline` is `None`, `Single` or `Double`.
- `AnsiColor` names the 8 palette colors, `Black` to `White`, and
  `index()` returns the palette index 0 to 7. ADR 0005 restricts patine
  to these 8.

The model holds no crossterm or ratatui types. The three structs are
`#[non_exhaustive]` with public fields: outside patine they are read
directly and built with `StyledSpan::new`, `StyledLine::new` and
`TextStyle::default()`. The two enums are exhaustive.

Every span carries its whole style, and patine merges adjacent spans of
the same style. Line prefixes are spans of their own: nesting and
continuation indents are unstyled spaces, blockquote bars are dim
`│ ` spans whatever the quoted content looks like. A space between two
words takes the style active where the whitespace occurs, so the space
in front of an inline element is styled like the text around it.
Frontmatter blocks and image markers are ordinary styled spans.

### Public functions

- `render_lines(input, width) -> Vec<StyledLine>` renders a Markdown
  document.
- `highlight(code, language: Option<&str>) -> Vec<StyledLine>`
  highlights source code outside Markdown with the theme of ADR 0005,
  without indent or wrapping. `None`, a blank or an unknown language
  yields unstyled lines.
- `render(input, writer, terminal_width)` keeps its signature and writes
  the lines of `render_lines` as ANSI escape sequences.

Both new functions are infallible. markdown-rs reports syntax errors
only for MDX, which patine does not enable. When syntect fails on a
block of code, that block falls back: to the flat code color inside
Markdown, to unstyled lines in `highlight`.

### Renderer and writer

The renderer keeps the node dispatch, wrapping, indentation and block
spacing of ADR 0003 and its style stack. Instead of writing, it appends
text to the line under construction, styled from the whole stack at
that moment, and a newline closes the line. The table layout returns
styled lines with bold header cells; the renderer lays the surrounding
style, such as a quote's dim italic, over them.

The ANSI writer writes each span as its attributes switched on, the
text, and the same attributes switched off again, so every span and
every line starts from the terminal's default style. Bold and dim are
switched off with SGR 22, never SGR 21. The writer formats these
sequences itself.

### Features

A default feature `cli` builds the `patine` binary and is the only
thing that pulls in `clap` and `crossterm`. Library users turn it off
with `default-features = false`.

## Consequences

- The renderer has no write errors. `render` fails only when its writer
  fails.
- The re-application code for shared SGR resets is gone, and with it
  the style leaks described above. Against the 1.4.0 snapshots, the
  text is unchanged apart from one trailing space that is no longer
  written before a link that wraps to the next line, and the styling
  changed only in those leaks and in the space in front of inline
  elements.
- Renderer tests compare lines: the snapshots in `tests/rendering.rs`
  write each span once with its complete style. `tests/ansi.rs` replays
  the output of `render` like a terminal, checks it against
  `render_lines` character by character, and pins the bytes of a few
  documents. Escape order no longer depends on crossterm.
- Adding a field to `TextStyle`, `StyledSpan` or `StyledLine` does not
  break code outside patine; adding a variant to `Underline` or
  `AnsiColor` does.
