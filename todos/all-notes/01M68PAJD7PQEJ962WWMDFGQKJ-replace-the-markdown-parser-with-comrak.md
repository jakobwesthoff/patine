---
title: "Replace the markdown parser with comrak"
kind: bug
component: render
impact: high
origin: upstream
---
# Replace the markdown parser with comrak

patine parses Markdown with the `markdown` crate 1.0.0 (wooorm/markdown-rs)
and builds its output from the mdast that `to_mdast` returns. That crate
panics on small inputs instead of returning a tree or an error, and it is
no longer maintained, so the panics will not be fixed upstream.

## Problem

Found on 2026-10-31 in asqr (asqr todo 01M67QJJZMZ5S055T5BJJG1MQZ, now
closed; asqr ADR 98 and 99), by a random generator over Markdown
fragments: 24 distinct panic messages in `markdown` 1.0.0. The shortest
inputs, all through `patine::render_lines_with`:

- `[a]:b\n=\n=+\r-`: "Cannot push to non-parent" (`to_mdast.rs:193`)
- `1. ~~~\n*` and `- ~~~\n1.`: "mismatched (non-jsx): ListOrdered /
  ListUnordered" (an unclosed fence in a list item, then an item of the
  other list kind)
- with the frontmatter option patine turns on: `---\n\n-` (index out of
  bounds) and `---\n\n[^a]:` (`Option::unwrap()` on `None`)

asqr now catches the panic and shows the text plain, but every other user
of patine, the `patine` command included, panics.

Upstream state, checked 2026-10-31: `markdown` 1.0.0 is the latest
release; the repository's last push was 2025-04-23; it has 97 open
issues, about a dozen of them panic reports with unmerged fix pull
requests (#271, #265, #263, #219, #213 and others).

## Decision (Jakob, 2026-10-31, in the chat)

Replace the parser with `comrak` rather than wait for upstream or keep a
fork: comrak 0.59.0 is maintained (last push 2026-10-26, 16 open
issues), returns an AST, and covers GitHub Flavored Markdown, footnotes
and frontmatter. Rejected: `pulldown-cmark` (an event stream, not a
tree, so a larger rewrite); filing the drafted bug report upstream (no
one maintains it); forking markdown-rs.

## Before the switch: effort and findings (lead, 2026-10-31)

A preliminary check on patine main 223450c, with probes against comrak
0.59.0 in a scratch crate; nothing in patine was changed. Size: L. The
port is mechanical (M), but six visible output changes need a decision
each, and the renderer's node types change from owned mdast values to
comrak's arena nodes (`&AstNode`), which reaches every signature in
`src/render.rs` and `src/table.rs`.

### Where markdown-rs is used

- `Cargo.toml`: the dependency, and `rust-version = "1.88"`.
- `src/lib.rs:170-187`: `ParseOptions::gfm()` plus `frontmatter: true`,
  `to_mdast(...).expect(...)` (unreachable with MDX off) and the call
  into the renderer.
- `src/render.rs`: `render_node` matches 21 mdast variants (lines
  236-265); about 15 functions take `&markdown::mdast::*` or `&[Node]`;
  a unit test at 1139-1165 builds mdast nodes by hand.
- `src/table.rs`: `row_cells`, `layout_table` and `layout_records` take
  `&[Node]`, and `AlignKind` is used for alignment.
- `src/line_endings.rs`: splits `\r\n` and `\r` inside node values,
  which mdast keeps raw; still needed only if comrak keeps them.
- `tests/lines.rs`, `tests/ansi.rs` and `src/main.rs` use only the
  patine API.

### Mapping

Most variants map one to one: Document, Heading (`level`), Paragraph,
Text, Strong, Emph, Strikethrough, LineBreak, Code (inline), List and
Item (`start` is `usize`), BlockQuote, ThematicBreak, Link (autolinks
and `mailto:` give the same `url`), TableCell, FootnoteReference
(nothing drawn either way). Definitions produce no node in either.
Needs an adapter, same output if done right:

- Soft breaks are `SoftBreak` nodes; mdast keeps the newline inside
  `Text`. A new arm must give what the text path gives today (tests
  `tests/rendering.rs:2914-2999`).
- `CodeBlock.info` is the whole info string: split off the first word as
  the language, and trim the one trailing newline of `literal`.
- `FrontMatter` keeps the fences and the blank line after them: strip
  them.
- Images have inline children instead of an `alt` string: flatten them
  to text (`![alt *em*]` gives `alt em` in both).
- HTML comes as `HtmlBlock` and `HtmlInline`, one `Html` node today:
  two arms into the same logic, including `push_drawn_text`
  (`src/render.rs:751`).
- Tables mark the header row with `TableRow(true)` instead of position,
  and alignment is `TableAlignment`.

Same in both, by probe: entities and backslash escapes, CRLF as a soft
break, tabs kept, ordered start, closing `#` of ATX headings, setext H1,
mismatched delimiter rows (no table), `~x~` and `~~y~~`, inline and block
HTML, autolinks.

### Output changes

1. **Reference links and images.** Today `[text][r]` with `[r]: /u`
   renders as plain `text`, and `![x][r]` disappears. comrak resolves
   both, so they gain the link underline and URL suffix, and the image
   shows.
2. **Footnote definitions** move to the end of the document in comrak;
   today they render in source order. A definition inside a list item
   (`tests/rendering.rs:2211`) was not checked.
3. **Cells past the delimiter row.** `| a | b |`, `|-|-|`,
   `| 1 | 2 | 3 |` is a three-column table today
   (`tests/rendering.rs:1094`); comrak drops the third cell, as GFM
   specifies.
4. **Task markers with no text.** `- [x]` and `- [ ]` stay plain items
   today (`tests/rendering.rs:670`); comrak makes them `TaskItem`s with
   no children, so patine has to apply the rule itself to keep it.
5. **TOML front matter.** comrak takes one delimiter. With `---`, a
   `+++` block becomes paragraphs (`tests/rendering.rs:2310`). Keeping
   it needs a split of `+++` blocks before parsing, or a second parse.
6. **Image alt** stays the same only if the flattening matches mdast's
   `alt` string.

### Decisions on the output changes (Jakob, 2026-10-31, in the chat)

1. Reference links and images render as inline links and images do:
   the link text underlined with the URL suffix, the image as
   `[image: alt] (url)`. The definition lines stay invisible and nothing
   moves; only text written in reference syntax changes. Rejected:
   rebuilding today's plain text.
2. Footnote definitions go to the end of the document, as comrak and
   GitHub place them. Rejected: keeping source order.
3. Cells past the delimiter row are dropped, as GFM specifies. Rejected:
   rebuilding the extra columns.
4. A task marker with no text is an empty task item, as comrak and
   GitHub render it; `tests/rendering.rs:670` changes with it. Rejected
   (the lead's recommendation): keeping it a plain item.
5. TOML front matter stays: patine splits a leading `+++` block off
   before parsing and renders it as today. Rejected: YAML only.
6. Image alt text (the lead's call): flatten the children so the output
   stays as today.

Before the switch, besides the characterisation tests below, a batch of
whole-document tests: varied example documents, each with its input and
its rendered output pinned, run across the rendering variants (options,
line endings, widths that force wrapping and overflow) (Jakob,
2026-10-31, in the chat).

### comrak facts (2026-10-31)

- 0.59.0, released 2026-10-26, 16 open issues. Licence BSD-2-Clause
  (crate metadata and `COPYING`).
- Its `rust-version` is 1.89; a build on 1.88 fails in its dependency
  `memchr-n` 0.1.10. patine's `rust-version` moves to 1.89, and the
  comment on it in `Cargo.toml` changes with it.
- `default-features = false` is needed: the default features pull in
  syntect with Oniguruma (C code). Without them the tree is pure Rust,
  21 crates, about 16 new to patine's lockfile (`caseless`, `entities`,
  `fearless_simd`, `finl_unicode`, `memchr-n`, the `phf` family,
  `rustc-hash`, `siphasher`, `tinyvec`, `typed-arena`,
  `unicode-normalization`), licensed MIT, Apache-2.0, Zlib or the
  Unicode licences.
- Panic reports in its tracker are closed and mostly in the HTML and
  CommonMark formatters, which patine does not use. The arena panics if
  the tree is changed while it is walked, so patine's walk must not
  change it.
- The inputs listed above all parse with comrak without a panic. A
  random run of 200,000 documents from 50 fragments gave 0 comrak panics
  and 38 `markdown` 1.0.0 panics.

### Tests patine lacks

patine has 384 passing tests (57 unit, 10 `ansi`, 26 `lines`, 278
`rendering`, 13 doc) and 98 insta snapshots. Nothing pins: reference
links and images; footnote definitions at top level and their order;
escaped pipes and short rows in tables; bare and uppercase task markers;
front matter followed directly by text, or YAML containing `+++`;
backslash escapes other than the hard break; entities; setext H2;
indented code and tilde fences; tabs; image alt with styles and link
titles; email autolinks; the `)` list delimiter; deep mixed nesting with
code; soft breaks after a link or inline code and inside a table cell.

Characterisation tests to add before the switch, as snapshots of
today's output (23; groups 1-4 are expected to change on purpose, group
5 may change, groups 6-9 must not):

1. Reference links: full, collapsed, shortcut and undefined; reference
   images, defined and undefined (2 tests).
2. Footnotes: a definition between two paragraphs with its reference in
   the second; one footnote referenced twice and defined at the end,
   plus a reference without a definition (2).
3. Tables: a cell past the delimiter row, a short row, header and
   delimiter with different counts, an escaped pipe (4).
4. Task markers: `- [x]`, `- [ ]`, `- [ ] `; `[X]` next to `[x]` (2).
5. Front matter directly followed by text; YAML whose body has a `+++`
   line (2).
6. Escapes `\*`, `\_`, `\\`, `\#`, `\[`; entities `&amp;`, `&nbsp;`,
   `&#x41;`, `&copy;` (2).
7. Setext H2 with a paragraph after it; top-level indented code; a
   tilde fence containing backticks; tabs in text, a list item and code
   (4).
8. Image alt with emphasis and inline code; a link title; `<a@b.c>` (3).
9. `1)` delimiters numbered all ones; a three-level mixed list with a
   fenced code block in the deepest item (2).

After the switch: the inputs above as regression tests, and a generated
test that random Markdown fragments never panic.

### Not checked yet

Rendered output after a port (only tree shapes were compared); footnotes
inside list items; lone `\r` in comrak; how comrak splits the info string;
whether glued runs such as `foo**bar**` split where markdown-rs splits
them (`tests/rendering.rs:3149-3290` depend on it); every default of
comrak's options; performance on large inputs; the 24 panic messages
found in asqr, of which only the shortest inputs ran through comrak.
