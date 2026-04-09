# Effective width can reach zero

`effective_width()` in `src/render.rs` chains three `saturating_sub` calls
(extra_indent, blockquote_depth, continuation_indent). With deep enough
nesting, the result can be 0, which causes `write_word` to wrap every single
word onto its own line.

Fix: add `.max(1)` at the end of `effective_width()` to guarantee at least one
column is available.

**File:** `src/render.rs` — `effective_width`
