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

## Suggested fix

Make the writer public, for example `patine::write_lines(lines:
&[StyledLine], writer: &mut impl Write) -> Result<()>`, documented as
the writer `render` uses. Then ntropy drops its copy and the parity test.
A minor release, since it only adds API.

## Relations

- Relates to: [Show control characters harmlessly in the patine CLI](01M68XJBM8WEW7FG05SMET774Y-show-control-characters-harmlessly-in-the-patine-cli.md)
