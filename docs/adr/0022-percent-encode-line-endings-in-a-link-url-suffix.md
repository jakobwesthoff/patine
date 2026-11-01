# 22. Percent-encode line endings in a link URL suffix

Date: 2026-11-01

## Status

Accepted

Builds on [17. Offer the ANSI encoding as a Display adapter](0017-offer-the-ansi-encoding-as-a-display-adapter.md) and [21. Show control characters as pictures in the CLI](0021-show-control-characters-as-pictures-in-the-cli.md)

## Context

The documentation of `StyledSpan` promises that the text of a span patine
builds never contains a line ending, `\n` or `\r`. A character reference
in the destination of a link or image, such as `&#13;` or `&#10;`,
decodes to the character, and the dim `(url)` suffix put it into its
span. A CR written to a terminal moves the cursor to the start of the
line, so text after it, such as a second URL, covers the link text.
Running text and code blocks hold no line ending in a span: running
text joins its lines or breaks them, and code is split into lines.

A link whose text equals its destination gets no suffix. The text of a
link whose destination holds a line ending draws that character as a
space, so it never equals the destination as the suffix would show it.

## Decision

The suffix writes CR as `%0D` and LF as `%0A`. This is the encoding
comrak's HTML output gives these characters in an `href`
(`escape_href`), which keeps a `%0D` already in the source as it is. A
suffix with `%0D` therefore stands for a CR or for the three characters
`%0D` in the source.

A link whose destination holds a CR or LF always gets the suffix, so the
destination shows although the text repeats it.

Other control characters stay in the suffix as they are, for the front
end to draw (ADR 17). The `patine` command replaces them (ADR 21).

## Consequences

- No span of `render_lines` holds a line ending. The test
  that renders random documents asserts it, and its fragments include
  references to CR and LF in destinations.
- The suffix is built before the layout measures it, so `%0D` counts
  three columns.
- A CR or LF reaches the `patine` command's replacement only through a
  file name or a command line argument, where it becomes `␍` or `␊`.
