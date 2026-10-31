---
title: "Number footnote references and their notes"
kind: bug
component: render
origin: discussion
---
# Number footnote references and their notes

A footnote reference (`[^1]`) draws nothing, and since patine 2.0.0 the
footnote definitions render at the end of the document. A reader sees a
note at the end and cannot tell which text it belongs to:
`See a guide[^1] and more.` shows as `See a guide and more.`, with
`The note.` at the end.

## Problem

`docs/output-style.md`, section Footnotes: "A footnote reference (`[^1]`)
draws nothing." Found while moving asqr to patine 2.0.0 (asqr a92839a,
snapshot `references_40` shows "a guide and ."), 2026-10-31.

## Decisions

- 2026-10-31, the owner, after seeing both variants rendered in asqr:
  a reference shows a dimmed number in square brackets, `[1]`, right
  after the text it follows. The notes at the end each start with the
  same dimmed `[1]`. Rejected: superscript digits (`¹`), which some fonts
  show small or uneven and which take two characters from 10 on.
- Numbers count from 1 in the order of the first reference to each
  footnote, the order the notes already render in, whatever the label
  in the source (`[^perf]` becomes `[1]` if it is referenced first).

## Suggested fix

Render `NodeValue::FootnoteReference` as a dimmed `[n]`, numbered in the order
of first references, and prefix each footnote definition
at the end with the same dimmed `[n]` and a space, its continuation lines
indented to the text after it. Update `docs/output-style.md`, ADR 0016
if it describes footnotes, the CHANGELOG and the snapshots.
