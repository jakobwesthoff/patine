# Meeting notes, week 42

Attendees: Ada, Grace, Linus. Notes by Ada.

## Decisions

1. Ship the parser change behind a flag first.
2. Move the nightly build to **03:00** UTC.
3. Drop support for the `legacy` backend after the next release.

## Numbers

| Metric            | Last week | This week | Change |
|:------------------|----------:|----------:|:------:|
| Build time (s)    |       412 |       388 |   -6%  |
| Test failures     |        17 |         9 |  -47%  |
| Open bugs         |        63 |        58 |   -8%  |
| Median review (h) |       5.5 |       4.0 |  -27%  |

## Snippets we looked at

The slow path:

```rust
fn lookup(map: &HashMap<String, Vec<u32>>, key: &str) -> usize {
    map.get(key).map(|ids| ids.len()).unwrap_or_default()
}
```

And the shell one-liner that finds the culprit:

```bash
git log --since=1.week --format='%h %s' -- src/parser | head -20
```

## Open questions

- Who owns the flaky `integration::upload` test?
- Is the `~~old~~` benchmark data still valid?
- Do we need a *second* staging cluster?

## Next week

Review the draft at https://example.com/draft and add comments by Friday.
