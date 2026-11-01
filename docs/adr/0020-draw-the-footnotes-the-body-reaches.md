# 20. Draw the footnotes the body reaches

Date: 2026-11-01

## Status

Accepted

Amends [16. Use comrak for Markdown parsing](0016-use-comrak-for-markdown-parsing.md)

## Context

ADR 16 draws footnote definitions at the end of the document in the order
of their first references and leaves out a definition that no reference
points to. It takes the numbers and the reference counts from comrak.

comrak counts every reference, including one inside a definition that it
drops because nothing refers to that definition, and one inside the alt
text of an image, which draws no reference. A definition that only such
references point to is still appended to the document with a number, but
no reference node in the drawn document carries that number. patine 2.2.1
panicked on that input:

    text

    [^z]: sees[^b]

    [^b]: B

    later[^c]

    [^c]: C

Two other kinds of input rendered without a panic and showed the same
mismatch. Definitions that refer only to each other or to themselves
were drawn although nothing in the body leads to them, and took numbers
from the notes the body does reach. A footnote first referenced from a
dropped definition and later from the body took its number from the
dropped reference.

## Decision

A footnote is drawn when a reference in the body of the document points
to it, or a reference in a drawn footnote does, in its text, headings,
tables or lists alike. A reference in the alt text of an image is not
drawn and does not count. A definition that stands where it was written
because a later definition of the same name replaced it is part of the
body.

Footnotes are numbered from 1 in the order of their first drawn
reference in the source. The renderer works the numbers out itself and
sets them on the reference nodes. When every footnote is drawn they are
the numbers comrak gives. The notes are drawn in the order of their
numbers.

## Consequences

- No combination of definitions and references makes the renderer
  panic.
- A definition that references only itself or other undrawn definitions
  is not shown, and its references take no numbers.
- A reference in the alt text of an image does not make its note appear.
- ADR 16 states that a definition no reference points to is not shown.
  This ADR extends that to a definition that only undrawn definitions or
  alt text point to.
