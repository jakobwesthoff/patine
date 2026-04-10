// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use insta::assert_snapshot;

/// Render markdown to a string, replacing ANSI escape sequences with
/// human-readable tags so that snapshots are easy to review and diff.
///
/// Mapping (patine SGR codes):
///   \x1b[1m      → [bold]         \x1b[22m     → [/intensity]  (NormalIntensity, clears Bold+Dim)
///   \x1b[2m      → [dim]
///   \x1b[3m      → [italic]       \x1b[23m     → [/italic]
///   \x1b[4m      → [underline]    \x1b[24m     → [/underline]
///   \x1b[38;5;3m → [code]         \x1b[39m     → [/color]
fn render(markdown: &str) -> String {
    render_with_width(markdown, 80)
}

/// Like [`render`], but with a custom terminal width.
fn render_with_width(markdown: &str, width: usize) -> String {
    let mut buf: Vec<u8> = Vec::new();
    patine::render(markdown, &mut buf, width).expect("render should not fail");

    String::from_utf8(buf)
        .expect("output should be valid utf-8")
        // Order matters: longer sequences must be replaced before shorter
        // ones that share a prefix (e.g., \x1b[22m before \x1b[2m).
        .replace("\x1b[38;5;3m", "[code]")
        .replace("\x1b[39m", "[/color]")
        // NormalIntensity (SGR 22) is used to disable both Bold and Dim.
        // In the tag output it appears as [/intensity]. The context
        // determines whether it was closing bold or dim.
        .replace("\x1b[22m", "[/intensity]")
        .replace("\x1b[23m", "[/italic]")
        .replace("\x1b[24m", "[/underline]")
        .replace("\x1b[1m", "[bold]")
        .replace("\x1b[2m", "[dim]")
        .replace("\x1b[3m", "[italic]")
        // Order matters: `\x1b[4:2m` (double underline) must be replaced
        // before `\x1b[4m` (single underline) so the longer prefix wins.
        .replace("\x1b[4:2m", "[double-underline]")
        .replace("\x1b[4m", "[underline]")
        .replace("\x1b[9m", "[strike]")
        .replace("\x1b[29m", "[/strike]")
}

/// Strip the `[tag]` markers (e.g. `[bold]`, `[/underline]`) that `render`
/// substitutes for ANSI escapes. Used to measure visible display width
/// or to assert on the plain-text shape of a line. Only the exact set of
/// style tags emitted by `render` is removed — this preserves literal
/// brackets in rendered text such as `[image]` markers.
fn strip_tags(s: &str) -> String {
    let mut out = s.to_string();
    for tag in [
        "[bold]",
        "[/intensity]",
        "[dim]",
        "[italic]",
        "[/italic]",
        "[underline]",
        "[/underline]",
        "[code]",
        "[/color]",
        "[strike]",
        "[/strike]",
    ] {
        out = out.replace(tag, "");
    }
    out
}

// =========================================================
// Empty / whitespace input
// =========================================================

#[test]
fn empty_input() {
    assert_snapshot!(render(""), @"");
}

#[test]
fn whitespace_only_input() {
    assert_snapshot!(render("   "), @"");
}

// =========================================================
// Headings
// =========================================================

#[test]
fn h1_bold_italic_double_underline() {
    assert_snapshot!(render("# Hello World"));
}

#[test]
fn h2_bold_underline() {
    assert_snapshot!(render("## Section Title"));
}

#[test]
fn h3_bold() {
    assert_snapshot!(render("### Sub Section"));
}

#[test]
fn h4_bold() {
    assert_snapshot!(render("#### Deep Heading"));
}

#[test]
fn consecutive_headings() {
    assert_snapshot!(render("# Title\n\n## Section\n\n### Sub"));
}

#[test]
fn h1_with_inline_formatting() {
    assert_snapshot!(render("# Hello **bold** World"));
}

// =========================================================
// Paragraphs
// =========================================================

#[test]
fn simple_paragraph() {
    assert_snapshot!(render("Hello world"));
}

#[test]
fn two_paragraphs() {
    assert_snapshot!(render("First paragraph.\n\nSecond paragraph."));
}

#[test]
fn paragraph_after_heading() {
    assert_snapshot!(render("## Title\n\nSome body text here."));
}

// =========================================================
// Inline formatting
// =========================================================

#[test]
fn bold_text() {
    assert_snapshot!(render("This is **bold** text."));
}

#[test]
fn italic_text() {
    assert_snapshot!(render("This is *italic* text."));
}

#[test]
fn bold_italic_text() {
    assert_snapshot!(render("This is ***bold italic*** text."));
}

#[test]
fn mixed_inline() {
    assert_snapshot!(render("Start **bold** middle *italic* end."));
}

#[test]
fn adjacent_bold_italic() {
    assert_snapshot!(render("**bold***italic*"));
}

#[test]
fn bold_entire_paragraph() {
    assert_snapshot!(render("**Everything is bold here.**"));
}

#[test]
fn bold_with_trailing_space_before_closing() {
    // Trailing space before `**` prevents markdown from parsing as bold.
    // This must still render the `**` markers literally.
    assert_snapshot!(render("**not bold **"));
}

#[test]
fn bold_without_trailing_space() {
    // Without trailing space, markdown correctly parses as bold.
    assert_snapshot!(render("**is bold**"));
}

// =========================================================
// Word wrapping
// =========================================================

#[test]
fn wraps_at_79_columns() {
    // 80+ characters of content to trigger a wrap.
    let long = "word ".repeat(20); // 100 chars of "word word word ..."
    assert_snapshot!(render(&long));
}

#[test]
fn single_long_word_no_break() {
    // A single word longer than 79 chars should not be broken (no hyphenation).
    let word = "a".repeat(100);
    assert_snapshot!(render(&word));
}

#[test]
fn wrap_preserves_bold() {
    // Bold text that spans across a line wrap should re-apply bold on the
    // new line.
    let long = format!("**{}**", "bold ".repeat(20).trim());
    assert_snapshot!(render(&long));
}

#[test]
fn wrap_preserves_nested_styles() {
    // Bold + italic spanning a wrap: both should be re-applied.
    let long = format!("***{}***", "styled ".repeat(15).trim());
    assert_snapshot!(render(&long));
}

#[test]
fn wrap_mid_paragraph_with_mixed_inline() {
    let text = format!(
        "Before **bold section that is {} enough to wrap** after.",
        "quite long ".repeat(8)
    );
    assert_snapshot!(render(&text));
}

// =========================================================
// Soft breaks
// =========================================================

#[test]
fn soft_break_becomes_space() {
    // A single newline in the source (not preceded by two spaces) is a soft
    // break, which should render as a space between words.
    assert_snapshot!(render("first\nsecond"));
}

// =========================================================
// Inline code
// =========================================================

#[test]
fn inline_code_colored() {
    assert_snapshot!(render("Use `println!` to print."));
}

#[test]
fn inline_code_in_bold_context() {
    assert_snapshot!(render("**bold `code` bold**"));
}

#[test]
fn multiple_inline_code_spans() {
    assert_snapshot!(render("Use `foo` and `bar` together."));
}

// =========================================================
// Strikethrough
// =========================================================

#[test]
fn strikethrough_text() {
    assert_snapshot!(render("This is ~~deleted~~ text."));
}

#[test]
fn strikethrough_with_bold() {
    assert_snapshot!(render("~~**bold and struck**~~"));
}

// =========================================================
// Links
// =========================================================

#[test]
fn link_with_text() {
    assert_snapshot!(render("[click here](https://example.com)"));
}

#[test]
fn link_in_sentence() {
    assert_snapshot!(render(
        "See the [documentation](https://example.com/docs) for details."
    ));
}

#[test]
fn bare_autolink() {
    assert_snapshot!(render("<https://example.com>"));
}

#[test]
fn link_with_bold_text() {
    assert_snapshot!(render("[**bold link**](https://example.com)"));
}

// =========================================================
// Regression: link underline must not bleed into surrounding space
// =========================================================

#[test]
fn link_space_before_not_underlined() {
    // The space between preceding text and the link must not be underlined.
    let output = render("word [link](https://x.com) word");
    // The underline must start at "link", not at the space before it.
    assert!(
        output.contains("word [underline]link[/underline]"),
        "space before link text must not be underlined.\nOutput: {output}"
    );
}

#[test]
fn link_text_is_underlined() {
    let output = render("[click here](https://example.com)");
    assert!(
        output.contains("[underline]click here[/underline]"),
        "link text must be underlined.\nOutput: {output}"
    );
}

#[test]
fn long_link_url_wraps_onto_new_line() {
    // A URL that won't fit on the current line alongside the link text
    // should move to its own line. The URL itself must never be broken
    // mid-string (it may overflow on its own line — overflow is preferable
    // to making a URL uncopyable).
    let url = "https://example.com/some/very/long/path/that/exceeds";
    let output = render_with_width(&format!("[link]({url})"), 40);
    // The link text "link" should be on one line; the URL suffix "(url)"
    // should begin a different line because it did not fit next to "link".
    let link_line = output
        .lines()
        .find(|l| l.contains("link"))
        .expect("must contain link text");
    assert!(
        !link_line.contains(url),
        "URL must not follow link text on the same line when it doesn't fit:\nline: {link_line}\nfull:\n{output}"
    );
    // The URL must still appear intact on some line.
    assert!(
        output.contains(url),
        "URL must not be broken mid-string:\n{output}"
    );
}

#[test]
fn long_image_url_wraps_onto_new_line() {
    let url = "https://example.com/some/very/long/path/that/exceeds.png";
    let output = render_with_width(&format!("![alt]({url})"), 40);
    let alt_line = output
        .lines()
        .find(|l| l.contains("[image: alt]"))
        .expect("must contain image alt marker");
    assert!(
        !alt_line.contains(url),
        "URL must not follow image marker on the same line when it doesn't fit:\nline: {alt_line}\nfull:\n{output}"
    );
    assert!(
        output.contains(url),
        "URL must not be broken mid-string:\n{output}"
    );
}

// =========================================================
// Regression: code blocks must have extra 2-space indent
// =========================================================

#[test]
fn code_block_extra_indent() {
    // Code blocks should be indented 2 spaces (one nesting level).
    // The color tag precedes the indent in our tagged output, so strip tags
    // before checking indentation.
    let output = render("```\ncode\n```");
    let content_line = output.lines().find(|l| l.contains("code")).unwrap();
    let stripped = content_line.replace("[code]", "").replace("[/color]", "");
    assert!(
        stripped.starts_with("  ") && !stripped.starts_with("    "),
        "code block should have 2-space nesting indent.\nStripped line: '{stripped}'"
    );
}

// =========================================================
// Regression: list must have blank line separation from preceding block
// =========================================================

#[test]
fn list_has_blank_line_after_heading() {
    let output = render("## Features\n\n- Item one\n- Item two");
    let lines: Vec<&str> = output.lines().collect();
    // Line 0: heading, Line 1: blank, Line 2: first list item.
    assert!(
        lines.len() >= 3,
        "expected at least 3 lines (heading, blank, list item)"
    );
    assert!(
        lines[1].trim().is_empty(),
        "there must be a blank line between heading and first list item.\nLine 1: '{}'\nFull output:\n{output}",
        lines[1]
    );
}

#[test]
fn list_has_blank_line_after_paragraph() {
    let output = render("Some text.\n\n- Item one\n- Item two");
    let lines: Vec<&str> = output.lines().collect();
    assert!(
        lines.len() >= 3,
        "expected at least 3 lines (paragraph, blank, list item)"
    );
    assert!(
        lines[1].trim().is_empty(),
        "there must be a blank line between paragraph and first list item.\nLine 1: '{}'\nFull output:\n{output}",
        lines[1]
    );
}

// =========================================================
// Ordered lists
// =========================================================

#[test]
fn ordered_list_basic() {
    assert_snapshot!(render("1. First\n2. Second\n3. Third"));
}

#[test]
fn ordered_list_with_bold() {
    assert_snapshot!(render(
        "1. **Item one** — description\n2. **Item two** — description"
    ));
}

#[test]
fn ordered_list_custom_start() {
    assert_snapshot!(render("5. Fifth\n6. Sixth"));
}

#[test]
fn ordered_list_after_heading() {
    assert_snapshot!(render("## Section\n\n1. First\n2. Second"));
}

// =========================================================
// Unordered lists
// =========================================================

#[test]
fn unordered_list_basic() {
    assert_snapshot!(render("- Alpha\n- Beta\n- Gamma"));
}

#[test]
fn unordered_list_with_inline_code() {
    assert_snapshot!(render("- Use `foo`\n- Use `bar`"));
}

// =========================================================
// Nested lists
// =========================================================

#[test]
fn nested_unordered_list() {
    assert_snapshot!(render(
        "- Top\n  - Nested\n    - Deep\n  - Back\n- Top again"
    ));
}

#[test]
fn ordered_inside_unordered() {
    assert_snapshot!(render("- Item\n  1. Sub one\n  2. Sub two\n- Item"));
}

#[test]
fn unordered_inside_ordered() {
    assert_snapshot!(render("1. Item\n   - Sub a\n   - Sub b\n2. Item"));
}

// =========================================================
// Tables
// =========================================================

#[test]
fn table_basic() {
    assert_snapshot!(render(
        "| Level | Items |\n|---|---|\n| High | Fix bugs |\n| Low | Cleanup |"
    ));
}

#[test]
fn table_header_centered() {
    // Header text should be centered within the column width.
    assert_snapshot!(render("| Name | Description |\n|---|---|\n| a | Short |"));
}

#[test]
fn table_with_bold_cells() {
    assert_snapshot!(render(
        "| Level | Items |\n|---|---|\n| **Blocker** | Auth system |"
    ));
}

#[test]
fn table_single_column() {
    assert_snapshot!(render("| Item |\n|---|\n| One |\n| Two |"));
}

#[test]
fn table_after_heading() {
    assert_snapshot!(render(
        "## Priority\n\n| Level | Items |\n|---|---|\n| High | Bugs |"
    ));
}

// =========================================================
// Regression: tables inside blockquotes must carry blockquote bars
// =========================================================

#[test]
fn table_in_blockquote_has_bars() {
    // Every line of a table rendered inside a blockquote must start
    // with the blockquote `│ ` bar. The table itself uses `│` for cell
    // borders, so we must strip ANSI tags *and* check the line prefix
    // — not just whether `│` appears anywhere.
    let output = render("> | col1 | col2 |\n> |------|------|\n> | a    | b    |");
    let table_line_markers = ["┌", "│ col1", "├", "│ a ", "└"];
    for marker in table_line_markers {
        let line = output
            .lines()
            .map(strip_tags)
            .find(|l| l.contains(marker))
            .unwrap_or_else(|| panic!("missing table line with {marker:?}\nfull:\n{output}"));
        assert!(
            line.starts_with("│ "),
            "line containing {marker:?} must start with a blockquote bar:\nline: {line:?}\nfull:\n{output}"
        );
    }
}

#[test]
fn table_in_nested_blockquote_has_double_bars() {
    // Two levels of blockquote nesting should produce two bar prefixes
    // on every table line.
    let output = render("> > | a | b |\n> > |--|--|\n> > | 1 | 2 |");
    let table_line_markers = ["┌", "│ a", "├", "│ 1", "└"];
    for marker in table_line_markers {
        let line = output
            .lines()
            .map(strip_tags)
            .find(|l| l.contains(marker))
            .unwrap_or_else(|| panic!("missing table line with {marker:?}\nfull:\n{output}"));
        assert!(
            line.starts_with("│ │ "),
            "line containing {marker:?} must start with two blockquote bars:\nline: {line:?}\nfull:\n{output}"
        );
    }
}

#[test]
fn table_after_blockquote_still_works() {
    // A table immediately following a blockquote (not inside it) must
    // render normally with no bars. This guards against the new
    // per-line `write_indent` loop accidentally picking up stale state.
    assert_snapshot!(render(
        "> a quote\n\n| col |\n|---|\n| val |"
    ));
}

#[test]
fn table_in_blockquote_snapshot() {
    // Full visual pin for a table inside a single-level blockquote.
    // Complements the per-line bar assertions above by locking in the
    // exact expected output shape (bar prefix, box drawing, bold header,
    // data cells) so future refactors cannot silently change it.
    assert_snapshot!(render(
        "> | col1 | col2 |\n> |------|------|\n> | a    | b    |"
    ));
}

#[test]
fn table_in_nested_blockquote_snapshot() {
    // Same, for two levels of blockquote nesting.
    assert_snapshot!(render("> > | a | b |\n> > |--|--|\n> > | 1 | 2 |"));
}

// =========================================================
// Table cell wrapping and width-aware column layout (phase 2)
// =========================================================

#[test]
fn table_wraps_long_cell_text() {
    // A table with a long sentence in one cell, rendered at a width too
    // narrow to fit naturally. After wrapping, no output line should
    // exceed the target width and every word from the input must still
    // appear somewhere in the rendered output.
    let md = "\
| Topic | Notes |
|---|---|
| cats | The quick brown fox jumps over the lazy dog repeatedly |";
    let output = render_with_width(md, 30);
    for line in output.lines() {
        let visible = strip_tags(line);
        assert!(
            unicode_width::UnicodeWidthStr::width(visible.as_str()) <= 30,
            "line exceeds width 30: {visible:?}\nfull:\n{output}"
        );
    }
    for word in [
        "quick", "brown", "fox", "jumps", "over", "the", "lazy", "dog", "repeatedly",
    ] {
        assert!(
            output.contains(word),
            "missing word {word:?} after wrapping:\n{output}"
        );
    }
}

#[test]
fn table_preserves_column_proportions() {
    // Three columns with very different natural widths. Their sum far
    // exceeds a 40-column budget, so the table must shrink. After
    // shrinking, (a) the overall table must fit in 40 columns and
    // (b) the column containing longer content must still be wider
    // than the one containing shorter content — proportions are
    // respected, not flattened.
    let md = "\
| A | Middle column | Much longer final column content |
|---|---|---|
| x | medium text | a lot more text here to pad it out |";
    let output = render_with_width(md, 40);
    // Every line must fit in 40 columns — this is the primary "shrank"
    // assertion that fails without width-aware layout.
    for line in output.lines() {
        let visible = strip_tags(line);
        assert!(
            unicode_width::UnicodeWidthStr::width(visible.as_str()) <= 40,
            "line exceeds width 40: {visible:?}\nfull:\n{output}"
        );
    }
    // Find the top border line (deterministic shape): `┌─...┬─...┬─...┐`.
    let border = output
        .lines()
        .map(strip_tags)
        .find(|l| l.starts_with('┌'))
        .expect("expected top border");
    // Segment widths are the number of `─` runs between the corners/T
    // junctions. Split on the junction characters to get each column's
    // segment (including its +2 padding).
    let segments: Vec<usize> = border
        .trim_start_matches('┌')
        .trim_end_matches('┐')
        .split('┬')
        .map(|seg| seg.chars().filter(|&c| c == '─').count())
        .collect();
    assert_eq!(segments.len(), 3, "expected 3 columns, got {segments:?}");
    // Column 3 (longest natural width) must remain widest; column 1
    // (shortest) must remain narrowest, even after shrinking.
    assert!(
        segments[2] > segments[1],
        "column 3 should be wider than column 2: {segments:?}"
    );
    assert!(
        segments[1] > segments[0],
        "column 2 should be wider than column 1: {segments:?}"
    );
}

#[test]
fn table_overflows_when_minimum_widths_exceed_budget() {
    // Each column contains a single long word that cannot wrap any
    // narrower. At a tiny terminal width, the table is forced to
    // overflow — but it must still render cleanly (no panic, no empty
    // output) and the wide words must appear intact (never split).
    let md = "\
| Col1 | Col2 |
|---|---|
| antidisestablishmentarianism | supercalifragilisticexpialidocious |";
    let output = render_with_width(md, 10);
    assert!(
        !output.trim().is_empty(),
        "expected non-empty output even when overflow is forced"
    );
    assert!(
        output.contains("antidisestablishmentarianism"),
        "first long word must appear intact:\n{output}"
    );
    assert!(
        output.contains("supercalifragilisticexpialidocious"),
        "second long word must appear intact:\n{output}"
    );
}

#[test]
fn table_header_centered_with_wrapping() {
    // A long header name forced to wrap to a narrow column. The header
    // cell must span more than one visual line (= wrapping happened),
    // and every line that contains one of the header words must also
    // carry the `[bold]` styling tag.
    let md = "\
| Very Long Header Name | x |
|---|---|
| a | b |";
    let output = render_with_width(md, 24);

    // Count distinct header-content lines: lines that start with the
    // table's left border `│` and contain any of the header words.
    let lines: Vec<String> = output.lines().map(strip_tags).collect();
    let header_words = ["Very", "Long", "Header", "Name"];
    let header_line_count = lines
        .iter()
        .filter(|l| l.starts_with('│') && header_words.iter().any(|w| l.contains(w)))
        .count();
    assert!(
        header_line_count >= 2,
        "header should wrap to ≥2 lines, saw {header_line_count}:\n{output}"
    );

    // Every header word must appear on a line that also contains the
    // bold styling tag (i.e. the wrapped header lines all stay bold).
    for word in header_words {
        let found = output
            .lines()
            .any(|line| line.contains("[bold]") && line.contains(word));
        assert!(found, "header word {word:?} must appear bold:\n{output}");
    }
}

#[test]
fn table_in_blockquote_with_wrapping() {
    // Phase 1 × Phase 2: a table inside a blockquote at narrow width.
    // Every line must still carry the blockquote bar prefix, and the
    // cells must be wrapped so the whole structure fits within the
    // available (content minus bar) width.
    let md = "\
> | Topic | Notes |
> |---|---|
> | cats | The quick brown fox jumps over the lazy dog |";
    let output = render_with_width(md, 36);
    // Every non-border table line and every border line must begin
    // with the blockquote bar after tag stripping.
    for line in output.lines() {
        let visible = strip_tags(line);
        if visible.is_empty() {
            continue;
        }
        // Identify lines that belong to the table by the box-drawing
        // characters; they must all start with the blockquote bar.
        let is_table_line = ['┌', '├', '└', '│']
            .iter()
            .any(|c| visible.contains(*c));
        if is_table_line {
            assert!(
                visible.starts_with("│ "),
                "table line inside blockquote missing bar prefix:\nline: {visible:?}\nfull:\n{output}"
            );
        }
    }
    // The table must fit within 36 columns (content stripped of tags).
    for line in output.lines() {
        let visible = strip_tags(line);
        assert!(
            unicode_width::UnicodeWidthStr::width(visible.as_str()) <= 36,
            "line exceeds width 36: {visible:?}\nfull:\n{output}"
        );
    }
}

#[test]
fn table_variable_row_heights() {
    // One cell wraps to multiple lines while its sibling is short. The
    // row must be tall enough for the wrapped cell, with the short
    // sibling blank-padded on the extra lines (not collapsed).
    let md = "\
| Tall | Short |
|---|---|
| one two three four five six | x |";
    let output = render_with_width(md, 22);
    // Count the number of content lines between the header separator
    // and the bottom border by looking for lines that begin with the
    // table's left border `│` and contain the short "x" cell or any
    // wrapped word of the tall cell.
    let lines: Vec<String> = output.lines().map(strip_tags).collect();
    // Indices of separator and bottom border anchor the data region.
    let sep_idx = lines
        .iter()
        .position(|l| l.starts_with('├'))
        .expect("separator");
    let bot_idx = lines
        .iter()
        .position(|l| l.starts_with('└'))
        .expect("bottom border");
    // The data row should span more than one content line because the
    // tall cell wrapped.
    let row_lines = &lines[sep_idx + 1..bot_idx];
    assert!(
        row_lines.len() >= 2,
        "expected variable-height row to span ≥2 lines, got {}: {row_lines:#?}",
        row_lines.len()
    );
    // The "x" must appear on at least one of those lines.
    assert!(
        row_lines.iter().any(|l| l.contains(" x ")),
        "short cell 'x' must appear on at least one row line:\n{output}"
    );
}

// =========================================================
// Code blocks
// =========================================================

#[test]
fn code_block_basic() {
    assert_snapshot!(render("```\nfn main() {\n    println!(\"hello\");\n}\n```"));
}

#[test]
fn code_block_with_language() {
    assert_snapshot!(render("```rust\nlet x = 42;\n```"));
}

#[test]
fn code_block_no_wrap() {
    // A very long line in a code block must not be wrapped.
    let long_line = "x".repeat(120);
    let md = format!("```\n{long_line}\n```");
    let mut buf: Vec<u8> = Vec::new();
    patine::render(&md, &mut buf, 80).expect("render should not fail");
    let output = String::from_utf8(buf).expect("valid utf-8");
    // The long line should appear intact (not broken into multiple lines).
    assert!(
        output.contains(&long_line),
        "code block lines must not be wrapped"
    );
}

#[test]
fn code_block_between_paragraphs() {
    assert_snapshot!(render("Before.\n\n```\ncode\n```\n\nAfter."));
}

// =========================================================
// Images
// =========================================================

#[test]
fn image_with_alt() {
    assert_snapshot!(render("![Architecture diagram](./docs/arch.png)"));
}

#[test]
fn image_without_alt() {
    assert_snapshot!(render("![](./screenshot.png)"));
}

#[test]
fn image_in_paragraph() {
    assert_snapshot!(render(
        "See the diagram: ![overview](./overview.png) for context."
    ));
}

// =========================================================
// Blockquotes
// =========================================================

#[test]
fn blockquote_simple() {
    assert_snapshot!(render("> This is a quote."));
}

#[test]
fn blockquote_multiple_paragraphs() {
    assert_snapshot!(render("> First paragraph.\n>\n> Second paragraph."));
}

#[test]
fn blockquote_nested() {
    assert_snapshot!(render("> Outer\n>\n>> Inner"));
}

#[test]
fn blockquote_with_emphasis() {
    assert_snapshot!(render("> This has **bold** in a quote."));
}

// =========================================================
// Regression: blockquote blank lines must have bar prefix
// =========================================================

#[test]
fn blockquote_blank_separator_has_bar() {
    // The blank line between two paragraphs inside a blockquote must render
    // with the │ bar prefix, so the visual bar column is unbroken.
    let output = render("> First\n>\n> Second");
    let lines: Vec<&str> = output.lines().collect();
    // Find lines that are purely whitespace or style tags (no visible text
    // and no bar). Every line between content lines must have the bar.
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        let stripped = line
            .replace("[dim]", "")
            .replace("[/dim]", "")
            .replace("[italic]", "")
            .replace("[/italic]", "")
            .replace("[bold]", "")
            .replace("[/bold]", "");
        let stripped = stripped.trim();
        // Lines that are purely closing style tags (no visible content) at the
        // end of the blockquote are expected — they close the outer dim/italic.
        if stripped.is_empty() && !line.contains('│') {
            // This is only acceptable as the very last non-empty line.
            let remaining_has_content = lines[i + 1..]
                .iter()
                .any(|l| !l.is_empty() && l.contains('│'));
            if remaining_has_content {
                panic!(
                    "line {i} inside blockquote has no │ bar prefix.\n\
                     Line content: '{line}'\n\
                     Full output:\n{output}"
                );
            }
        }
    }
}

// =========================================================
// Regression: list item text wrapping must indent past bullet
// =========================================================

#[test]
fn list_item_wrap_indentation() {
    // When a list item's text is long enough to wrap, the continuation line
    // must be indented past the bullet/number, not back at the left margin.
    let long_item = format!("- {}", "word ".repeat(30).trim());
    let output = render(&long_item);
    let lines: Vec<&str> = output.lines().collect();
    assert!(
        lines.len() >= 2,
        "expected wrapping to produce multiple lines"
    );
    // The second line should have more indentation than just the 2-space
    // global indent — it should align with the text after "• ".
    let second_line = lines[1];
    // "• " is 2 columns, so continuation should start with 2 spaces.
    assert!(
        second_line.starts_with("  "),
        "wrapped list continuation should be indented past the bullet.\nSecond line: '{second_line}'"
    );
}

#[test]
fn ordered_list_item_wrap_indentation() {
    let long_item = format!("1. {}", "word ".repeat(30).trim());
    let output = render(&long_item);
    let lines: Vec<&str> = output.lines().collect();
    assert!(
        lines.len() >= 2,
        "expected wrapping to produce multiple lines"
    );
    let second_line = lines[1];
    // "1. " is 3 chars, so continuation should start with 3 spaces.
    assert!(
        second_line.starts_with("   "),
        "wrapped ordered list continuation should be indented past the number.\nSecond line: '{second_line}'"
    );
}

// =========================================================
// Regression: blockquote must not emit bare styles before first bar
// =========================================================

#[test]
fn blockquote_no_bare_style_lines() {
    // The blockquote must not emit lines that are purely style escape codes
    // without the bar prefix. Every line with content or style tags should
    // also include the │ bar (except the final style-closing line).
    let output = render("> Hello");
    let lines: Vec<&str> = output.lines().collect();
    // The first line must contain the bar.
    assert!(
        lines[0].contains('│'),
        "first line of blockquote must contain │ bar.\nFirst line: '{}'\nFull output:\n{output}",
        lines[0]
    );
}

// =========================================================
// Regression: no empty bar line at start of blockquote
// =========================================================

#[test]
fn blockquote_no_leading_empty_bar_line() {
    // When a blockquote follows another block (e.g., a heading), the blank
    // line separator must NOT contain a │ bar. The bar should only appear
    // on lines that are part of the blockquote content.
    let output = render("# Title\n\n> Quote");
    let lines: Vec<&str> = output.lines().collect();
    // Find the first line that contains │. It must also contain the word "Quote".
    let first_bar_line = lines.iter().find(|l| l.contains('│'));
    assert!(
        first_bar_line.is_some_and(|l| l.contains("Quote")),
        "first line with │ bar must be a content line, not an empty bar.\nFull output:\n{output}"
    );
}

#[test]
fn heading_then_blockquote_has_blank_line() {
    // A blank line must separate the heading from the blockquote content.
    let output = render("# Title\n\n> Quote");
    let lines: Vec<&str> = output.lines().collect();
    // There must be an empty line between the heading line and the blockquote.
    assert!(
        lines.len() >= 3,
        "expected at least 3 lines (heading, blank, blockquote)"
    );
    // The second line (index 1) should be empty (the blank separator).
    assert!(
        lines[1].trim().is_empty() || !lines[1].contains('│'),
        "blank line between heading and blockquote must not have a bar.\nLine: '{}'\nFull output:\n{output}",
        lines[1]
    );
}

// =========================================================
// Regression: NoBold must not produce double underlines
// =========================================================

#[test]
fn bold_disable_uses_normal_intensity() {
    // crossterm's Attribute::NoBold emits SGR 21, which some terminals
    // interpret as "doubly underlined" rather than "not bold". We must use
    // NormalIntensity (SGR 22) instead. Verify that the rendered output
    // does NOT contain \x1b[21m.
    let mut buf: Vec<u8> = Vec::new();
    patine::render("**bold**", &mut buf, 80).expect("render should not fail");
    let raw = String::from_utf8(buf).expect("valid utf-8");
    assert!(
        !raw.contains("\x1b[21m"),
        "output must not contain SGR 21 (NoBold). Use SGR 22 (NormalIntensity) instead.\nRaw output: {raw:?}"
    );
}

#[test]
fn bold_then_dim_no_double_underline() {
    // After bold text followed by a dim section (e.g., link URL), there
    // must be no SGR 21 in the output.
    let mut buf: Vec<u8> = Vec::new();
    patine::render("**bold** then [link](https://x.com)", &mut buf, 80)
        .expect("render should not fail");
    let raw = String::from_utf8(buf).expect("valid utf-8");
    assert!(
        !raw.contains("\x1b[21m"),
        "output must not contain SGR 21.\nRaw output: {raw:?}"
    );
}

// =========================================================
// Horizontal rules
// =========================================================

#[test]
fn horizontal_rule() {
    assert_snapshot!(render("Above\n\n---\n\nBelow"));
}

#[test]
fn horizontal_rule_width() {
    // At 40-column terminal width, the rule should span all 40 columns.
    let mut buf: Vec<u8> = Vec::new();
    patine::render("---", &mut buf, 40).expect("render should not fail");
    let output = String::from_utf8(buf).expect("valid utf-8");
    let dash_count = output.matches('─').count();
    assert_eq!(dash_count, 40, "rule should span full terminal width");
}

// =========================================================
// Block spacing
// =========================================================

#[test]
fn no_leading_blank_line() {
    let output = render("# Hello");
    assert!(
        !output.starts_with('\n'),
        "output must not start with a blank line"
    );
}

#[test]
fn trailing_newline() {
    let output = render("Hello");
    assert!(output.ends_with('\n'), "output must end with a newline");
    assert!(
        !output.ends_with("\n\n"),
        "output must not end with a double newline"
    );
}

#[test]
fn heading_paragraph_heading_spacing() {
    assert_snapshot!(render("## First\n\nA paragraph.\n\n## Second"));
}

// =========================================================
// Effective width floor
// =========================================================

#[test]
fn effective_width_floored_thematic_break_not_empty() {
    // With a very narrow terminal and deep nesting, effective_width() can
    // reach 0 after subtracting indent widths. A thematic break rendered at
    // effective_width=0 produces an empty (invisible) rule because
    // "─".repeat(0) is "".
    //
    // Four levels of blockquote at width 8 consumes all 8 columns of indent
    // (4 × 2 = 8), leaving effective_width = 0 without the floor.
    let output = render_with_width("> > > > ---", 8);
    assert!(
        output.contains('─'),
        "thematic break must not disappear at zero effective width:\n{output}"
    );
}

// =========================================================
// Edge case: very small terminal widths
// =========================================================

#[test]
fn very_narrow_width_zero_does_not_panic() {
    // Width 0 should not panic; the `effective_width().max(1)` floor and
    // `wrap_text`'s `width.max(1)` defensive clamp together should be
    // enough. Whatever we produce is allowed to overflow — but the
    // render call itself must succeed and produce non-empty output for
    // non-empty input.
    let output = render_with_width("hello world", 0);
    assert!(!output.trim().is_empty(), "empty output at width 0");
    assert!(output.contains("hello"));
    assert!(output.contains("world"));
}

#[test]
fn very_narrow_width_one_does_not_panic() {
    let output = render_with_width("hello world", 1);
    assert!(!output.trim().is_empty(), "empty output at width 1");
    assert!(output.contains("hello"));
    assert!(output.contains("world"));
}

#[test]
fn narrow_width_five_wraps_every_word() {
    // At width 5, "hello" fits exactly, so each word goes on its own line.
    let output = render_with_width("hello world foo bar", 5);
    // Every word should appear.
    for word in ["hello", "world", "foo", "bar"] {
        assert!(output.contains(word), "missing {word:?} in:\n{output}");
    }
    // Every word should be on its own line (because 5+1+5 > 5).
    let visible_lines: Vec<String> = output
        .lines()
        .map(strip_tags)
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert!(
        visible_lines.len() >= 4,
        "expected ≥4 lines (one per word), got {}:\n{output}",
        visible_lines.len()
    );
}

#[test]
fn narrow_width_heading_does_not_panic() {
    // Headings have their own block spacing and underline logic; make
    // sure it survives narrow widths without panicking.
    let output = render_with_width("# a heading", 3);
    assert!(output.contains("heading") || output.contains("a"));
}

#[test]
fn narrow_width_code_block_does_not_panic() {
    let output = render_with_width("```\nlong code line here\n```", 6);
    // The content should still appear; code blocks don't wrap, so this
    // just confirms we don't panic and output is non-empty.
    assert!(!output.trim().is_empty());
    assert!(output.contains("long"));
}

// =========================================================
// Edge case: hard line breaks
// =========================================================

#[test]
fn hard_break_trailing_backslash() {
    // A `\` at the end of a line is a Markdown hard break. The two
    // parts must appear on separate visual lines (not joined by a
    // space).
    let output = render("first line\\\nsecond line");
    // Check: "first line" and "second line" are on different lines.
    let lines: Vec<String> = output.lines().map(strip_tags).collect();
    let first_idx = lines
        .iter()
        .position(|l| l.contains("first line"))
        .expect("first line present");
    let second_idx = lines
        .iter()
        .position(|l| l.contains("second line"))
        .expect("second line present");
    assert_ne!(
        first_idx, second_idx,
        "hard break must put the two halves on different visual lines:\n{output}"
    );
}

#[test]
fn hard_break_two_trailing_spaces() {
    // Two trailing spaces before a newline is the classic Markdown
    // hard-break spelling. Same expected behavior as the backslash
    // form.
    let output = render("first line  \nsecond line");
    let lines: Vec<String> = output.lines().map(strip_tags).collect();
    let first_idx = lines
        .iter()
        .position(|l| l.contains("first line"))
        .expect("first line present");
    let second_idx = lines
        .iter()
        .position(|l| l.contains("second line"))
        .expect("second line present");
    assert_ne!(
        first_idx, second_idx,
        "hard break must put the two halves on different visual lines:\n{output}"
    );
}

// =========================================================
// Edge case: multi-paragraph list items
// =========================================================

#[test]
fn list_item_with_multiple_paragraphs() {
    // A single list item containing two paragraphs. Both paragraphs
    // must render inside the item (indented past the bullet), with a
    // blank line between them — not glued together inline.
    let md = "- first paragraph\n\n  second paragraph\n- next item";
    let output = render(md);
    assert!(
        output.contains("first paragraph"),
        "missing first paragraph:\n{output}"
    );
    assert!(
        output.contains("second paragraph"),
        "missing second paragraph:\n{output}"
    );
    assert!(
        output.contains("next item"),
        "missing next item:\n{output}"
    );
    // The second paragraph must appear after the first and before
    // "next item" in the output.
    let first_pos = output.find("first paragraph").unwrap();
    let second_pos = output.find("second paragraph").unwrap();
    let next_pos = output.find("next item").unwrap();
    assert!(
        first_pos < second_pos && second_pos < next_pos,
        "paragraphs out of order:\nfirst@{first_pos} second@{second_pos} next@{next_pos}\n{output}"
    );
    // There must be at least one newline between the two paragraphs
    // of the same list item — they cannot be glued together inline.
    let between = &output[first_pos + "first paragraph".len()..second_pos];
    assert!(
        between.contains('\n'),
        "expected newline between the two paragraphs, got {between:?}\nfull:\n{output}"
    );
    // Pin the exact visual shape so regressions are loud.
    assert_snapshot!(output);
}

// =========================================================
// Edge case: empty code blocks
// =========================================================

#[test]
fn empty_code_block_does_not_panic() {
    // A fenced code block with no body must render without panicking
    // and must produce no visible text (empty → empty). ANSI style
    // escapes may still be emitted around the absent body; they're
    // ignored here by stripping tags before inspecting visible
    // content.
    let output = render("```\n```");
    let visible = strip_tags(&output);
    assert!(
        visible.trim().is_empty(),
        "empty code block should produce no visible content, got {visible:?}"
    );
}

#[test]
fn empty_code_block_with_language_does_not_panic() {
    let output = render("```rust\n```");
    let visible = strip_tags(&output);
    assert!(
        visible.trim().is_empty(),
        "empty code block should produce no visible content, got {visible:?}"
    );
}

#[test]
fn empty_code_block_then_following_content_still_renders() {
    // An empty code block followed by a paragraph should not swallow
    // or mangle the following content.
    let output = render("```\n```\n\ntext after empty block");
    assert!(
        output.contains("text after empty block"),
        "content after empty code block must still render:\n{output}"
    );
}

// =========================================================
// End-to-end showcase: a realistic document using every feature
// =========================================================

/// Fixture: a realistic document that exercises every Markdown feature
/// patine supports — h1/h2/h3, paragraphs with soft breaks,
/// bold/italic/bold+italic/strikethrough, inline code, fenced code
/// blocks (with and without a language hint), ordered and unordered
/// lists, nested lists, multi-paragraph list items, simple and nested
/// blockquotes, tables, thematic breaks, inline links, autolinks,
/// images, and hard breaks.
///
/// The snapshot locks in the full end-to-end visual shape. Any change
/// that alters how ANY feature renders will show up here in a single
/// diff, making regressions trivially visible during review.
const SHOWCASE_MD: &str = include_str!("fixtures/showcase.md");

#[test]
fn showcase_document_renders() {
    assert_snapshot!(render(SHOWCASE_MD));
}
