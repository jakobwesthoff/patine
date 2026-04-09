# URL handling in links and images

`render_link` and `render_image` in `src/render.rs` write URLs directly via
`write!`, bypassing `write_word`. This is actually correct for the URL itself —
URLs must never be broken across lines (they become unselectable/uncopyable).

However, the current code doesn't handle the case where the URL pushes the
line past the terminal width. The URL should be moved to a new line if it
doesn't fit on the current one, but never broken mid-URL.

Additionally, both methods have near-identical URL-in-parens logic (push Dim,
write `(`, write URL raw, write `)`, pop Dim). This should be extracted into a
shared helper like `write_dimmed_url_suffix`.

**Files:** `src/render.rs` — `render_link` and `render_image`
