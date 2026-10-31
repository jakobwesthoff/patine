---
title: "Replace the markdown parser with comrak"
kind: bug
component: render
impact: high
origin: upstream
---
# Replace the markdown parser with comrak

patine parses Markdown with the `markdown` crate 1.0.0 (wooorm/markdown-rs)
and builds its output from the mdast that `to_mdast` returns. That crate
panics on small inputs instead of returning a tree or an error, and it is
no longer maintained, so the panics will not be fixed upstream.

## Problem

Found on 2026-10-31 in asqr (asqr todo 01M67QJJZMZ5S055T5BJJG1MQZ, now
closed; asqr ADR 98 and 99), by a random generator over Markdown
fragments: 24 distinct panic messages in `markdown` 1.0.0. The shortest
inputs, all through `patine::render_lines_with`:

- `[a]:b\n=\n=+\r-`: "Cannot push to non-parent" (`to_mdast.rs:193`)
- `1. ~~~\n*` and `- ~~~\n1.`: "mismatched (non-jsx): ListOrdered /
  ListUnordered" (an unclosed fence in a list item, then an item of the
  other list kind)
- with the frontmatter option patine turns on: `---\n\n-` (index out of
  bounds) and `---\n\n[^a]:` (`Option::unwrap()` on `None`)

asqr now catches the panic and shows the text plain, but every other user
of patine, the `patine` command included, panics.

Upstream state, checked 2026-10-31: `markdown` 1.0.0 is the latest
release; the repository's last push was 2025-04-23; it has 97 open
issues, about a dozen of them panic reports with unmerged fix pull
requests (#271, #265, #263, #219, #213 and others).

## Decision (Jakob, 2026-10-31, in the chat)

Replace the parser with `comrak` rather than wait for upstream or keep a
fork: comrak 0.59.0 is maintained (last push 2026-10-26, 16 open
issues), returns an AST, and covers GitHub Flavored Markdown, footnotes
and frontmatter. Rejected: `pulldown-cmark` (an event stream, not a
tree, so a larger rewrite); filing the drafted bug report upstream (no
one maintains it); forking markdown-rs.

## Before the switch

A preliminary estimate of the effort and an inventory of the tests patine
lacks to make the switch safe follow below.
