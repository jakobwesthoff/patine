---
title: "Rendering panics on a footnote referenced only from an unreferenced footnote"
kind: bug
component: render
impact: critical
origin: review
---
# Rendering panics on a footnote referenced only from an unreferenced footnote

patine panics while rendering a document in which a footnote is referenced
only from inside another footnote definition that nothing in the body
references. Every front end on patine crashes on such input: the `patine`
CLI, `ntropy show` and `render --to ansi`, and asqr wherever it renders
Markdown. Found on 2026-11-01 by an agent working on ntropy's HTML
footnotes; reproduced the same day with patine 2.2.1:

```
$ printf 'text\n\n[^z]: sees[^b]\n\n[^b]: B\n\nlater[^c]\n\n[^c]: C\n' | patine -w 60
thread 'main' panicked at .../patine-2.2.1/src/render.rs:408:26:
a definition with references has a reference node
```

## Problem

`render_node` for `NodeValue::FootnoteDefinition` (`src/render.rs`, around
line 401) numbers a definition when comrak counts references to it
(`total_references > 0`) and expects its number in `footnote_numbers`.
`footnote_numbers` (from commit 52ae24f, "Number footnote references and
their notes") does not give a number to `b`, whose only reference sits in
the unreferenced definition `z`, so the `expect` fails.

## Expected

No input makes patine panic. A footnote whose only references are in
definitions that are not drawn is treated like an unreferenced one, the
same rule ntropy's HTML output follows (only footnotes reached from the
body are drawn); the `expect` becomes a case the code handles.
