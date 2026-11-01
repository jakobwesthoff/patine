---
title: "Render GitHub alerts as callouts"
kind: feature
component: render
status: needs-discussion
origin: request
---
# Render GitHub alerts as callouts

GitHub's alert syntax marks a block quote as a callout with its first
line: `> [!NOTE]`, `> [!TIP]`, `> [!IMPORTANT]`, `> [!WARNING]` and
`> [!CAUTION]`. GitHub, and ntropy's own site export, draw these as boxes
with a title. patine draws them as plain block quotes with the marker as
literal text, so `ntropy show` (ntropy 2.5.0, which renders through
patine) shows `│ [!NOTE] A note callout …` for every callout in a note.

## Problem

Seen on 2026-11-01 in the capture of `ntropy show` on ntropy's
kitchen-sink fixture: each of the five alert kinds renders as a quote
bar followed by `[!NOTE]`, `[!TIP]` and so on. Whether comrak 0.59.0 can
parse alerts itself (an extension option) is not checked yet.

## Open questions for the owner

- How a callout looks in the terminal: for example a coloured bar and a
  bold title line (`Note`, `Tip`, …) instead of the marker, one colour per
  kind, or an icon. A rendered comparison should be shown first.
- Whether an unknown marker (`> [!FOO]`) stays a plain quote with the
  text as written, as on GitHub.
