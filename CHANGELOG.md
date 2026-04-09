# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
