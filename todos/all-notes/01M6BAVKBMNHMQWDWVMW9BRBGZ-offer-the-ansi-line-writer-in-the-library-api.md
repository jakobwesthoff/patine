---
title: "Offer the ANSI line writer in the library API"
kind: improvement
component: render
origin: request
tags: [api-design]
---
# Offer the ANSI line writer in the library API

patine's writer that turns `StyledLine`s into text with SGR escape
sequences (`write_lines`, `sgr_on` and `sgr_off` in `src/ansi.rs`) is
crate-private. A library user who changes the lines between layout and
output has to copy it. ntropy 2.5.0 does exactly that for `ntropy show`:
it escapes control characters in each span and expands tabs after
`render_lines_with`, then writes the lines with its own copy of the
writer (ntropy `src/render/ansi.rs`), kept equal to patine's by a test
that compares both outputs byte for byte.

## Problem

`TextStyle` is `#[non_exhaustive]`. A style a later patine release adds
reaches patine's writer but not a copied one, which then drops it
silently; ntropy's ADR 0082 records this as a known cost.

## Decisions

2026-11-01, the owner, through asqr (session 01M6BEQJJM51SM71807CJ99SFH),
after a discussion document with four options and an adversarial review
of it (logbook `plans/patine-api-discussion.md`):

- A `Display` adapter over lines as a free item, not a `write_lines`
  function: `patine::ansi(lines: &[StyledLine]) -> AnsiLines<'_>`, at the
  crate root, documented as the encoding `render` uses; `render` writes
  through it. It writes into a `String` and any `io::Write` alike, so
  the API fixes no writer parameter and no error type.
- `AnsiLines` is a named type with `Clone`, `Copy` and `Debug`, not an
  opaque `impl Display`.
- `TextStyle::without_colors(self) -> Self` (`#[must_use]`, `const`) is
  added now: it clears every colour field, so a colour field a later
  patine adds never shows in a caller's colourless output.
- Control characters and tabs stay with the caller: patine encodes
  styles only. The CLI's control pictures (the related todo) live in
  `main.rs`.
- Lines patine does not build itself are written as given, and the docs
  say what that means: an empty span gets its on and off sequences, a
  span holding a line ending is written with the line ending inside its
  style.

Then ntropy drops its copy of the writer and its parity test, a minor
release since it only adds API.

## Relations

- Relates to: [Show control characters harmlessly in the patine CLI](01M68XJBM8WEW7FG05SMET774Y-show-control-characters-harmlessly-in-the-patine-cli.md)
