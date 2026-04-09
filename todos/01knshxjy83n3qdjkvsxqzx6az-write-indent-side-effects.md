# write_indent has side effects when writing nothing

`write_indent` in `src/render.rs` sets `at_start = false` and
`consecutive_newlines = 0` even when `extra_indent == 0` and
`blockquote_depth == 0`, meaning it writes zero bytes but still mutates
output-tracking state.

This is currently harmless because `ensure_block_spacing` always runs before
`write_indent`, but the semantics are misleading. A function called
"write indent" that writes nothing shouldn't change tracking state.

Consider guarding the state mutations behind a check that something was
actually written.

**File:** `src/render.rs` — `write_indent`
