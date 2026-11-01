# 17. Offer the ANSI encoding as a Display adapter

Date: 2026-11-01

## Status

Accepted

Amends [6. Render to style-neutral lines](0006-render-to-style-neutral-lines.md)

## Context

ADR 0006 gives patine two outputs: `render` writes ANSI escape
sequences, and `render_lines` returns style-neutral lines. The writer
between them was private. A program that changes the lines between
layout and output, such as one that escapes control characters in the
text or expands tabs, had to copy that writer to get the same escape
sequences.

`TextStyle` is `#[non_exhaustive]`, so a minor release may add a field.
A copied writer does not know such a field and drops it without a
warning.

## Decision

### `patine::ansi`

```rust
pub fn ansi(lines: &[StyledLine]) -> AnsiLines<'_>;

#[derive(Clone, Copy, Debug)]
pub struct AnsiLines<'a> { /* private */ }

impl fmt::Display for AnsiLines<'_> { /* ... */ }
```

`ansi` sits at the crate root. `AnsiLines` formats as the lines with the
escape sequences of ADR 0006: each span's attributes switched on, its
text, the attributes switched off in reverse order, and a newline after
every line. `render` and `render_with` write through it, so for the
lines of `render_lines` the bytes are the ones `render` writes.

A `Display` value writes into a `String` and into any `io::Write`, so
the API has no writer parameter and no error type of its own. Formatting
fails only when the destination fails. The lines are a slice, since a
`Display` value may be formatted more than once. `AnsiLines` is a named
type, so a caller can store it and gets `Clone`, `Copy` and `Debug`; its
field is private.

### What `ansi` writes

patine encodes styles only. Span text is written as it is: control
characters and tabs stay with the caller, who knows how its output is
shown.

Lines a caller builds are written as given. An empty span gets its on
and off sequences around nothing. A span holding a line ending writes
it with the span's style still on, so the style reaches into the next
line until the span ends. The documentation of `ansi` and of
`StyledSpan` says so.

### `TextStyle::without_colors`

```rust
#[must_use]
pub const fn without_colors(self) -> Self;
```

returns the style with every color field cleared, the foreground, and
every other attribute kept. It names every field of `TextStyle` without
`..`, so a field added later does not compile there until it is decided
whether it is a color. A program that writes without colors maps its
spans' styles through it before `ansi`.

## Consequences

- `render`'s error for a failing writer reads `write the rendered lines`
  over the writer's error, where it named the span or the newline that
  failed. The writer's error stays the root cause.
- On `tests/fixtures/showcase.md` at width 80, `render` makes 576 write
  calls into an unbuffered writer where it made 466, for the same 6026
  bytes. The `patine` binary writes to a locked stdout, which is
  line-buffered.
- Plain text output needs no third function: mapping every style to
  `TextStyle::default()` before `ansi` writes no escape sequence.
