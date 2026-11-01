# 13. Measure widths grapheme by grapheme

Date: 2026-10-22

## Status

Accepted

Amended by [19. Count halfwidth sound marks whatever width unicode-width gives them](0019-count-halfwidth-sound-marks-whatever-width-unicode-width-gives-them.md)

## Context

patine measured text with unicode-width, one string at a time.
Terminals and ratatui draw text grapheme by grapheme, and for some text
the two differ:

- unicode-width gives the halfwidth Katakana sound marks `ﾞ` (U+FF9E)
  and `ﾟ` (U+FF9F) no width, as they extend the grapheme before them.
  Terminals and ratatui draw each of them in a cell of its own. A table
  with a cell of `ｶﾞ` six times, rendered at width 12, fit for patine
  and took 16 columns on screen.
- unicode-width gives some ligatures one column as a string, such as
  the Arabic Lam-Alef `لا` and the Hebrew Alef-Lamed `א‍ל`. Their
  letters are separate graphemes, and drawn grapheme by grapheme they
  take a column each.

A line that patine fit to its width was therefore wider on screen, and
asqr, which counts columns as ratatui draws them, wrapped such a line
again.

## Decision

patine measures every width it lays out through one function: the sum,
over the text's extended grapheme clusters, of each cluster's Unicode
width, plus one column for each halfwidth sound mark, U+FF9E or U+FF9F.
This is how terminals and ratatui draw text. The combining sound marks
U+3099 and U+309A keep their Unicode width, and so do control
characters, which a front end draws as it decides. A front end does not
correct patine's widths on its side.

A word wider than its room breaks between extended grapheme clusters,
the units a terminal and ratatui draw whole.

## Consequences

- `StyledSpan::width` measures its text this way, and
  `StyledLine::width` sums its spans, as a front end draws a line span
  by span.
- A broken word keeps every grapheme whole: `ｶﾞ` stays together as a
  unit two columns wide, and a flag emoji is no longer cut in two.
- patine depends on `unicode-segmentation`.
- ratatui's own `Span::width` and `Line::width` measure a whole string
  with unicode-width, so they differ from patine's widths for these
  texts.
