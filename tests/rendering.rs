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
    let mut buf: Vec<u8> = Vec::new();
    patine::render(markdown, &mut buf, 80).expect("render should not fail");

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
        .replace("\x1b[4m", "[underline]")
        .replace("\x1b[9m", "[strike]")
        .replace("\x1b[29m", "[/strike]")
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
fn h1_italic_underline() {
    assert_snapshot!(render("# Hello World"));
}

#[test]
fn h2_bold() {
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
