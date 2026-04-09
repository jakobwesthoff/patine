# 4. Wrap text at terminal width, never wrap code blocks

Date: 2026-04-09

## Status

Accepted

## Context

The initial design specified a hardcoded maximum content width of 79
characters for word wrapping. During implementation, this was reconsidered:

- A fixed width ignores the actual terminal size. Users with wide terminals
  waste space; users with narrow terminals get double-wrapped lines (once
  by patine, once by the terminal emulator).
- The 79-character limit was borrowed from a convention for source code and
  plain text files, but terminal rendering is a different context — the
  output adapts to the display, not the other way around.
- Code blocks must never be wrapped because they are verbatim content
  where line breaks carry semantic meaning (indentation, alignment,
  syntax).

## Decision

- **Text blocks** (paragraphs, headings, list items, blockquotes) wrap at
  the terminal width, queried at startup via `crossterm::terminal::size()`.
  The width is passed into the renderer as a parameter.
- **Code blocks** (fenced and indented) are rendered verbatim and never
  wrapped, regardless of terminal width.
- When the terminal width cannot be determined (e.g., output is piped to a
  file), the caller falls back to a default of 80 columns.

The `render()` API takes `terminal_width: usize` explicitly rather than
querying the terminal internally, keeping the rendering logic pure and
testable with any width.

## Consequences

- Tests can pass an arbitrary width to exercise wrapping behavior at
  specific column counts without depending on the test runner's terminal.
- A future `--width` flag can override the detected terminal width
  trivially — just pass a different value to `render()`.
- Output piped to a file or another program uses the 80-column default,
  which may not match the user's terminal. A `--width` flag would address
  this.
- Code blocks that exceed the terminal width will cause horizontal
  scrolling or terminal-level wrapping. This is intentional — breaking
  code lines would be worse.
