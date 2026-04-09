# Document the column tracking invariant

The `column` field in `Renderer` tracks content position excluding the nesting
indent. `effective_width()` also excludes the indent. This implicit contract
makes the wrapping math work, but it's undocumented and fragile.

Add a doc comment on the `column` field explicitly stating: "Tracks content
column position after the indent prefix. Wrapping decisions compare `column`
against `effective_width()`, both of which exclude indent width."

Also applies to `render_code_block` where `self.column` is set to the line's
display width without accounting for the indent that was just written — correct
by the invariant, but confusing without documentation.

**File:** `src/render.rs` — `column` field, `render_code_block`
