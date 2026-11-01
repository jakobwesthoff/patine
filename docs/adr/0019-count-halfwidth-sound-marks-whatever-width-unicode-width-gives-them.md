# 19. Count halfwidth sound marks whatever width unicode-width gives them

Date: 2026-11-01

## Status

Accepted

Amends [13. Measure widths grapheme by grapheme](0013-measure-widths-grapheme-by-grapheme.md)

## Context

ADR 13 measures each extended grapheme cluster with unicode-width and
adds one column for each halfwidth Katakana sound mark, `ﾞ` (U+FF9E) or
`ﾟ` (U+FF9F), because unicode-width gave them no width.

unicode-width 0.2.3 gives each of the two marks one column. patine
requires `unicode-width = "0.2.2"`, which admits 0.2.3. With 0.2.3 the
rule of ADR 13 counted each mark twice: `ｶﾞ` took 3 columns and `ﾟ`
took 2.

## Decision

A grapheme cluster takes its unicode-width width plus, for each
halfwidth sound mark in it, the columns unicode-width leaves out of that
mark's one column: one with unicode-width 0.2.2, none with 0.2.3. Each
mark takes one column under both versions, and the cluster is measured
with the marks in it.

Everything else in ADR 13 stays: widths are measured grapheme by
grapheme, the combining sound marks U+3099 and U+309A keep their Unicode
width, and so do control characters.

## Consequences

- The halfwidth sound marks lay out the same under unicode-width 0.2.2
  and 0.2.3.
- unicode-width 0.2.3 moves from Unicode 17 to Unicode 18. A character
  that Unicode 18 assigns can measure differently under the two
  versions, as unicode-width decides. U+05C8 takes one column under
  0.2.2 and none under 0.2.3.
- Measured as a whole string, unicode-width 0.2.3 gives `ｶﾞ` the width
  patine gives it, 2, and unicode-width 0.2.2 gives it 1. The ligatures
  of ADR 13, such as `لا`, measure 1 as a whole string under both
  versions and 2 for patine.
