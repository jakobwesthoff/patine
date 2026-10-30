# 7. Render options for soft breaks and HTML

Date: 2026-10-14

## Status

Accepted

Amends [6. Render to style-neutral lines](0006-render-to-style-neutral-lines.md)

Amended by [8. Glued inline elements wrap as one word](0008-glued-inline-elements-wrap-as-one-word.md)

Amended by [10. Quote style option](0010-quote-style-option.md)

Amended by [15. First line width option](0015-first-line-width-option.md)

## Context

asqr renders the intro, the question texts and the option descriptions
of its sessions with `render_lines`. Agents write these fields as JSON
strings, and two parts of patine's Markdown rendering do not fit them:

- A single newline in running text is a soft line ending. CommonMark
  joins the lines into one paragraph, and patine draws it as a space. In
  a JSON string an agent writes `\n` to break the line, as a GitHub
  comment would show it.
- Raw HTML disappears. This was never decided: `Node::Html` reached the
  fallback arm of `render_node`, which renders the children of a node
  and skips leaves. `Use a Vec<String> here` rendered as `Use a Vec
  here`, and an HTML block rendered nothing.

asqr could rewrite the Markdown before handing it to patine, but it
would parse every text twice and edit the source by position. The user
decided to solve both in patine, as options. Markdown documents, asqr's
Markdown entries and patine's command-line interface among them, keep
the 1.5.0 rendering.

What markdown-rs 1.0 produces for these inputs:

- A soft line ending stays inside the `Text` value, as `\n`, `\r\n` or a
  lone `\r`, whichever the source used. The spaces around it are already
  removed. Inline code and image alt text keep it in their values too.
  A setext heading over two lines holds both lines in one `Text`.
- Inline and block HTML are both `Node::Html`. Inline HTML is one node
  per tag, so `Vec<String>` is `Text "Vec"` followed by
  `Html "<String>"`. A block is one node whose value holds all its lines.
  An HTML comment block ends with the line that closes the comment, so
  `<!-- a --> trailing` is a single node.
- Autolinks such as `<https://example.com>` are `Link` nodes.
- A table row is one source line, so a table cell cannot hold a line
  ending. It can hold inline HTML.

## Decision

### API

```rust
#[non_exhaustive]
pub struct RenderOptions { pub soft_breaks: SoftBreaks, pub html: Html }

#[non_exhaustive]
pub enum SoftBreaks { Join, Break }   // default Join

#[non_exhaustive]
pub enum Html { Drop, Literal }       // default Drop

pub fn render_lines_with(input: &str, width: usize, options: &RenderOptions) -> Vec<StyledLine>;
pub fn render_with(input: &str, writer: &mut impl Write, terminal_width: usize,
                   options: &RenderOptions) -> Result<()>;
```

`render_lines` and `render` call these with the default options. The
command-line interface has no flags for them.

`RenderOptions` derives `Clone`, `Copy`, `Debug`, `Default`, `PartialEq`,
`Eq` and `Hash`. It is `#[non_exhaustive]` so that further options can
come in minor releases, which keeps code outside patine from writing it
as a struct literal. Such code starts from `RenderOptions::new()` or
`RenderOptions::default()`, both the defaults, and either sets the
public fields, as with `TextStyle`, or chains `with_soft_breaks` and
`with_html`. These builders take and return the options by value. They
are `const fn`, like `new`, so a set of options can be a constant:

```rust
const FIELD_OPTIONS: RenderOptions = RenderOptions::new()
    .with_soft_breaks(SoftBreaks::Break)
    .with_html(Html::Literal);
```

The `with_` prefix keeps a builder from sharing its name with the field
it sets.

Both enums are `#[non_exhaustive]`, unlike `Underline` and `AnsiColor`
in ADR 0006. Front ends match on those two to map styles, so a new
variant has to be a breaking change. Callers build the option enums and
have no reason to match on them, and further variants are conceivable,
such as an HTML mode that interprets `<br>` or one that removes tags and
keeps their text.

`render_with` exists for symmetry with `render_lines_with`. It writes
the lines of `render_lines_with`, as `render` writes those of
`render_lines`.

### `SoftBreaks::Break`

A soft line ending in running text breaks the line the way a hard break
(`Node::Break`, a trailing backslash or two spaces) does: the next line
starts under the text above, past a list bullet and inside blockquote
bars. The renderer calls the same function for both.

Running text is every `Text` node: paragraphs, list items, blockquotes,
emphasis, link text and setext headings. Inline code and image alt text
are not `Text` nodes and keep drawing a line ending as a space. Code
blocks and frontmatter keep their source lines under both settings.

A line that held nothing but dropped HTML stays as an empty line, since
the line endings around it still break.

### `Html::Literal`

The renderer tells inline from block HTML by position: HTML among the
inline children of a paragraph or heading is inline, HTML anywhere else
is a block.

Inline HTML is running text. It takes the style of the text around it
(bold inside `**…**`, underlined inside a link, dim italic in a quote)
and wraps like words. A line ending inside a tag spanning several lines
follows `SoftBreaks` like one in text. In a table cell, inline HTML is
part of the cell text.

An HTML block keeps its source lines verbatim and never wraps, like a
code block. It gets block spacing, the nesting indent and blockquote
bars, but neither the code color nor the code block's extra indent:
HTML is part of what the author wrote, not code, and the indent would
make it look like a code block. It is drawn in the context style only,
so plain at the top level and dim italic in a blockquote.

HTML comments show as written, inline and as blocks. With `Literal`
meaning all HTML, no tag needs special handling; dropping only comments
would have to cut a comment block such as `<!-- a --> trailing` apart in
the source. In asqr the user then sees everything an agent sent.

Autolinks are `Link` nodes and render as links under both settings.

## Consequences

- With the default options, every snapshot from 1.5.0 renders
  unchanged. A test renders the showcase and documents with soft breaks
  and HTML in every position through `render_lines` and through
  `render_lines_with` with the defaults, and compares the lines.
- `Html::Drop` keeps the 1.5.0 rendering, as an explicit case of the
  renderer instead of a side effect of the fallback.
- Text and HTML glued together without a space, `Vec` and `<String>`,
  are separate nodes. In 1.6.0 they could wrap apart at the end of a
  line, as any two adjacent inline elements could. Since 1.6.1 they wrap
  as one word, see ADR 0008.
- Adding an option or a variant does not break code outside patine.
