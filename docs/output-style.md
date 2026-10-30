# Output Style Specification

This document defines how patine renders Markdown to styled terminal output.
All examples show the visual output, not the Markdown source.

## Global Layout

- Content starts at the left terminal edge (no global indent).
- Blank lines separate block-level elements (headings, paragraphs, lists,
  tables, code blocks, horizontal rules).
- Long lines wrap at the terminal width. With
  `RenderOptions::first_line_width`, the first line of the output wraps
  at that width instead, and a table that starts the output is laid out
  for it.
- Widths count grapheme by grapheme, as terminals and ratatui draw
  text: the letters of a Lam-Alef `لا` take a column each, and a
  halfwidth sound mark, `ﾞ` or `ﾟ`, takes one column. Control
  characters keep their Unicode width.
- Nested structures (lists, code blocks) are indented 2 spaces per level.
- Wrapped continuation lines inside list items align past the bullet/number.

## Headings

Markdown `#` markers are stripped. A blank line appears after each heading.

Every heading is **bold** as a consistent baseline; underline decorations
create the visual hierarchy between levels:

- **H1**: bold + italic + double underline.
- **H2**: bold + underlined.
- **H3–H6**: bold only.

```
AI Templates: POC → Product — Summary    ← bold, italic, double-underlined
                                         ← blank line
Business Decisions (need alignment)      ← bold, underlined
                                         ← blank line
Sub-Section Heading                      ← bold
```

Double underline is emitted as the modern `CSI 4:2 m` (Kitty
sub-parameter) sequence, which terminals that don't support it
gracefully degrade to a single underline. Italic on H1 keeps it
distinguishable from H2 even after such a degradation.

No color differentiation between heading levels — the entire hierarchy
relies on bold + underline decorations.

## Paragraphs

Plain text blocks are rendered at normal weight with blank line separation
between paragraphs.

```
This is a paragraph of regular text. It wraps at the terminal width
naturally without any special continuation indent.

This is another paragraph separated by a blank line.
```

## Inline Formatting

### Bold (`**text**`)

Markers stripped. Text rendered **bold**, default terminal color.

### Italic (`*text*`)

Markers stripped. Text rendered *italic* (terminal italic escape, with
dimmed color as fallback for terminals that lack italic support).

### Bold Italic (`***text***`)

Markers stripped. Text rendered **bold** + *italic*.

### Inline Code (`` `code` ``)

Backticks stripped. Text rendered in a distinct color to visually separate
it from prose. This is one of the few elements that uses color.

```
Schema via CREATE TABLE IF NOT EXISTS, no versioning.
            ^^^^^^^^^^^^^^^^^^^^^^^^^
            distinct color
```

### Strikethrough (`~~text~~`)

Markers stripped. Text rendered with strikethrough escape sequence. Fallback:
dimmed text for terminals without strikethrough support.

### Links (`[text](url)`)

Link text rendered underlined. URL shown after in parentheses, dimmed.

```
See the documentation (https://example.com/docs) for details.
        ^^^^^^^^^^^^^
        underlined     ^^^^^^^^^^^^^^^^^^^^^^^^^^^
                       dimmed
```

A link whose text is its URL gets no dimmed URL: autolinks and bare URLs
render as the underlined URL alone. An email autolink counts as such a
link, since its URL is its text behind `mailto:`. A bare `www.` address
keeps the dimmed URL, which has `http://` in front of it.

```
See https://example.com for details.
    ^^^^^^^^^^^^^^^^^^^
    underlined
```

The dimmed `(url)` block is treated as one atomic unit when wrapping. If
it doesn't fit alongside the link text on the current line, the whole
`(url)` block moves to a fresh line — the URL itself is never broken
mid-string so it remains copy-pasteable. Only in a value of a table
laid out as records, when it is wider than its room, does it break (see
Records under Tables).

## Ordered Lists

Number and period preserved from source. Inline formatting (bold, code, etc.)
applied within items. Wrapped continuation lines align past the number prefix.

```
1. Deployment model — Single global instance vs. per-customer?
2. Authentication — Own auth system needed. Request → approval → active.
...
10. UI overhaul — Login, user dashboard, generation progress.
```

No special alignment padding for single-digit vs. multi-digit numbers — they
simply take the space they need.

## Unordered Lists

Bullet character is `•`, followed by a space, then content. Task items
show a box instead (see Task Lists).

```
• First item with some text
• Second item with some text
```

## Task Lists

A task list item (`- [ ]`, `- [x]`) shows a box in place of its bullet:
`☐` for an open task, `☑` for a done one. A numbered task keeps its
number, with the box after it (`1. ☐`). The boxes have no color.

A done task recedes: its line, number and box included, is dimmed, and
so is everything else in the item except nested lists. Each item of a
nested list shows its own state, so an open sub-task under a done task
does not look done.

```
☐ Write the release notes
☑ Tag the release               ← dimmed
  ☐ Publish the notes           ← nested, keeps its own style

1. ☐ Open numbered task
2. ☑ Done numbered task         ← dimmed, number included
```

## Nested Lists

Additional **2 spaces** of indent per nesting level. This applies to both
ordered and unordered lists, and to mixed nesting (ordered inside unordered
and vice versa).

```
• Top-level item
  • Nested item
    • Deeply nested item
  • Another nested item
• Back to top level

1. First
  1. Sub-first
  2. Sub-second
2. Second
```

All nesting levels use the same `•` bullet character; task items show a
box instead (see Task Lists).

A list nested directly in a list item follows the item's text without a
blank line. A list inside a quote or footnote gets the blank line it
gets anywhere else, also when the quote or footnote sits in a list item.

A single list item may contain multiple paragraphs (a "loose" list).
Subsequent paragraphs are separated from the first by a blank line and
aligned to the same continuation indent — i.e. they sit past the bullet,
vertically aligned with the wrapped lines of the first paragraph rather
than back at the left margin. When nothing has been drawn after the
bullet, for example because the item starts with an HTML block that is
not shown or an empty code block, the next paragraph starts on the line
under the bullet without a blank line, like a quote or code block in
the same place.

## Tables

Tables use full Unicode box-drawing characters with single-line borders.
A table whose columns would have to break a word is laid out as records
instead (see Records).

- **Alignment**: a column of a boxed table follows its delimiter row:
  `:--` left, `:-:`
  centered, `--:` right, for header and data cells alike. A column
  without a colon has its header **centered** and its data
  **left-aligned**. Alignment applies per wrapped line when cell text
  wraps across multiple lines.
- **Header row**: rendered **bold**, on top of each cell's own inline
  styles.
- **Cell text**: inline formatting renders as in running text (see
  Inline Formatting): bold, italic, strikethrough, inline code in its
  color, links underlined with their dimmed `(url)`, images as
  `[image: alt]` with their dimmed path. A cell wraps like running
  text: a line breaks at whitespace and in front of a `(url)`, so glued
  pieces such as `**bold**,` stay together. A word never breaks in a
  boxed table.
- **Data rows**: 1 space padding on each side.
- **Separators**: horizontal rules between the header and every data row.
- **Column width**: a three-tier algorithm. (1) If the natural cell
  widths fit within the available terminal width, use them. (2)
  Otherwise, distribute the available width proportionally between each
  column's minimum (the widest unbreakable token in that column) and
  its natural width. (3) If even the per-column minimums don't fit, a
  column would get narrower than its widest word, and the table is laid
  out as records.
- **Row height**: each cell is word-wrapped to its allocated column
  width, and the row height is the max wrapped height across its cells.
  Shorter cells are blank-padded on the extra lines so the row aligns
  vertically.

```
┌──────────┬────────────────────────────┐
│  Level   │           Items            │
├──────────┼────────────────────────────┤
│ Blocker  │ Auth system, replace CLI   │
├──────────┼────────────────────────────┤
│ High     │ SSE reconnect, test infra  │
├──────────┼────────────────────────────┤
│ Low      │ Dead code cleanup          │
└──────────┴────────────────────────────┘
```

Box-drawing characters used:

| Position       | Character |
|----------------|-----------|
| Top-left       | `┌`       |
| Top-right      | `┐`       |
| Bottom-left    | `└`       |
| Bottom-right   | `┘`       |
| Top-T          | `┬`       |
| Bottom-T       | `┴`       |
| Left-T         | `├`       |
| Right-T        | `┤`       |
| Cross          | `┼`       |
| Horizontal     | `─`       |
| Vertical       | `│`       |

### Records

A table is laid out as records as soon as one of its columns would get
narrower than the widest word of its cells, so that a word would have to
break. A long URL is a word like any other. Each data row becomes a
record, each column a field: the column's name from the header row,
**bold** and dimmed on top of its own styles, then the row's cell. A
plain rule of `─` across the width separates two records.

```
step   retry
owner  jaw
doc    policy
due    11-02
state  open
prio   high
team   core
area   queue
──────────────────────
step   backoff
owner  ana
doc    notes
due    11-09
state  done
prio   low
team   tui
area   pane
```

- **Side by side**: the names are padded to the widest name, and the
  values start two columns after it. A value wraps like running text in
  the room left, its further lines under its first.
- **Stacked**: when the room beside the names would break a word or a
  character that fits the room under them, every field of the table
  shows its name on a line of its own and its value under it, indented
  by two columns. A word wider than that room breaks either way, so it
  does not by itself stack the fields.
- **Long words**: a word wider than its room breaks between graphemes
  and keeps its styles on every line, so a URL stays dim. A line is
  wider than the width only where one grapheme, such as a wide
  character or an emoji, is wider than the room it gets.
- **Empty cells**: an empty header cell, or a column past the header's
  last cell, has an empty name; when every name is empty, values start
  at the left edge. A field whose cell is empty or missing shows its
  name alone. Every record has the same lines in the same order, so a
  field without a name or a value is a blank line.
- **Header only**: a table without data rows shows its names, one per
  line.
- **Alignment** from the delimiter row does not apply; names and values
  are left-aligned.

## Code Blocks (Fenced)

Fenced code blocks are indented by one nesting level (2 spaces) to visually
separate them from surrounding prose. Content is never wrapped. The
opening/closing fence markers (`` ``` ``) are stripped. The language
identifier from the fence (e.g., `` ```rust ``) drives syntax highlighting
but is not itself displayed. Inside a blockquote, the indent follows the
bar (see Blockquotes).

### Syntax highlighting

When a recognized language identifier is present on the fence, tokens
are colorized using **only the 8 standard ANSI colors** (palette
indices 0–7). Colors are emitted as ANSI-indexed escape sequences,
not truecolor RGB — so the colors visible in the rendered output
are whatever the user's terminal has configured for those palette
slots. The user's terminal theme (Solarized, Gruvbox, a custom
`base16` scheme, plain defaults) flows through automatically with no
configuration on patine's part. Highlighting also honors bold and
italic where the theme specifies them, consistent with patine's
typographic palette.

```
  fn main() {
      println!("Hello, world!");
  }
```

When the fence has **no language identifier**, or the identifier is
not recognized, the entire block falls back to a flat rendering in
the same distinct color used for inline code. This is the same
rendering that applied before syntax highlighting existed.

Supported languages include the full set shipped by
[`syntect`](https://docs.rs/syntect/) and
[`two-face`](https://github.com/CosmicHorrorDev/two-face) (100+
syntaxes — rust, python, js/ts, go, bash, json, yaml, toml,
markdown, dockerfile, nix, and many more). Patine does not attempt
to auto-detect languages from code content; an explicit fence
identifier is required for highlighting.

## Blockquotes

Blockquote markers (`>`) are stripped. A thin vertical bar `│` is drawn
on the left in cyan. The text keeps its normal style, so emphasis, bold
and inline code look as they do outside a quote. Programs using the
library can choose another bar color, or a dim bar without a color,
through `RenderOptions::quote_style`.

```
│ This is a blockquoted paragraph. It wraps at the terminal
│ width like normal text, with the bar continuing on each line.
│
│ Multiple paragraphs within a blockquote are separated by a
│ blank line, still prefixed with the bar.
```

Nested blockquotes add additional `│` bars:

```
│ Outer quote
│ │ Inner quote
```

Block-level elements nested inside a blockquote (such as tables, code
blocks, lists, and horizontal rules) carry the `│ ` bar prefix on
every rendered line — including border, header, separator, and data
lines for tables.

Prefixes follow the nesting from the outside in. A quote's bar comes
before the indent of a code block or list item inside it, and a quote
inside a list item sits after the item's indent:

```
│   let x = 1;          ← code block in a quote
│ • Item
│   • Nested item       ← list nested in a list in a quote
• Item
  │ Quote in the item
```

## Horizontal Rules

Rendered as a line of `─` characters spanning the full terminal width.

```
────────────────────────────────────────────────────────────────────────────────
```

## Images

Images cannot be displayed in the terminal. The alt text is shown, prefixed
with an image marker, followed by the file path or URL in dimmed style.

```
[image: Architecture diagram] (./docs/architecture.png)
                               ^^^^^^^^^^^^^^^^^^^^^^^^
                               dimmed
```

If no alt text is provided:

```
[image] (./docs/screenshot.png)
```

The dimmed `(url)` block follows the same atomic-wrapping rule as for
links: if it doesn't fit alongside the image marker on the current
line, it moves to a fresh line below rather than being broken
mid-string, except in a value of a table laid out as records (see
Tables).

## Style Summary

The default terminal color is used everywhere unless noted otherwise.
Only a few elements deviate from the default — the goal is minimal,
restrained use of color and styling.

| Element             | Style                              |
|---------------------|------------------------------------|
| H1                  | Bold, italic, double-underlined    |
| H2                  | Bold, underlined                   |
| H3–H6               | Bold                               |
| Body text           | Default                            |
| **Bold**            | Bold                               |
| *Italic*            | Italic                             |
| `inline code`       | Distinct color (one of few colored)|
| Code blocks (known lang) | Syntax-highlighted, ANSI 8-color palette |
| Code blocks (unknown/no lang) | Distinct color (same as inline)    |
| Link text           | Underlined                         |
| Link URL            | Dimmed                             |
| Done task           | Dimmed (nested lists excepted)     |
| Blockquote text     | Default                            |
| Blockquote bar `│`  | Cyan                               |
| Table borders       | Default                            |
| Table header text   | Bold, centered unless aligned      |
| Table cell text     | As in running text                 |
| Image marker        | Default                            |
| Image path/URL      | Dimmed                             |
| Horizontal rule     | Default                            |
