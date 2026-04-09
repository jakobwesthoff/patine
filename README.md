# patine

> *Look how the floor of heaven*
> *Is thick inlaid with patines of bright gold*
>
> — William Shakespeare, *The Merchant of Venice*, Act V, Scene I

A terminal Markdown renderer written in Rust. Reads Markdown from files or
stdin and renders it with formatting directly in your terminal.

```bash
patine README.md
cat doc.md | patine
```

## Why not glow?

[glow](https://github.com/charmbracelet/glow) is an excellent and mature
terminal Markdown renderer. patine exists because we wanted something
different:

- **Less color, more structure.** glow uses colored backgrounds for headings,
  syntax highlighting for code blocks, and colored text throughout. patine
  stays close to the terminal's default colors and relies on bold, italic,
  underline, and dim to convey structure. The result is quieter and — in our
  opinion — easier to read. This comes down to personal taste.
- **Focused scope.** glow includes a TUI file browser, stash functionality,
  and Glamour-based theming. patine is a single-purpose tool: pipe Markdown
  in, get styled text out. Nothing more.

## Features

- **Headings** — H1 italic + underlined, H2–H6 bold
- **Inline formatting** — bold, italic, bold-italic, strikethrough, inline code
- **Code blocks** — verbatim output in a distinct color, never wrapped
- **Links** — text followed by the URL in dimmed parentheses
- **Images** — `[image: alt text]` with the path/URL dimmed
- **Lists** — ordered and unordered, with arbitrary nesting
- **Tables** — Unicode box-drawing borders, centered bold headers, row separators
- **Blockquotes** — `│` bar prefix, dimmed italic text, nested blockquote support
- **Horizontal rules** — `─` spanning the terminal width
- **Word wrapping** — at terminal width, not a hardcoded column count

## Installation

### From source

```bash
cargo install --path .
```

### Build from repository

```bash
git clone https://github.com/jakobwesthoff/patine.git
cd patine
cargo build --release
```

The binary will be at `target/release/patine`.

## Usage

```bash
# Render a file
patine document.md

# Pipe from stdin
cat document.md | patine
curl -s https://example.com/README.md | patine

# Show help
patine --help
```

When no file argument is given and stdin is a terminal (not a pipe), patine
prints its help message.

## Why "patine"?

Shakespeare used *patine* to describe golden discs inlaid into the floor of
heaven — raw material transformed into something luminous. The word carries the
same idea as *patina*: a surface that becomes more beautiful through
transformation.

That is what `patine` does. It takes raw Markdown and transforms it into
something rich and readable, right in your terminal. And if you look closely,
you will find **Rust** has left its mark on the word itself — a quiet nod to the
language that forged it.

## License

This project is licensed under the [Mozilla Public License 2.0](LICENSE).
