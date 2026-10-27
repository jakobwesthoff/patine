# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.8.1] - 2026-10-22

### Changed
- Added `--ascii` flag for displaying with ascii only chars

## [1.8.0] - 2026-10-22

### Changed

- A table whose columns would have to break a word is laid out as
  records: one record per data row, one line per column with the
  column's name from the header row in bold and dim and then the
  cell's text, and a rule across the width between records. A value
  wraps in the room beside the names, or stands under its name when
  that room would break a word that fits there. Eight columns of short
  words at width 22 used to come out one letter wide. A word no longer
  breaks in a boxed table; in a record, a word wider than its room
  breaks between graphemes, and a line is wider than the width only
  where a single grapheme is wider than the room it gets.

### Fixed

- A halfwidth sound mark, `ﾞ` or `ﾟ`, counts as one column, as
  terminals draw it. Text and tables holding one were laid out
  narrower than they show: a table cell of `ｶﾞ` six times, rendered at
  width 12, took 16 columns on screen. `StyledSpan::width` and
  `StyledLine::width` count the mark too.
- Widths count grapheme by grapheme, as terminals and ratatui draw
  text. unicode-width gives some ligatures one column as a string,
  such as Lam-Alef `لا` and the Hebrew Alef-Lamed, where their letters
  are drawn in a column each, so text holding one was laid out
  narrower than it shows. A word broken between characters keeps every
  grapheme whole, so a flag emoji is no longer cut in two. patine now
  depends on `unicode-segmentation`.

## [1.7.2] - 2026-10-16

### Fixed

- A table whose longest words do not fit the width breaks those words
  across lines, between characters, and fits the width. A long URL in
  a cell set its column's width and made the table overflow, 66
  columns for a table rendered at 40. Words that fit their column stay
  whole, and a broken word keeps its style on every line, so a URL
  stays dim. A table now overflows only when not even one character
  per column fits. Running text is unchanged: a word wider than the
  line still overflows it.

## [1.7.1] - 2026-10-16

### Fixed

- Table cells keep their inline styles: bold, italic, strikethrough,
  inline code and links render as in running text, and a header cell
  adds bold on top. Cells used to render as plain text. A link in a
  cell shows its dimmed `(url)` and an image its `[image: alt]` marker
  with its path, as in running text, so a column of links is wider than
  before; an image used to leave its cell empty. Cell text wraps like
  running text: glued pieces such as `**bold**,` stay on one line, and
  a `(url)` may move to the next line. A run of spaces in a cell, or
  inline HTML that is not drawn, no longer widens its column.
- A list after text in a block quote inside a list item gets the blank
  line, carrying the bar, that it gets in a quote anywhere else: `- a`
  followed by a quote `b` and a list `c` rendered `  │ b` and `  │ • c`
  with no line between them. The same holds for a list in a footnote
  definition inside a list item.
- A paragraph in a list item whose earlier blocks drew nothing, such as
  an HTML block under `Html::Drop` or an empty code block, follows the
  bullet on the next line without a blank line, as a quote or code
  block there does.
- Highlighting a log (`log`) takes about a fifth of the time it took:
  20,000 lines of an access log took 3.8 s and take 0.8 s, with the
  same colors. Four patterns of the Log syntax that two-face ships took
  time quadratic in the length of a line; patine anchors them in its
  own copy of the syntax set. Building that copy costs 0.2 s and about
  7 MB once, on the first log highlighted, with a passing peak of
  66 MB.

## [1.7.0] - 2026-10-16

### Added

- `RenderOptions::quote_style` and `with_quote_style` choose how block
  quotes look: `QuoteStyle::Bar(color)` draws the `│` bar in a palette
  color (the default is cyan), `QuoteStyle::DimBar` draws it dim without
  a color.
- Task list items show `☐` (open) or `☑` (done) in place of the
  bullet. A numbered task keeps its number in front of the box
  (`1. ☐`). A done task is dim, number and box included, and so is
  everything in it except nested lists, whose items show their own
  state. Task items used to show a plain bullet, so done and open tasks
  looked the same.
- Table columns follow the alignment of the delimiter row (`:--`,
  `:-:`, `--:`), header and data cells alike. A column without
  alignment keeps its centered header and left-aligned data.

### Changed

- Block quote text renders upright in its normal style instead of dim
  and italic, so emphasis shows inside a quote, and the `│` bar is cyan
  instead of dim. Tables, code blocks and HTML blocks in a quote keep
  their own styles.

### Fixed

- A code block, list or other block inside a block quote sits inside
  the quote's bar. A code block in a quote rendered as `  │ code` and
  now renders as `│   code`; a nested list in a quote rendered as
  `  │ • b` and now renders as `│   • b`. The same holds for blocks
  nested in a list item inside a quote: `> - > quote` rendered its inner
  line as `  │ │ quote` and now renders `│   │ quote`. The blank line
  between two blocks there ends at the bar.
- A list item whose first block draws nothing, such as an empty code
  block or an HTML block under `Html::Drop`, puts no blank line between
  its bullet and the next block, as when the item starts with that
  block.

## [1.6.3] - 2026-10-14

### Fixed

- A list item that starts with a block quote, code block or heading
  shows the block on the line under the bullet or number. A blank line
  used to separate them, as if the bullet stood alone. A nested list
  starting an item already rendered this way.
- With `Html::Drop` (the default), a link whose text is its URL wrapped
  in raw HTML gets no dimmed `(url)` after it, since the HTML is not
  drawn: `[<b>https://x.org</b>](https://x.org)` rendered as
  `https://x.org (https://x.org)`. With `Html::Literal` the HTML is
  drawn, so the link text differs from its URL and keeps the `(url)`.

## [1.6.2] - 2026-10-14

### Changed

- A link whose text is its URL gets no dimmed `(url)` after it, since
  the URL is already there: `<https://example.com>`, bare URLs and email
  addresses, `<someone@example.com>` (whose URL has `mailto:` in front)
  and `[https://example.com](https://example.com)`, which showed the URL
  three times. A link with other text keeps the `(url)`, as does a bare
  `www.example.com`, whose URL is `http://www.example.com`.

### Fixed

- List items wrap at the full width. The bullet or number was counted
  twice, so an item's lines broke as many columns early as the bullet or
  number is wide: at width 20, `1. aaa bbb ccc ddd` broke before `ddd`.
  A table or thematic break inside a list item was that much narrower
  too. Items that wrap render differently from 1.6.1.
- A list item whose first word is wider than the room after the bullet
  keeps that word next to the bullet, where it overflows like any word
  wider than the line. It used to leave the bullet alone on its line.
- A blockquote or heading inside a list item wraps under its own text.
  Its wrapped lines used to start as far right again as the item's bullet
  or number is wide.
- A lone `\r` line ending ends the line in code blocks, frontmatter and
  `highlight`, like `\n` and `\r\n` do. It used to stay inside the line,
  where a terminal moves the cursor back and overwrites the line's start.
- With `SoftBreaks::Break`, a soft break written as a lone `\r` right
  before an inline element breaks the line. `a\r**b**` rendered as `ab`.

## [1.6.1] - 2026-10-14

### Fixed

- Inline elements written without whitespace between them wrap as one
  word, each in its own style. A line used to break between `foo` and
  `**bar**` in `foo**bar**`, before the comma in `**bold**,`, after the
  `(url)` of `[link](url)s`, and with `Html::Literal` between `Vec` and
  `<String>`. A line now breaks only where the source has whitespace or
  a line break, and in front of a link's or image's `(url)`. This
  changes the default rendering as well: where 1.6.0 broke a line inside
  such a run, 1.6.1 keeps the run together.

## [1.6.0] - 2026-10-14

### Added

- `render_lines_with(input, width, options)` and
  `render_with(input, writer, terminal_width, options)` render with
  `RenderOptions`. `render_lines` and `render` use the defaults, which
  render exactly as 1.5.0 did apart from the fixes below.
- `RenderOptions::soft_breaks` decides what a single newline inside
  running text does: `SoftBreaks::Join` (the default) joins the lines as
  CommonMark specifies, `SoftBreaks::Break` breaks the line like a hard
  break.
- `RenderOptions::html` decides what happens to raw HTML: `Html::Drop`
  (the default) leaves it out, `Html::Literal` shows it as written,
  inline HTML within its line and an HTML block line by line without
  wrapping.

### Fixed

- The line after a hard break (a trailing backslash or two spaces) in a
  list item starts under the item's text, like a wrapped line. It used to
  start under the bullet, at the item's nesting indent.

## [1.5.0] - 2026-10-14

### Added

- `render_lines(input, width)` returns the rendered document as
  `StyledLine`s instead of escape sequences: lines of `StyledSpan`s,
  each with a complete `TextStyle` (bold, italic, dim, strikethrough,
  `Underline` and an optional `AnsiColor` from the 8-color palette).
  Applications that draw their own interface, such as ratatui
  applications, map these onto their own types and render Markdown
  exactly like patine. `render` writes the same lines as ANSI escape
  sequences.
- `highlight(code, language)` syntax-highlights source code outside
  Markdown into `StyledLine`s with the same theme as code blocks. Without
  a known language it returns the lines unstyled.

### Changed

- The command-line interface sits behind a new `cli` feature, enabled by
  default. Library users can depend on patine with
  `default-features = false` to leave out `clap`; `cargo install patine`
  builds the binary as before.
- The library writes its escape sequences itself and no longer depends
  on `crossterm`, which only the `cli` feature pulls in now.
- A code block that syntect fails to highlight renders in the flat code
  color instead of making `render` return an error.
- `render` switches the styles of every span on before it and off after
  it. The escape sequences in the output therefore differ from 1.4.0
  even where the text looks the same on screen.

### Fixed

- Styles no longer leak from one line into the next. A link or
  strikethrough that wrapped inside a list item underlined or struck
  through the indent of its continuation lines; every line now closes
  the styles it opens, and indents are always unstyled.
- Blockquote `│` bars are dim only. They used to pick up the quote's
  italic, and on the wrapped lines of a heading inside a quote also its
  bold and underline.
- A table inside a blockquote is dim and italic throughout. Before, the
  rest of the header row after the first header cell lost the dim.
- The space in front of inline code, bold, italic or struck-through text
  is styled like the text around it. It used to take on the style of the
  element after it, which showed as a struck-through space before
  `~~deleted~~` text. Links already kept that space plain.

## [1.4.0] - 2026-07-12

### Added

- YAML (`---`) and TOML (`+++`) frontmatter at the start of a document
  is now recognized and rendered as a syntax-highlighted verbatim
  block, fence lines included, styled exactly like a fenced code block
  of that language. Empty frontmatter renders nothing. Previously such
  blocks were parsed as ordinary markdown, which mangled the metadata
  into a setext heading; documents relying on that accidental rendering
  will look different now.

## [1.3.0] - 2026-04-17

### Added

- Syntax highlighting for fenced code blocks. When a fence carries a
  recognized language identifier (e.g. `` ```rust ``), tokens are
  colorized using only the 8 standard ANSI palette colors, so the
  user's configured terminal theme (Solarized, Gruvbox, base16, etc.)
  drives the actual appearance. Highlighting is powered by `syntect`
  with the grammar and theme collection from
  [`two-face`](https://github.com/CosmicHorrorDev/two-face) (the bat
  project's curated set). Fences with no language, or with an
  unrecognized one, fall back to the existing flat code color.

### Changed

- Release builds now enable full cross-crate LTO, a single codegen
  unit, symbol stripping, and `panic = "abort"`. Distributed binaries
  are noticeably smaller (~3 MB on macOS aarch64) and benefit from
  whole-program optimization; release link time is correspondingly
  longer.

## [1.2.0] - 2026-04-11

### Added

- Heading hierarchy now uses bold + underline decorations to create a
  clearer visual ladder. **H1**: bold + italic + double underline.
  **H2**: bold + underlined. **H3–H6**: bold (unchanged). Double
  underline is emitted as the modern `CSI 4:2 m` (Kitty sub-parameter)
  sequence; terminals that don't support it gracefully degrade to a
  single underline, with H1's italic keeping it distinct from H2.

### Fixed

- Style state could leak across nested inline scopes when the same
  attribute appeared more than once on the style stack — for example,
  `# Heading with **bold**` would lose its heading bold for the rest
  of the line after the inline `**bold**` closed, and `*italic*` text
  inside an italic blockquote would similarly drop the blockquote's
  italic. The disable-style routine now re-emits Bold, Italic, Dim,
  Strikethrough, and Underline / DoubleUnderline if an outer scope
  still has them on the stack.

## [1.1.1] - 2026-04-11

### Changed

- Unordered list bullets reverted to `•` (U+2022 BULLET). The larger
  `●` (U+25CF) introduced in 1.1.0 was visually too heavy in practice.

## [1.1.0] - 2026-04-11

### Added

- Tables now word-wrap cell contents to fit the available terminal
  width. Column widths are distributed proportionally toward each
  column's minimum (the widest unbreakable token) and rows lay out at
  variable height to accommodate wrapped cells. Tables that cannot fit
  even at their minimum widths overflow the terminal rather than
  breaking words mid-string.
- Long link and image URLs whose `(url)` suffix does not fit alongside
  the preceding content now move to their own line instead of
  overflowing inline. The URL itself is never broken mid-string so it
  remains copy-pasteable.

### Changed

- Unordered list bullets now use `●` (U+25CF BLACK CIRCLE) instead of
  `•` (U+2022 BULLET) for a larger, more visually distinct marker.

### Fixed

- Tables rendered inside blockquotes now carry the blockquote `│` bar
  prefix on every line, including border and separator lines.
  Previously the bars were dropped, visually breaking the table out of
  its enclosing blockquote.
- Multi-paragraph list items no longer render their paragraphs glued
  together with no separator. Subsequent paragraphs now emit a
  blank-line separator and sit past the bullet, aligned with the
  wrapped lines of the first paragraph.
- `effective_width()` could reach zero with deep nesting, producing
  invisible horizontal rules and degenerate one-word-per-line
  wrapping. It now floors at 1.

## [1.0.0] - 2026-04-09

### Added

- Render Markdown files or stdin to styled terminal output.
- Headings: H1 rendered italic + underlined, H2-H6 rendered bold.
- Paragraphs with word wrapping at terminal width.
- Inline formatting: bold, italic, bold-italic, strikethrough.
- Inline code rendered in a distinct color.
- Fenced code blocks rendered verbatim (no wrapping) in a distinct color.
  Language hints are preserved internally for future syntax highlighting.
- Links rendered as underlined text followed by the URL in dimmed parentheses.
- Images rendered as `[image: alt text]` followed by the path/URL dimmed.
- Ordered and unordered lists with proper nesting (2-space indent per level).
- Unordered lists use `\u{2022}` as the bullet character.
- GFM tables with Unicode box-drawing borders, centered bold headers, and
  row separators between every row.
- Blockquotes with dimmed `\u{2502}` bar prefix and dim+italic text, including
  nested blockquote support.
- Horizontal rules rendered as `\u{2500}` spanning the terminal content width.
- Fenced code blocks indented by one nesting level (2 spaces) for visual
  separation from prose.
- Hard breaks (trailing backslash or two spaces) rendered as actual line breaks.
- Automatic terminal width detection via `crossterm::terminal::size()`, with
  80-column fallback.
