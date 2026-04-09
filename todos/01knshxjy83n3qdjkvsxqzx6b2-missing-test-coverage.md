# Missing test coverage for edge cases

The following scenarios have no test coverage:

- Table inside a blockquote (would expose the table/renderer state issue)
- Very small terminal widths (e.g., `width = 5` or `width = 0`)
- Hard breaks (`\` at end of line or two trailing spaces)
- Multi-paragraph list items (paragraph + paragraph inside one `<li>`)
- Links/images with very long URLs that exceed terminal width
- Empty code blocks (``` ``` ```)

**File:** `tests/rendering.rs`
