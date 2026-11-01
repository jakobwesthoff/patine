---
title: "A character reference puts a line ending into a link URL span"
kind: bug
component: render
origin: review
---
# A character reference puts a line ending into a link URL span

`src/styled.rs` (around lines 258-260 on db6d420) promises that span text
never holds a line ending, so a front end can draw each line as it comes.
A link destination with a decoded `&#13;` or `&#10;` breaks that: the dim
URL suffix span of `[t](http://x/a&#13;b&#10;c)` holds a CR and an LF.

## Problem

Found on 2026-11-01 by the plan review of the CLI control-character fix
(patine todo 01M68XJBM8). Repro:
`printf '[t](http://x/a&#13;b&#10;c)\n' | patine | od -c` shows `\r` and
`\n` inside the suffix. A front end that trusts the promise (the
library's own ANSI writer, ntropy `show`) writes them to the terminal,
which moves the cursor and breaks the layout.

## Suggested fix

Keep the promise in the library: when the URL suffix is built, a line
ending in the destination becomes something that holds no line ending
(for example its percent-encoding, which is what the link means). Check
link text, titles, image alt and every other place a decoded reference
can reach a span the same way, and pin the promise with a test that
walks every span of a document full of decoded controls.
