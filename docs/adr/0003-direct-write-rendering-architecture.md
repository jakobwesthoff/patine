# 3. Direct-write rendering architecture

Date: 2026-04-09

## Status

Accepted

## Context

The renderer needs to walk the markdown AST (produced by markdown-rs) and
emit styled terminal output. Three architectural approaches were considered:

- **Trait per element** — Each AST node type gets a wrapper struct that
  implements a `Render` trait. Polymorphic dispatch handles rendering.
- **Intermediate representation** — The AST walker produces a tree of
  `StyledSpan` / `Block` values, which a separate writer serializes to
  terminal output. Enables testing the IR in isolation and swapping output
  backends.
- **Direct write with match dispatch** — A `Renderer` struct holds a
  `&mut impl Write` and a style/state context. A single `render_node`
  method matches on `Node` variants and delegates to dedicated `render_*`
  methods.

The trait approach was rejected because the AST node types are variants of
`markdown::mdast::Node`, an enum we do not control. Implementing a trait
would require wrapper types around each variant — boilerplate for no real
polymorphism gain, since we always match on the enum regardless.

The IR approach was rejected as premature: we have exactly one output
backend (terminal) and no concrete need for a second. The IR would add
allocation and an abstraction layer without earning its keep. It can be
extracted later if a second backend materializes.

## Decision

The renderer uses **direct write to `impl Write`** with a match-based
dispatch. A `Renderer` struct tracks state (column position, style stack,
spacing) and provides one `render_*` method per AST node type. Adding
support for a new element means adding one match arm and one method.

Testability is achieved by writing to a `Vec<u8>` in tests and comparing
the output via insta snapshots, with ANSI escape sequences converted to
human-readable tags.

## Consequences

- All rendering logic lives in `render.rs` with a flat set of methods.
  No wrapper types, no vtable dispatch, no separate IR serialization pass.
- The style stack (`push_style` / `pop_style`) handles re-applying
  formatting after line wraps, keeping inline style management centralized.
- If a second output backend is ever needed (e.g., plain text without
  ANSI, or HTML), the `render_*` methods would need to be refactored
  behind a trait or an IR would need to be extracted at that point.
- Testing couples to the exact ANSI byte output. The tag-conversion helper
  in tests mitigates readability concerns but means snapshot updates are
  needed when crossterm changes its escape code mappings.
