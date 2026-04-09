# Output Style Specification

This document defines how patine renders Markdown to styled terminal output.
All examples show the visual output, not the Markdown source.

## Global Layout

- Content starts at the left terminal edge (no global indent).
- Blank lines separate block-level elements (headings, paragraphs, lists,
  tables, code blocks, horizontal rules).
- Long lines wrap at the terminal width.
- Nested structures (lists, code blocks) are indented 2 spaces per level.
- Wrapped continuation lines inside list items align past the bullet/number.

## Headings

Markdown `#` markers are stripped. A blank line appears after each heading.

- **H1**: italic + underlined, default terminal color.
- **H2–H6**: bold, default terminal color.

```
AI Templates: POC → Product — Summary    ← italic, underlined
                                         ← blank line
Business Decisions (need alignment)      ← bold
                                         ← blank line
Sub-Section Heading                      ← bold
```

No color differentiation between heading levels — styling relies on
italic/underline for H1 and bold for all others.

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

For autolinks and bare URLs, the entire URL is rendered underlined with the
URL in dimmed parentheses.

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

Bullet character is `•`, followed by a space, then content.

```
• First item with some text
• Second item with some text
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

All nesting levels use the same `•` bullet character.

## Tables

Tables use full Unicode box-drawing characters with single-line borders.

- **Header row**: text **centered** within the column, rendered **bold**.
- **Data rows**: text **left-aligned** with 1 space padding on each side.
- **Separators**: horizontal rules between the header and every data row.
- **Column width**: sized to fit the widest cell content in each column.

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

Bold markers inside table cells (e.g., `**Blocker**`) are stripped and the
text is rendered bold.

## Code Blocks (Fenced)

Fenced code blocks are indented by one nesting level (2 spaces) to visually
separate them from surrounding prose. Content is displayed in a distinct
color (same color as inline code) and is never wrapped.
The opening/closing fence markers (`` ``` ``) are stripped.

```
  fn main() {
      println!("Hello, world!");
  }
```

The language identifier from the fence (e.g., `` ```rust ``) is not displayed
but is preserved internally to support syntax highlighting in a later version.

> **Future:** Syntax highlighting will colorize tokens based on the language
> identifier. The rendering architecture should accept a language hint per
> code block and pass it through even if initially unused.

## Blockquotes

Blockquote markers (`>`) are stripped. A thin vertical bar `│` is drawn on
the left, with the text rendered in dimmed/italic style.

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

## Style Summary

The default terminal color is used everywhere unless noted otherwise.
Only a few elements deviate from the default — the goal is minimal,
restrained use of color and styling.

| Element             | Style                              |
|---------------------|------------------------------------|
| H1                  | Italic, underlined                 |
| H2–H6              | Bold                               |
| Body text           | Default                            |
| **Bold**            | Bold                               |
| *Italic*            | Italic                             |
| `inline code`       | Distinct color (one of few colored)|
| Code blocks         | Distinct color (same as inline)    |
| Link text           | Underlined                         |
| Link URL            | Dimmed                             |
| Blockquote text     | Dimmed, italic                     |
| Blockquote bar `│`  | Dimmed                             |
| Table borders       | Default                            |
| Table header text   | Bold, centered                     |
| Image marker        | Default                            |
| Image path/URL      | Dimmed                             |
| Horizontal rule     | Default                            |
