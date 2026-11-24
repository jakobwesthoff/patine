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

crossterm 0.29 reads the size from `/dev/tty` before it tries stdout
(`src/terminal/sys/unix.rs`, `window_size`). A run started from a
terminal therefore gets that terminal's width even when stdout goes to
a file or a pipe. This includes `cargo test` started from a terminal,
which runs `tests/cli.rs` with fixed `--width` values.

The library already takes the width as a parameter
(`patine::render_lines`), so the cap belongs in the CLI. Tools that
embed patine, such as asqr, choose their own width.

## Proposal

Change `--width` from an exact width to a maximum instead of adding a
second flag. Two flags that both set a width would be hard to tell
apart in `--help`. A cap is also what most calls of `--width` want.

- stdout is a terminal: the width is `min(terminal width, --width)`.
- stdout is not a terminal: `--width` is used as given. Without this
  rule, `patine -w 120 doc.md > doc.txt` from an 80-column terminal
  would yield 80 columns, since crossterm still finds the terminal
  through `/dev/tty`. The `tests/cli.rs` tests would also depend on the
  width of the terminal `cargo test` runs in.

The README describes `--width` as an override (the usage example, the
word wrapping feature and the `--help` excerpt) and changes with it.
Scripts that rely on output wider than the terminal while stdout is a
terminal lose that. Whether that counts as a breaking change for the
version number is part of the release decision.

## Open questions

- Should the CLI cap the width by default, and at what value? Common
  reading widths sit around 80 to 100 columns. A default changes the
  output of every existing call on wide terminals, so it needs a way
  to switch it off (for example `--width 0` or a `none` value).
- Should the cap apply to tables too? Code blocks never wrap, but a
  table that does not fit the width is laid out as records, one line
  per column. A cap would push wide tables into that layout on
  terminals where they would fit as a grid.
- Should an environment variable such as `PATINE_WIDTH` set the
  maximum, so users do not have to pass the flag on every call?
- Is an exact width still needed while stdout is a terminal, for
  example to check how output looks on a wider display? If so, it needs
  its own spelling.
