# 25. Number footnotes in source order and draw the ones comrak keeps

Date: 2026-11-24

## Status

Accepted

Supersedes [20. Draw the footnotes the body reaches](0020-draw-the-footnotes-the-body-reaches.md)

Amends [16. Use comrak for Markdown parsing](0016-use-comrak-for-markdown-parsing.md)

## Context

ADR 20 gave the renderer its own rules for the footnotes it draws and for
their numbers, because the numbers and reference counts of comrak 0.59
counted references that nothing drew: those in a definition the parser
dropped, and those in the alt text of an image. patine 2.2.1 panicked on
such input.

comrak 1.2.0 detaches a definition that no reference reaches from the body
or from a drawn definition, together with the references in it, and
detaches a later definition of a label. It leaves the definitions it draws
at the end of the document in the order of its own numbers, and every
reference that stays in the document is drawn. It numbers the references of
the body first and those inside definitions after them. GitHub numbers the
footnotes in the order of the first reference in the source, a reference in
a definition counting where the definition is written.

## Decision

The footnotes drawn are the ones comrak keeps. The numbers are GitHub's.

- A footnote is drawn when a reference reaches it from the body, or from a
  drawn footnote. The first definition of a label is the footnote. A
  definition that nothing reaches, or that only undrawn definitions refer
  to, or that repeats a label, is not shown, and its references count for
  nothing.
- The footnotes are numbered from 1 in the order of the first drawn
  reference in the source. A reference in a definition counts where the
  definition is written, and a reference in the alt text of an image counts
  at its place. The notes are drawn at the end of the document in the order
  of their numbers.
- A footnote reference in the alt text of an image is drawn with its number,
  dimmed like a reference in running text, and its note is drawn:
  `![bild[^1]](u.png)` shows `[image: bild[1]] (u.png)`. The alt text keeps
  its other rules: a soft break is a space, inline HTML shows as written, a
  hard break adds nothing. A nested image adds only the numbers of the
  references in it. Alt text with a number and no text is `[image: [1]]`,
  and `[image]` is the alt text with neither.
- Every drawn note's number is carried by a drawn reference, and every drawn
  reference points to a drawn note. No combination of definitions and
  references makes the renderer panic.

The renderer takes the n-th definition at the end of the document as the
footnote comrak numbered n, and maps that number to the one the references
show. It sets that number on every reference, those in alt text included.

Example: `x[^a]`, `[^a]: A[^c]`, `[^c]: C`, `y[^b]`, `[^b]: B` and
`![bild[^d]](u.png)`, `[^d]: D`, each on its own paragraph, show `x[1]`,
`y[3]`, `[image: bild[4]] (u.png)`, and the notes `[1] A[2]`, `[2] C`,
`[3] B`, `[4] D`. GitHub numbers the same way.

GitHub differs where a reference stands in a definition that comrak does not
draw. With `text`, `[^z]: sees[^b]`, `[^b]: B`, `later[^c]` and `[^c]: C`,
GitHub shows `later[2]` and draws the note `B` with no reference to it.
patine shows `later[1]` and draws only the note `C`.

This supersedes ADR 20's rules for the drawn set and the numbers. It
replaces two statements of ADR 16: "A hard break, a nested image and a
footnote reference add nothing to it" for image alt text, and the order of
the first references for the footnote definitions at the end of the
document. ADR 20's rules that a footnote is drawn when it is reached, that
a definition nothing reaches is not shown, that the renderer does not panic
for any combination, and that the notes follow the order of their numbers,
hold as stated above.
