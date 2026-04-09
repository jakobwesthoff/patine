# Table rendering ignores Renderer state

`render_table` in `src/render.rs` delegates to `table::render_table` which
writes directly to the underlying writer, bypassing the Renderer's state
tracking entirely.

Problems:
- **Blockquote bars missing:** The indent string is built from `extra_indent`
  only, not from `blockquote_depth`. Tables inside blockquotes won't have `│`
  bar prefixes on their lines.
- **No width constraint:** The table module doesn't know `content_width`. A
  wide table overflows without truncation or warning.

**Files:** `src/render.rs` — `render_table`, `src/table.rs`
