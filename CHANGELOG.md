# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
