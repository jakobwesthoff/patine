# 21. Show control characters as pictures in the CLI

Date: 2026-11-01

## Status

Accepted

## Context

ADR 17 leaves the text of a span to the caller of `patine::ansi`:
control characters and tabs are written as they are. The `patine`
command wrote them as they were, so a document could run an escape
sequence in the terminal that rendered it. A raw ESC reached the
terminal, and so did a numeric character reference such as `&#27;`,
`&#x1b;` or `&#x7f;`, which the parser decodes to the control character.

Controls reach a span in these places: running text, headings, code
spans and code blocks with and without highlighting, link text, image
alt text, table cells, footnotes, quotes, callouts, lists, frontmatter
and link and image destinations, which the command shows as a dim
suffix. Whitespace splitting turns U+0085, vertical tab, form feed, CR
and LF in running text and inline code into spaces before a span
exists; a code block keeps U+0085, vertical tab and form feed.

Every character the command writes to stdout is span text, apart from
the SGR sequences and the newline `ansi` adds. On stderr, the error for
a file that cannot be read names the file as given, and clap's error
for a bad command line quotes the argument it rejects. Both can hold
controls.

## Decision

The command renders with `render_lines`, replaces the controls in the
text of every span and writes the result with `patine::ansi`. This
decision leaves the library alone, so other front ends keep deciding
for themselves what to do with controls.

A control is a C0 character (U+0000 to U+001F), DEL (U+007F) or a C1
character (U+0080 to U+009F), other than tab.

- C0 characters become their Control Pictures, U+2400 to U+241F in
  order, so `␛` stands for ESC. DEL becomes U+2421.
- C1 characters become U+FFFD, as the Control Pictures block has no
  pictures for them.
- Tab stays.

A picture keeps the reader able to see which control the document held.
U+FFFD hides that, so it is used only where no picture exists.

Each replacement is one character that unicode-width counts as one
column, as it counted the control. A control is a grapheme cluster of
its own, apart from CR directly followed by LF, which no span holds
(ADR 22), so no line gets wider than the layout measured.

## Consequences

- The messages on stderr get the same replacement: the file name in the
  error for a file that cannot be read, and the argument that clap
  quotes for a bad command line. The command line is parsed a second
  time with the controls in its arguments replaced, and clap's message
  for that parse is the one printed. The file name used to read the file
  is the one given.
- The output is UTF-8 and is harmless on terminals that decode UTF-8. A
  terminal that acts on 8-bit C1 bytes reads the UTF-8 encoding of a
  Control Picture, whose last byte lies between 0x80 and 0x9F, as it
  reads any non-ASCII letter, and patine does not guard against that.
- U+FFFD has East Asian width Ambiguous. A terminal that draws ambiguous
  characters in two columns draws a C1 replacement in two columns while
  the layout counts one. The box-drawing characters patine writes are
  Ambiguous as well.
- Characters that are not controls stay, among them the bidirectional
  overrides U+202A to U+202E and U+2066 to U+2069, U+2028 and U+2029 and
  zero-width characters.
