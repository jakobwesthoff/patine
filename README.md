# patine

A simple, elegant Markdown renderer for the terminal — with opinions. Pipe Markdown in, get styled text out. No TUI, no themes, no config files.

> *Look how the floor of heaven*
> *Is thick inlaid with patines of bright gold*
>
> — William Shakespeare, *The Merchant of Venice*, Act V, Scene I

## Installation

```bash
cargo install patine
```

### Pre-built Binaries

Pre-built binaries are available on the [GitHub Releases](https://github.com/jakobwesthoff/patine/releases) page for macOS (Apple Silicon & Intel), Linux (x86_64 & aarch64), and Windows (x86_64).

## Quick Start

```bash
# Render a file
patine README.md

# Pipe from stdin
cat doc.md | patine

# Override output width
patine --width 100 README.md
```

## Why not glow?

[glow](https://github.com/charmbracelet/glow) is an excellent and mature terminal Markdown renderer. patine exists because I wanted something different:

- **Less color, more structure.** glow uses colored backgrounds for headings, syntax highlighting for code blocks, and colored text throughout. patine stays close to the terminal's default colors and relies on bold, italic, underline, and dim to convey structure. The result is quieter and — in my opinion — easier to read. This comes down to personal taste.
- **Focused scope.** glow includes a TUI file browser, stash functionality, and Glamour-based theming. patine is a single-purpose tool: pipe Markdown in, get styled text out. Nothing more.

## What It Renders

- **Headings** — H1 italic + underlined, H2–H6 bold
- **Inline formatting** — bold, italic, bold-italic, strikethrough, inline code
- **Code blocks** — verbatim output in a distinct color, never wrapped
- **Links** — underlined text, URL in dimmed parentheses
- **Images** — `[image: alt text]` with the path/URL dimmed
- **Lists** — ordered and unordered, with arbitrary nesting
- **Tables** — Unicode box-drawing borders, centered bold headers, row separators
- **Blockquotes** — `│` bar prefix, dimmed italic text, nested support
- **Horizontal rules** — `─` spanning the terminal width
- **Word wrapping** — at terminal width (overridable with `--width`)

## Usage

```
Usage: patine [OPTIONS] [FILE]

Arguments:
  [FILE]  Markdown file to render. Reads from stdin if omitted.

Options:
  -w, --width <WIDTH>  Override the output width in columns
  -h, --help           Print help
  -V, --version        Print version
```

When no file is given and stdin is a terminal, patine prints its help message.

## Why "patine"?

Shakespeare used *patine* to describe golden discs inlaid into the floor of heaven — raw material transformed into something luminous. The word carries the same idea as *patina*: a surface that becomes more beautiful through transformation.

That is what `patine` does. It takes raw Markdown and transforms it into something rich and readable, right in your terminal. And if you look closely, you will find **Rust** has left its mark on the word itself — a quiet nod to the language that forged it.

## Development

```bash
git clone https://github.com/jakobwesthoff/patine.git
cd patine
cargo build --release
cargo test
```

## License

This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

Copyright (c) 2026 Jakob Westhoff <jakob@westhoffswelt.de>
