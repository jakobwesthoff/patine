# 2. Use markdown-rs for Markdown parsing

Date: 2026-04-09

## Status

Accepted

## Context

Patine needs to parse Markdown (including GFM extensions like tables) and walk
the resulting AST to produce styled terminal output. Two mature Rust crates were
evaluated:

- **comrak** — arena-allocated AST wrapped in `RefCell`, built-in `Traverse`
  iterator with `Enter`/`Leave` events, GFM support via extension flags. Stable
  since 2017, ~4.2M total downloads.
- **markdown-rs** (`markdown` crate) — returns an owned `mdast::Node` enum tree,
  traversed via plain recursive pattern matching. GFM support via
  `Options::gfm()`. Reached 1.0 in April 2025, ~5.9M total downloads.

Both support the GFM table extension we need (`Table`, `TableRow`, `TableCell`
nodes with alignment metadata).

## Decision

We will use **markdown-rs** (`markdown` crate) for Markdown parsing.

Key reasons:

1. **Simpler API** — An owned enum tree with recursive `match` is more
   idiomatic and easier to reason about than comrak's arena + `RefCell`
   approach, which introduces runtime borrow panics if scopes are mishandled.
2. **No lifetime gymnastics** — comrak requires an `Arena` that must outlive all
   nodes. markdown-rs returns a self-contained `Node` tree with no lifetime
   constraints.
3. **Sufficient for our use case** — comrak's `Traverse` iterator (flat
   `Enter`/`Leave` loop with a style stack) is a nice pattern, but the same
   effect is trivially achieved with recursive rendering in markdown-rs. The
   extra machinery comrak provides does not justify the added complexity.
4. **GFM table metadata** — comrak exposes slightly richer table metadata
   (`num_columns`, `num_rows` in a single struct), but markdown-rs provides
   alignment info per cell and the row/column counts are easily derived from
   the child node structure.
5. **Stable 1.0 release** — The crate reached 1.0 and offers a stable API
   surface.

## Consequences

- Markdown parsing uses `markdown::to_mdast(src, &markdown::Options::gfm())`.
- AST traversal is implemented as recursive functions matching on `mdast::Node`
  variants.
- Deeply nested input should be capped (~500 KB) to avoid stack issues, as
  markdown-rs does not have built-in depth limiting.
- If we later need comrak-specific features (e.g., rendering back to
  CommonMark, or its built-in HTML sanitization), migration would require
  rewriting the AST walker but not the rendering logic.
