---
title: "Show control characters harmlessly in the patine CLI"
kind: bug
component: cli
origin: review
---
# Show control characters harmlessly in the patine CLI

The `patine` command writes the text of every span to the terminal as it
is, so a control character in a Markdown file, such as ESC, reaches the
terminal and can run an escape sequence. Rendering a file someone else
wrote is then not safe to do in a terminal.

## Problem

Found in the adversarial review of the comrak switch on 2026-10-31
(reviewer O, finding O10), by reading the code and a probe; the CLI
itself was not exploited.

- `src/styled.rs:185` leaves control characters in span text for the
  front end to draw ("a front end decides how to draw them"), and the
  CLI's writer in `src/ansi.rs` writes them unfiltered.
- A raw ESC in the source already reached the terminal before the switch.
  With comrak, numeric character references to C0 and C1 controls
  (`&#27;`, `&#x1b;`, `&#x7f;`) decode to the control character too,
  where markdown-rs gave U+FFFD, so an entity-encoded escape sequence now
  reaches the terminal the same way.
- asqr is not affected: its adapter draws control characters as their
  pictures (asqr `src/tui/render/patine_adapter.rs`, asqr ADR 33).

## Suggested fix

Before the CLI writes a span, replace C0 and C1 control characters other
than tab with their Control Pictures (U+2400 block) or U+FFFD, as asqr
does, in text and in link URL suffixes. The library output stays as it
is, so other front ends keep deciding for themselves.
