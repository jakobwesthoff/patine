# Minor code cleanups

Small improvements that don't affect behavior:

- `render.rs` — `render_thematic_break`: the `rule` variable could be inlined
  into the `write!` call.
- `render.rs` — `render_list_item`: `"• ".to_string()` allocates
  unnecessarily. Both bullet and number branches could return `Cow<str>`.
- `table.rs` — `extract_text` clones `t.value` for every text node. For
  measuring column widths only `&str` is needed; could avoid the allocation
  with a `write!`-to-String accumulator pattern.
- `main.rs` — `Cli::parse_from(["patine", "--help"])` followed by
  `unreachable!()` is a workaround. Clap has `Cli::command().print_help()`
  which is more direct.

**Files:** `src/render.rs`, `src/table.rs`, `src/main.rs`
