# Constructs that change with the parser

Reference links: full [the text][r1], collapsed [r1][], shortcut [r1] and
undefined [text][nope].

Reference image: ![alt text][img] and ![img][] and undefined ![alt][nope].

A footnote reference[^one] and another[^two], referenced in reverse order of
the definitions below[^two][^one].

[^one]: The first footnote, written first.

Text between the definitions.

[^two]: The second footnote, with a [link](https://example.com).

[^unused]: Nobody refers to this footnote.

| a | b |
|---|---|
| 1 | 2 | 3 |
| 4 |

- [x]
- [ ]
- [ ] 
- [x] with text

Autolink literals: ftp://example.com/file, [ https://example.com after a bracket,
WWW.example.com, [a]www.example.com, https://example.com/a] and www.example.com].

[r1]: https://example.com/reference "Title"
[img]: https://example.com/image.png
