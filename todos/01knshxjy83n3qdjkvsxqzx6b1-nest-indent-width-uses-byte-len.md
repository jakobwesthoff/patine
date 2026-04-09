# NEST_INDENT_WIDTH uses byte length instead of display width

`NEST_INDENT_WIDTH` is defined as `NEST_INDENT.len()`, which returns byte
count, not display width. Currently safe because the indent is ASCII spaces,
but if someone changes it to a non-ASCII character (e.g., tab or Unicode
space), width calculations would silently break.

Should use `UnicodeWidthStr::width(NEST_INDENT)` for consistency with the rest
of the codebase. However, this can't be `const` — would need to be a function
or a lazy static.

**File:** `src/render.rs` — `NEST_INDENT_WIDTH`
