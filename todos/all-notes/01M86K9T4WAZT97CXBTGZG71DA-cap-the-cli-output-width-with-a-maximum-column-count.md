---
title: "Cap the CLI output width with a maximum column count"
kind: feature
component: cli
status: needs-discussion
origin: idea
---
# Cap the CLI output width with a maximum column count

On a wide terminal the `patine` CLI wraps running text at the full
terminal width, and lines of 200 or more columns are hard to read. The
CLI should take a maximum column count, wrap at the terminal width or
that maximum, whichever is smaller, and perhaps apply a default maximum
when none is given. Noticed while reading output in a wide terminal on
2026-11-24.

## Context

`src/main.rs`, `main`, takes the width from `--width` / `-w` or else
from `crossterm::terminal::size()`, falling back to 80. `--width` sets
the width exactly: it can make the output wider than the terminal, and
it has to be adjusted by hand whenever the window changes. A maximum
differs from that. It never widens the output and only narrows it on
terminals wider than the maximum.

The library already takes the width as a parameter
(`patine::render_lines`), so the cap belongs in the CLI. Tools that
embed patine, such as asqr, choose their own width.

## Proposal

Add a flag such as `--max-width <COLUMNS>`. The effective width is
`min(terminal width, max width)`. An explicit `--width` wins over both.

## Open questions

- Should the CLI cap the width by default, and at what value? Common
  reading widths sit around 80 to 100 columns. A default changes the
  output of every existing call on wide terminals, so it needs a way
  to switch it off (for example `--max-width 0` or a `none` value).
- Should the cap apply to tables too? Code blocks never wrap, but a
  table that does not fit the width is laid out as records, one line
  per column. A cap would push wide tables into that layout on
  terminals where they would fit as a grid.
- Should an environment variable such as `PATINE_MAX_WIDTH` set the
  maximum, so users do not have to pass the flag on every call?
- Name and short form of the flag, and how `--help` explains its
  relation to `--width`.
