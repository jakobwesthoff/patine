# 24. Follow comrak 1.2 where it changes the rendering

Date: 2026-11-24

## Status

Accepted

Amends [16. Use comrak for Markdown parsing](0016-use-comrak-for-markdown-parsing.md)

Amends [20. Draw the footnotes the body reaches](0020-draw-the-footnotes-the-body-reaches.md)

## Context

ADR 16 chose comrak 0.59.0. comrak 1.0 to 1.2 specify several of its
extensions and limit how deep containers nest. patine 2.2.3 with comrak
0.59.0 and patine with comrak 1.2.0 render the cases below differently.
The tests pin each case.

## Decision

patine parses with comrak 1.2.0 and renders what it parses. Where the two
versions differ, the output follows comrak 1.2.0:

- Footnotes. The first definition of a label wins, and a later one with
  the same label is not shown, wherever it stands. Labels still match
  without regard to case. A label matches as written: `[^&#97;]` no
  longer matches `[^a]`, and a reference that matches no definition stays
  text with its label read as inline Markdown (`[^*b*]` shows `b` in
  italics). A label of inline code, `` [^`b`] ``, matches its
  definition `` [^`b`]: ``, where it was text. A definition nested in a
  note is a definition of its own: when nothing refers to it, it is
  dropped with its text, and the notes only it reaches are dropped with
  it. A line of one to three spaces no longer ends a definition. No
  definition stays where it was written.
- Nesting. A list, footnote definition, description item or fenced
  container opens only below fewer than 100 ancestors of any kind, block
  quotes and callouts included. Lists inside lists stop at about 50
  levels, and a list below about 100 quotes is text. The deeper markers
  show as text. Block quotes are not limited by comrak and stay at the 64
  levels of the renderer. This replaces ADR 16's sentence that the
  renderer nests "block quotes, list items and footnote definitions 64
  levels deep" for list items.
- Table cells. A backslash pair before a pipe is a pair: `| a \\| b |`
  over a two-column delimiter row is a table with the cells `a \` and
  `b`, and `| c \\| d | e |` splits into `c \`, `d` and `e`, of which the
  third is cut off by the two columns. Only spaces and tabs pad a cell or
  the delimiter row, so a form feed in the delimiter row ends the table.
  This replaces ADR 16's statement that `| a \\| b |` is no table.
- Autolinks. A trailing `_` is stripped before the domain check, so an
  address followed by `_` links, inside emphasis or not
  (`_https://z.com_`, `https://z.com_`). A `www.` address shows its
  `http://` URL after it. `foo&#x40;bar.com` is text. This replaces ADR
  16's statement that a bare address before the `_` that closes emphasis
  is no link.
- Strikethrough. A delimiter without a partner no longer stops the
  emphasis after it: `~~a~ *b*` shows `b` in italics.
- Task lists. A task marker needs a space, a tab or the end of the line
  after it. `- [x]*a*` is a list item showing `[x]`, as is a marker
  followed by a vertical tab or a form feed.

ADR 20's sentence "A definition that stands where it was written because
a later definition of the same name replaced it is part of the body" no
longer applies, as no definition stays in place. Its sentence "When every
footnote is drawn they are the numbers comrak gives" no longer applies:
comrak 1.2 numbers the references of the body before those inside notes.
