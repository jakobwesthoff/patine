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

## Decisions

2026-11-01, the owner, through asqr (session 01M6BEQJJ8VYYK8J4N0PG1RE3M),
after seeing four rendered variants (logbook `plans/patine-alerts-design.md`
and `plans/patine-alerts/`):

- Look: a heavy bar `┃` in the kind's colour and a bold title in that
  colour (Note blue, Tip green, Important magenta, Warning yellow,
  Caution red). A symbol before the title is welcome, but the first set
  (ⓘ ✦ ❢ △ ⊘) looked uneven in the owner's terminal; the symbols are
  still open.
- A custom title (`> [!WARNING] Back up first`) replaces the kind's name
  and is rendered as inline Markdown.
- An unknown marker (`> [!FOO]`) becomes a neutral callout titled `Foo`.
- An empty alert shows its title line alone.
- Alerts render as callouts wherever comrak finds them, nested ones
  included.
- No blank line between the title and the body (the owner asked to see
  both side by side before this is final).
- `RenderOptions` gets a setting for the callout colours, as
  `QuoteStyle` is for quotes.

## Still open

- The symbols, and the blank-line question after the side-by-side view.
