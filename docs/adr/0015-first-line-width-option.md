# 15. First line width option

Date: 2026-10-30

## Status

Accepted

Amends [7. Render options for soft breaks and HTML](0007-render-options-for-soft-breaks-and-html.md)

## Context

asqr draws a marker, `◀ default`, at the right end of the first line of
an option's text. Only that line shares its row with the marker. patine
lays out every line for one width, so asqr rendered the whole text
narrower to keep the marker's room free, which left an empty column
under the marker.

asqr cannot narrow the first line alone from outside patine. Under
`SoftBreaks::Break` a line ended by a line break and a line ended by
wrapping look the same in the output, so the output cannot be wrapped
again; and cutting the source after the first line breaks lists,
quotes, tables and frontmatter.

## Decision

`RenderOptions` gets a field `first_line_width: Option<usize>` and a
`const` builder `with_first_line_width(width)`. `None`, the default,
lays out every line at the full width.

While no line of the output is finished, the renderer's content width
is the first line width where that is narrower. Everything that takes
its width from there sees it: running text wraps its first line there,
a list item's bullet and a quote's bars count against it, a thematic
break is as long, and a table that starts the output is laid out for it
on all its rows, since a table is laid out once. Later lines take the
full width, a list item's continuation under its text and a quote's
behind its bars. Code blocks, frontmatter and literal HTML blocks are
never wrapped, so their first line may be wider, as their other lines
may be wider than the width. A word wider than the first width
overflows the first line, as a word wider than the width overflows any
line.

A table cell is laid out by a renderer of its own at its column's
width; the option does not reach it.

The command-line interface has no flag for it.

## Consequences

- Rendering without the option is unchanged.
- A table at the start of the output is narrower than one further down.
