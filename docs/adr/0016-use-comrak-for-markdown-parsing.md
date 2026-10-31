# 16. Use comrak for Markdown parsing

Date: 2026-10-31

## Status

Accepted

Supersedes [2. Use markdown-rs for Markdown parsing](0002-use-markdown-rs-for-parsing.md)

## Context

markdown-rs 1.0.0 (the `markdown` crate) panics on small inputs instead
of returning a tree or an error. Inputs that make `render_lines_with`
panic:

- `[a]:b\n=\n=+\r-`
- `1. ~~~\n*` and `- ~~~\n1.`
- `---\n\n-` and `---\n\n[^a]:` with the front matter option on
- `+++\n\n- *   `

1.0.0 is the latest release of markdown-rs. Its repository has had no
push since 2025-04-23 and has about a dozen open panic reports with
unmerged fixes.

comrak 0.59.0 was released on 2026-10-26. It returns an AST and covers
GitHub Flavored Markdown tables, strikethrough, task lists, autolinks and
footnotes. The inputs above parse with it without a panic.

ADR 2 chose markdown-rs over comrak for five reasons. Against comrak 0.59.0
in patine:

1. comrak's nodes sit in a `RefCell`. The renderer only takes shared
   borrows of them (`node.data.borrow()`) and never changes the tree, so
   no borrow conflict arises.
2. comrak's nodes live in an `Arena`. `render_lines_with` creates one per
   call, and the renderer passes nodes as `&'a AstNode<'a>`.
3. The renderer walks the tree by recursion over `children()`, as it did
   with markdown-rs. comrak's `Traverse` iterator is not used.
4. comrak's table node holds the column alignments. The header row is the
   first row of the table.
5. markdown-rs 1.0.0 is a stable release and still panics.

## Decision

patine parses Markdown with comrak 0.59.0, with its default features
off. The default features pull in syntect with Oniguruma, which is C
code, and patine already has syntect with fancy-regex. Without them the
dependency tree is pure Rust: the lockfile gains 17 packages and loses 2.

comrak's dependency `memchr-n` needs Rust 1.89, so `rust-version` is
1.89.

The public API does not change.

YAML (`---`) and TOML (`+++`) front matter are split off the start of the
document by patine before parsing, and the rest of the document is parsed
without front matter. comrak's front matter rules differ from the ones
patine keeps: it does not accept spaces or tabs after a fence, a lone
`\r` as a line ending or an empty block, and it looks for a closing fence
with `\r\n` line endings before one with `\n`. A fence is three markers
and optional spaces or tabs alone on a line, and the closing fence is
the first later line that is the same fence.

Image alt text is the text of the image's children, as before. A hard
break, a nested image and a footnote reference add nothing to it.

The renderer nests block quotes, list items and footnote definitions 64
levels deep. Blocks below that depth render in document order at the indent
they have reached, without a bar or bullet of their own, so their text is
kept. The indent and the bars stop growing before a line is left with less
than 20 columns of text, and a prefix of 6 columns is always kept. Inline
content is walked with a stack of its own, not by recursion. A thread with a
2 MiB stack renders documents nested 100,000 block quotes deep or 20,000
emphasis levels deep.

Documents that follow CommonMark and GFM render as before. On edge input
where markdown-rs departed from CommonMark or GFM, the output now follows
comrak (see the list below).

### Output that changes

- Reference links (`[text][ref]`, `[ref][]`, `[ref]`) and reference images
  (`![alt][ref]`) render as inline links and images do: the link text
  underlined with the dimmed URL after it, and the image as
  `[image: alt] (url)`. The definition lines stay invisible. Before, a
  reference link rendered as its plain text and a reference image
  disappeared.
- Footnote definitions render at the end of the document, in the order of
  the first references to them, wherever they are written. Before, they
  rendered where they were written.
- A footnote definition that no reference points to is not shown. Before,
  it rendered in place.
- A row has as many cells as the delimiter row has columns, and a cell
  past the last column is dropped, as the GFM specification says.
  Before, such a cell got a column of its own.
- A task marker with no text (`- [x]`, `- [ ]`) is an empty task item: the
  box alone. Before, it was a plain item with the marker as text.
- A document lays out 100,000 table cells at most, the cells the parser
  fills in to complete a row included. A table that would take it past that
  renders one line of text per row, wrapped like running text, with ` | `
  between the cells and the empty cells at the end of a row left out.
- Autolink literals follow comrak's rules at their edges:
  - `ftp://` addresses are links.
  - An address is not a link while a `[` before it is still open
    (`[ https://example.com`, `[https://example.com]`).
  - A `www.` address is not a link right after a `]`, and is a link only
    in lowercase.
  - A `]` straight after an address belongs to its link.
- Edge input where markdown-rs departed from CommonMark or GFM follows
  comrak:
  - Emphasis next to punctuation and the rule of 3 (`*a *b c**~ x`,
    `~~**a b* c* d~~`).
  - Tabs after a list marker in indented code (`-\t\tcode`).
  - A footnote reference inside link text (`[a[^1]b](u)`) no longer
    prevents the link.
  - Entities in an angle autolink are decoded (`<https://a.b/&amp;c>`
    shows `&`).
  - A `1.` with nothing after it is text, not an empty list item
    (`-\n1.`, `> aa\n1.`).
  - Numeric references to control characters decode to the character
    (`&#27;` is an escape character, not U+FFFD).
  - In a table row, a pipe after a backslash pair (`\\|`) is escaped and
    does not end the cell, so `| a \\| b |` over a two-column delimiter
    row is no table.
  - A byte order mark after front matter is dropped.
- An indented code block that starts on the line of a list marker keeps
  its lines together; a blank line used to separate them. A blank line
  inside indented code in a quote carries the code's indent.
- A list item that starts with an empty line in a document with `\r\n` or
  `\r` line endings (`-\r\n  text`) holds the text that follows, as it does
  with `\n` line endings. Before, the text became a paragraph after an
  empty item. Line endings no longer change how a document reads here.

## Consequences

- ADRs 3, 6, 7, 8, 9 and 14 are not edited. Each describes markdown-rs
  as it was when written:
  - ADR 3 walks the tree markdown-rs returns and names its `Node` type.
  - ADR 6 says markdown-rs reports syntax errors only for MDX.
  - ADR 7 records what markdown-rs 1.0 produces for its inputs.
  - ADR 8 describes markdown-rs making every inline element its own node.
  - ADR 9 records what markdown-rs 1.0 produces for links.
  - ADR 14 describes a column past the header's last cell, which comrak
    drops.
