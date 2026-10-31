// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Constructs whose rendering no other test pins. Each snapshot holds the
//! input as a string literal above the lines it renders to, so a change in
//! the output shows next to the input that caused it. Tests with several
//! inputs separate them with a rule.

mod common;

use common::{pinned, pinned_all};
use insta::assert_snapshot;
use patine::{Html, RenderOptions, SoftBreaks};

const WIDTH: usize = 80;

fn default() -> RenderOptions {
    RenderOptions::default()
}

fn break_and_literal() -> RenderOptions {
    RenderOptions::new()
        .with_soft_breaks(SoftBreaks::Break)
        .with_html(Html::Literal)
}

// =========================================================
// Reference links and images
// =========================================================

#[test]
fn reference_links_full_collapsed_shortcut_and_undefined() {
    assert_snapshot!(pinned(
        "full [text][r], collapsed [r][], shortcut [r], undefined [text][nope] and [nope].\n\n[r]: /u \"title\"",
        WIDTH,
        &default()
    ));
}

#[test]
fn reference_images_defined_and_undefined() {
    assert_snapshot!(pinned(
        "full ![alt][r], collapsed ![r][], shortcut ![r], undefined ![alt][nope].\n\n[r]: /u.png",
        WIDTH,
        &default()
    ));
}

// =========================================================
// Footnotes
// =========================================================

#[test]
fn footnote_definition_between_paragraphs() {
    assert_snapshot!(pinned(
        "first\n\n[^1]: the note\n\nsecond[^1]",
        WIDTH,
        &default()
    ));
}

#[test]
fn footnote_referenced_twice_defined_last_with_dangling_reference() {
    assert_snapshot!(pinned(
        "one[^a] two[^a] three[^missing]\n\n[^a]: the note",
        WIDTH,
        &default()
    ));
}

#[test]
fn footnote_definitions_unreferenced_reversed_duplicated_and_nested() {
    assert_snapshot!(pinned_all(
        &[
            "text\n\n[^unused]: nobody refers to this",
            "[^a]: first defined\n\n[^b]: second defined\n\nrefers to b[^b] and then a[^a]",
            "[^a]: one\n\n[^a]: two\n\nrefers[^a]",
            "- item[^n]\n\n  [^n]: note in an item\n\nafter",
            "> quote[^n]\n>\n> [^n]: note in a quote\n\nafter",
        ],
        WIDTH,
        &default()
    ));
}

// =========================================================
// Tables
// =========================================================

#[test]
fn table_cell_past_the_delimiter_row() {
    assert_snapshot!(pinned("| a | b |\n|-|-|\n| 1 | 2 | 3 |", WIDTH, &default()));
}

#[test]
fn table_short_row() {
    assert_snapshot!(pinned(
        "| a | b | c |\n|---|---|---|\n| 1 |\n| 2 | 3 |",
        WIDTH,
        &default()
    ));
}

#[test]
fn table_header_and_delimiter_counts_differ() {
    assert_snapshot!(pinned_all(
        &["| a | b |\n|---|\n| 1 | 2 |", "| a |\n|---|---|\n| 1 | 2 |",],
        WIDTH,
        &default()
    ));
}

#[test]
fn table_escaped_pipe_in_cell() {
    assert_snapshot!(pinned(
        "| a \\| b | c |\n|---|---|\n| `x \\| y` | z |",
        WIDTH,
        &default()
    ));
}

// =========================================================
// Task markers
// =========================================================

#[test]
fn task_markers_without_text() {
    assert_snapshot!(pinned_all(
        &["- [x]", "- [ ]", "- [ ] ", "1. [x]", "- [x]\n- [ ] text"],
        WIDTH,
        &default()
    ));
}

#[test]
fn task_markers_upper_and_lower_case() {
    assert_snapshot!(pinned(
        "- [X] upper\n- [x] lower\n- [ ] open",
        WIDTH,
        &default()
    ));
}

// =========================================================
// Front matter
// =========================================================

#[test]
fn frontmatter_directly_followed_by_text() {
    assert_snapshot!(pinned_all(
        &[
            "---\ntitle: x\n---\nBody text",
            "+++\ntitle = \"x\"\n+++\nBody text",
        ],
        WIDTH,
        &default()
    ));
}

#[test]
fn frontmatter_yaml_containing_plus_line() {
    assert_snapshot!(pinned(
        "---\na: 1\n+++\nb: 2\n---\n\nBody",
        WIDTH,
        &default()
    ));
}

/// The fence rules: what opens and closes a block, and what is left to
/// parse after it.
#[test]
fn frontmatter_fence_rules() {
    assert_snapshot!(pinned_all(
        &[
            "---\n---\n\nBody",
            "+++\n+++\n\nBody",
            "---\n\n---\n\nBody",
            "--- \ntitle: x\n---\t\n\nBody",
            "+++ \ntitle = 1\n+++ \n\nBody",
            "----\ntitle: x\n----\n\nBody",
            "---\ntitle: x\n----\n\nBody",
            "---\ntitle: x\n---",
            "---\ntitle: x",
            "+++\ntitle = 1",
            "---\r\ntitle: x\r\n---\r\n\r\nBody",
            "---\rtitle: x\r---\r\rBody",
            "---\na\n---\nb\r\n---\r\nBody",
            "\u{feff}---\ntitle: x\n---\n\nBody",
            "---\ntitle: x\n--not a fence\n---\n\nBody",
            "+++\ntitle = 1\n+++\n\n---\nsecond: 2\n---\n\nBody",
            "---\ntitle: x\n---\n\n+++\nsecond = 2\n+++\n\nBody",
            " ---\ntitle: x\n---\n\nBody",
        ],
        WIDTH,
        &default()
    ));
}

// =========================================================
// Escapes and entities
// =========================================================

#[test]
fn backslash_escapes() {
    assert_snapshot!(pinned(
        "\\*not em\\* \\_not em\\_ back\\\\slash \\# not a heading \\[not a link\\]",
        WIDTH,
        &default()
    ));
}

#[test]
fn entities() {
    assert_snapshot!(pinned(
        "&amp; &lt;tag&gt; a&nbsp;b &#x41; &#66; &copy; &unknown; &#0;",
        WIDTH,
        &default()
    ));
}

// =========================================================
// Blocks
// =========================================================

#[test]
fn setext_h2_followed_by_paragraph() {
    assert_snapshot!(pinned("Title\n-----\nparagraph after", WIDTH, &default()));
}

#[test]
fn top_level_indented_code() {
    assert_snapshot!(pinned(
        "para\n\n    code line\n\n    more code\n\nafter",
        WIDTH,
        &default()
    ));
}

#[test]
fn tilde_fence_containing_backticks() {
    assert_snapshot!(pinned(
        "~~~\n```\ninner\n```\n~~~\n\nafter",
        WIDTH,
        &default()
    ));
}

#[test]
fn tabs_in_text_item_and_code() {
    assert_snapshot!(pinned(
        "a\tb\n\n-\titem\ttext\n\n```\ncode\tline\n```\n\n\tindented\tcode",
        WIDTH,
        &default()
    ));
}

/// Info strings, blank last lines and empty fences: what the language of a
/// block is and which lines it shows.
#[test]
fn code_block_info_strings_and_trailing_lines() {
    assert_snapshot!(pinned_all(
        &[
            "```rust ignore\nfn main() {}\n```",
            "```rust,ignore\nfn main() {}\n```",
            "```rust\tignore\nfn main() {}\n```",
            "```  rust\nfn main() {}\n```",
            "```\nlast line blank\n\n```",
            "```\nthree blank\n\n\n\n```",
            "```\n```",
            "```rust\n```",
            "    indented\n\n\n    again",
            "```\r\nwith crlf\r\n\r\n```",
            "```\rwith cr\r\r```",
            "```\nunclosed fence\nstill code",
        ],
        WIDTH,
        &default()
    ));
}

// =========================================================
// Inline content and links
// =========================================================

#[test]
fn image_alt_with_emphasis_and_code() {
    assert_snapshot!(pinned(
        "![alt *em* and `code` and **strong**](/u.png)",
        WIDTH,
        &default()
    ));
}

#[test]
fn link_with_title() {
    assert_snapshot!(pinned(
        "[text](/u \"the title\") and [t](<a b> 'x')",
        WIDTH,
        &default()
    ));
}

#[test]
fn email_autolink() {
    assert_snapshot!(pinned(
        "write to <a@b.c> or a@b.co or mailto:a@b.co",
        WIDTH,
        &default()
    ));
}

/// What alt text is made of: breaks, nested images, deleted text, HTML.
#[test]
fn image_alt_text_made_of_other_nodes() {
    assert_snapshot!(pinned_all(
        &[
            "![a\\\nb](u)",
            "![a  \nb](u)",
            "![a\nb](u)",
            "![a ![b](c) d](u)",
            "![a ~~b~~ c](u)",
            "![a <b>x</b> c](u)",
            "![a [link](l) b](u)",
            "![a[^1]](u)\n\n[^1]: n",
            "[![https://x.y](i)](https://x.y)",
            "[![alt](i)](https://x.y)",
        ],
        WIDTH,
        &RenderOptions::new().with_html(Html::Literal)
    ));
}

/// Bare URLs and emails. comrak and markdown-rs differ at their edges, so
/// the edges are pinned one by one.
#[test]
fn autolink_literals() {
    assert_snapshot!(pinned_all(
        &[
            "see https://example.com/a, and http://example.com. also (https://example.com/b)",
            "see www.example.com and www.example.com/path?q=1.",
            "ftp://example.com/file and sftp://example.com",
            "[ https://example.com and [x www.example.com",
            "[https://example.com] and [www.example.com]",
            "WWW.example.com and Www.example.com",
            "[a]www.example.com and (www.example.com) and *www.example.com*",
            "x https://example.com/a] y and x www.example.com] y",
            "https://example.com/a_b_ and https://example.com/a&amp;b and https://example.com/a&copy;",
            "a.b@c.de and a+b@c-d.com. and a@b and a@b.c_",
            "mailto:a@b.co and xmpp:a@b.co and xmpp:a@b.co/resource",
            "http://localhost and https://x and https://",
        ],
        WIDTH,
        &default()
    ));
}

// =========================================================
// Lists
// =========================================================

#[test]
fn ordered_list_with_paren_delimiter_all_ones() {
    assert_snapshot!(pinned("1) a\n1) b\n1) c", WIDTH, &default()));
}

#[test]
fn three_level_mixed_list_with_code_in_deepest_item() {
    assert_snapshot!(pinned(
        "- top\n  1. middle\n     - deepest\n\n       ```rust\n       let x = 1;\n       ```\n\n     - sibling\n  2. second\n- next",
        WIDTH,
        &default()
    ));
}

// =========================================================
// HTML
// =========================================================

/// Blocks that close, blocks that run to the end of the document, and
/// inline tags that span lines, under both [`Html`] values.
#[test]
fn html_blocks_and_inline_tags() {
    let inputs = [
        "<div>\nblock\n</div>\n\nafter",
        "<div>\nblock\n</div>",
        "<!-- comment -->\n\nafter",
        "<!-- open\ntext\n",
        "<!-- open\n\n\n",
        "<?php open\ntext",
        "<script>\nvar x;\n</script>\n\nafter",
        "- item\n\n  <div>in item</div>\n\n- next",
        "> <div>in quote</div>",
        "<div>\r\nblock\r\n</div>\r\n\r\nafter",
        "inline <span\nclass=\"x\">text</span> here",
        "inline <b>bold</b> and <!-- c --> and <br/>",
        "| a | b |\n|---|---|\n| <b>x</b> | y |",
    ];
    let drop = pinned_all(&inputs, WIDTH, &default());
    let literal = pinned_all(&inputs, WIDTH, &break_and_literal());
    assert_snapshot!(format!(
        "{drop}=== literal, soft breaks as breaks ===\n{literal}"
    ));
}

// =========================================================
// Soft breaks
// =========================================================

#[test]
fn soft_breaks_next_to_other_inline_content() {
    let inputs = [
        "[link](u)\nnext",
        "`code`\nnext",
        "text\n[link](u)\n`code`\n**bold**\nend",
        "# heading with\nsoft break",
        "> quote line\n> second line",
        "- item line\n  second line",
        "| a\nb | c |\n|---|---|\n| d | e |",
        "a  \nhard\\\nbreaks\nsoft",
        "trailing space \nbreak",
        "a\r\nb\rc",
    ];
    let join = pinned_all(&inputs, WIDTH, &default());
    let break_ = pinned_all(&inputs, WIDTH, &break_and_literal());
    assert_snapshot!(format!("{join}=== soft breaks as breaks ===\n{break_}"));
}
