# 5. Syntax highlighting via syntect with an ANSI-palette theme

Date: 2026-04-17

## Status

Accepted

## Context

Fenced code blocks have until now been rendered in a single distinct
foreground color (`DarkYellow`), regardless of language. The language
hint from the fence (e.g. `` ```rust ``) was preserved internally but
unused, with the intent of enabling syntax highlighting at a later
date (see ADR 0003 and `docs/output-style.md`). This ADR records the
decisions for that feature.

Four axes needed resolution: *library choice*, *color model*,
*language coverage*, and *user-facing surface (configuration,
language label, auto-detection)*. Each interacts with patine's
stated design ethos: less color, more structure; focused scope; no
config files.

### Library choice

Two Rust ecosystems for syntax highlighting were evaluated:

- **[syntect](https://docs.rs/syntect/)** — regex-based, driven by
  Sublime Text `.sublime-syntax` and TextMate `.tmTheme` files. The
  de-facto standard for Rust CLI tools (used by `bat`, `zola`,
  `delta`, many others). Highlighting quality for mainstream languages
  (Rust, C, Python, JS/TS) is consistently strong. Stable API; slow
  first-use theme/syntax loading, acceptable once cached.
- **[tree-sitter-highlight](https://crates.io/crates/tree-sitter-highlight)**
  — AST-driven via per-language `tree-sitter-<lang>` grammar crates.
  More accurate parsing in theory, but the Rust story has known rough
  edges: highlight-group names are not standardized across grammars,
  minor version bumps silently change highlight output, each language
  adds a separate crate and compile cost, and highlighting quality
  for common languages (Rust, C) is often *worse* than syntect because
  the grammars were authored for editors with their own post-processing.
  Other projects (e.g., Zola) have repeatedly evaluated the migration
  and remained on syntect.

Tree-sitter's advantages (incremental reparsing, language injection,
query-based analysis) are not needed for a one-shot terminal renderer.

To broaden syntect's language coverage without hand-curating grammars,
**[two-face](https://github.com/CosmicHorrorDev/two-face)** was
considered. It repackages the bat project's curated grammar and theme
collection as a standalone syntect extension: 100+ syntaxes (TOML,
TypeScript, Dockerfile, Nix, GraphQL, Svelte, WGSL, Typst, …) and
30+ themes, shipped as precompiled `.packdump` assets. Linker dead-code
elimination keeps binary overhead proportional to what is actually
used (~0.6 MiB of syntax data over syntect alone). The crate is
actively maintained (latest release December 2025).

Critically, two-face ships the bat-authored `ansi`, `base16`, and
`base16-256` themes, which are required for the color-model decision
below.

### Color model — respecting the user's terminal palette

Patine's existing aesthetic uses the terminal's default colors and
relies on bold/italic/underline/dim for structure (see
`docs/output-style.md`). Conventional syntax highlighting themes
define *absolute* RGB truecolor values, which override the user's
terminal theme and clash with this principle.

A convention established by bat (and adopted by syntect renderers
downstream) solves this: encode an ANSI palette index (0–15) in a
theme's color values such that renderers can emit terminal SGR
codes like `CSI 38;5;<n> m` instead of truecolor escapes. The
encoding lives in the `alpha` channel of syntect's `Color` struct,
not in RGB proper:

- `color.a == 0x00` — ANSI-indexed color, with `color.r` carrying the
  palette index (`0x00`–`0x07` for standard ANSI, `0x08`+ for the
  extended 256-color range).
- `color.a == 0x01` — use the terminal's default foreground.
- Otherwise — truecolor RGB, interpreted from `r`/`g`/`b`.

(Reference: bat's `src/terminal.rs`, which performs the decode and
emits ANSI SGR on this basis.) When a renderer detects these
sentinel values, it emits an ANSI-indexed color. When the user's
terminal has a custom palette configured (e.g., Solarized, Gruvbox,
any base16 shell scheme), those colors flow through automatically
with no additional configuration on patine's part.

Three such themes exist in the bat/two-face collection:

- **`ansi`** — uses only ANSI palette indices 0–7 (standard colors).
  Every terminal has these slots configured. Lowest color count,
  quietest output, maximum portability.
- **`base16`** — uses indices 0–15 (standard + bright). More color
  variety, but bright-color rendering varies across terminals (some
  render them as lighter shades, some as bolded normals, some as
  semantically-distinct extra colors). Depends on the user having
  configured all 16 slots sensibly.
- **`base16-256`** — targets 256-color terminals with a specific
  base16-shell layout. Too prescriptive for general use.

### Language coverage, configuration surface, and auto-detection

Patine's README explicitly rejects config files and TUI features.
Three orthogonal questions follow from that stance:

- **Configuration** — should the theme be user-selectable?
- **Language label** — should the fence's language hint be displayed?
- **Auto-detection** — if a fence has no language hint, should
  patine guess?

Auto-detection of raw code snippets is not a solved problem in Rust.
`hyperpolyglot` (Linguist-in-Rust) leans on filenames and
extensions, which are absent for a pasted fence. syntect's
`find_syntax_by_first_line` catches shebangs and a few modelines but
has a low hit rate. Full content-based classifiers
(`guesslang`-style ML models) have no production-quality Rust port.

## Decision

### Library

Use **`two-face`** as the dependency entry point, configured with
the `syntect-default-fancy` feature (pure-Rust `fancy-regex` engine)
and with default features disabled. This transitively pulls in
syntect and gives us the curated bat grammar set plus the
bat-authored `Ansi` theme in one integration. The `fancy-regex`
choice avoids the C dependency that the default `syntect-onig`
feature would introduce (`oniguruma`), keeping the build pure-Rust
and simplifying cross-compilation. Bind to syntect's public types
where needed (`HighlightLines`, `Style`, `Color`) — two-face
explicitly tracks syntect versions and is considered compatible for
direct use alongside it.

### Color model

Render with the bat-authored **`Ansi`** theme from `two-face`'s
`EmbeddedThemeName` set (8-color palette, indices 0–7 only).
Implement a small adapter that inspects each styled span's
syntect `Color`:

- `color.a == 0x00` → emit
  `SetForegroundColor(Color::AnsiValue(color.r))` via crossterm.
- `color.a == 0x01` → emit nothing (terminal default foreground,
  restore whatever the outer context had).
- Any other value (would indicate truecolor RGB) → treat as the
  terminal default and log nothing. The `Ansi` theme does not
  produce such values; this branch exists only as a safe fallback
  in case a future theme swap emits unexpected data.

Rationale for `ansi` over `base16`: patine's design ethos favors
restraint; 8 colors are more portable (every terminal has those slots
configured sensibly) and produce quieter output. `base16`'s bright
variants render inconsistently across terminal configurations.

### Language coverage

Accept whatever set two-face + syntect ship out of the box. No
hand-curated allow-list — linker dead-code elimination naturally
trims unused grammar data. Languages we test explicitly in the
snapshot suite: **rust, python, javascript, typescript, go, bash,
json, yaml, toml, markdown**. Any other language recognized by the
combined syntax set works without special-casing.

### Configuration, language label, auto-detection

- **No configuration.** The theme is hardcoded. Consistent with
  patine's stated "no config files, no themes" scope.
- **No language label displayed.** Fences with a language hint
  render identically to fences without, apart from the highlighting
  itself. Less visual decoration; the highlighting itself signals
  the language to readers familiar with it.
- **No auto-detection.** When a fence lacks a language hint, or
  specifies a language we cannot recognize, fall back to the current
  flat `CODE_COLOR` rendering. Honest about our knowledge, avoids
  heuristic guesses that would sometimes be wrong.

## Consequences

- **Dependency footprint.** `two-face` pulls in syntect, `onig` (or
  `fancy-regex`, depending on feature flag), and embedded asset
  `.packdump` files. Binary size grows by roughly 1–2 MiB.
- **Terminal compatibility.** Output uses ANSI 8-color escapes
  (`CSI 3<n> m` or `CSI 38;5;<n> m`), supported universally. No
  truecolor requirement. Users with custom terminal palettes see
  highlighting in their configured theme.
- **Piped output.** Redirecting patine to a file or another program
  will include ANSI escapes. This matches the existing behavior for
  `CODE_COLOR`, bold, etc. — no regression.
- **Fallback behavior.** Unknown/missing language → flat
  `CODE_COLOR`. Users can always force the fallback by omitting the
  fence language hint or specifying a nonsense one.
- **Unsupported SGR attributes.** Syntect themes can request bold
  and italic alongside colors. The renderer will honor these via
  patine's existing style stack — `ansi` theme uses them sparingly,
  matching the project's typographic palette.
- **Future flexibility.** If we ever want to broaden to 16 colors,
  switching to the `base16` theme is a one-line change. If we ever
  want to expose a `--theme` flag, the renderer is already
  theme-driven — only the CLI surface would need to change. Neither
  is planned.
- **Test coupling.** Snapshot tests will capture exact ANSI-indexed
  color escapes per language, coupling them to the `ansi` theme's
  scope-to-color mapping. Upstream theme changes would require
  snapshot review — acceptable given two-face's slow release cadence
  and the reviewability of snapshot diffs.
