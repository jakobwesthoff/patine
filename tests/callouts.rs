// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Callouts: a quote whose first line is a marker such as `[!NOTE]`, the
//! GitHub alert syntax, or any other `[!type]` comrak's Obsidian-style
//! callouts accept. The snapshots hold each input above the lines it
//! renders to; the direct assertions pin what a snapshot would bury.

mod common;

use common::pinned_all;
use insta::assert_snapshot;
use patine::{
    AnsiColor, Html, RenderOptions, SoftBreaks, StyledLine, TextStyle, render_lines_with,
};

const WIDTH: usize = 80;

fn default() -> RenderOptions {
    RenderOptions::default()
}

/// The plain text of every line of `markdown` at `width`.
fn plain(markdown: &str, width: usize, options: &RenderOptions) -> Vec<String> {
    render_lines_with(markdown, width, options)
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn lines(markdown: &str) -> Vec<StyledLine> {
    render_lines_with(markdown, WIDTH, &default())
}

fn colored(color: Option<AnsiColor>) -> TextStyle {
    let mut style = TextStyle::default();
    style.foreground = color;
    style
}

fn bold_colored(color: Option<AnsiColor>) -> TextStyle {
    let mut style = colored(color);
    style.bold = true;
    style
}

// =========================================================
// The five kinds and the neutral one
// =========================================================

#[test]
fn each_kind_draws_a_heavy_bar_and_a_bold_title_in_its_color() {
    let kinds = [
        ("NOTE", "● Note", Some(AnsiColor::Blue)),
        ("TIP", "◆ Tip", Some(AnsiColor::Green)),
        ("IMPORTANT", "■ Important", Some(AnsiColor::Magenta)),
        ("WARNING", "▲ Warning", Some(AnsiColor::Yellow)),
        ("CAUTION", "✕ Caution", Some(AnsiColor::Red)),
        ("FOO", "○ Foo", None),
    ];
    for (marker, title, color) in kinds {
        let rendered = lines(&format!("> [!{marker}]\n> body"));
        let texts: Vec<String> = rendered.iter().map(ToString::to_string).collect();
        assert_eq!(
            texts,
            [format!("┃ {title}"), "┃ body".to_string()],
            "{marker}"
        );

        let title_line = &rendered[0].spans;
        assert_eq!(title_line[0].text, "┃ ", "{marker}");
        assert_eq!(title_line[0].style, colored(color), "{marker}");
        assert_eq!(title_line[1].text, title, "{marker}");
        assert_eq!(title_line[1].style, bold_colored(color), "{marker}");

        // A neutral bar has the body's plain style, so the two share a
        // span.
        let body_line = &rendered[1].spans;
        assert_eq!(body_line[0].style, colored(color), "{marker}");
        assert_eq!(
            body_line.last().map(|span| span.style),
            Some(TextStyle::default()),
            "{marker}"
        );
    }
}

#[test]
fn the_five_kinds() {
    assert_snapshot!(pinned_all(
        &[
            "> [!NOTE]\n> Useful information.",
            "> [!TIP]\n> Helpful advice.",
            "> [!IMPORTANT]\n> Key information.",
            "> [!WARNING]\n> Urgent info, see `cargo publish`.",
            "> [!CAUTION]\n> Risks of an action.",
        ],
        WIDTH,
        &default()
    ));
}

#[test]
fn a_marker_matches_in_any_letter_case() {
    for marker in ["[!note]", "[!Note]", "[!nOtE]"] {
        assert_eq!(
            plain(&format!("> {marker}\n> body"), WIDTH, &default()),
            ["┃ ● Note", "┃ body"],
            "{marker}"
        );
    }
}

// =========================================================
// What comrak accepts beyond GitHub, and what stays a quote
// =========================================================

#[test]
fn markers_beyond_github() {
    assert_snapshot!(pinned_all(
        &[
            "> [!FOO]\n> b",
            "> [!x]\n> b",
            "> [!my kind]\n> b",
            "> [!Überblick]\n> b",
            "> [!**bold**]\n> b",
            "> [! note ]\n> b",
            "> [!NOTE] Title\n> b",
            "> [!NOTE]\tTitle\n> b",
            "> [!NOTE] Title",
            "> [!NOTE]",
            "> [!NOTE]\n\nafter",
            "> [!NOTE]-\n> b",
            "> [!NOTE]+\n> b",
            "> [!NOTE]- Title\n> b",
            "> [!NOTE|wide]\n> b",
            "> [!NOTE|]\n> b",
            "> [!a|b|c]\n> b",
            "> [!|meta]\n> b",
            "> [! ]\n> b",
            "> [!|]\n> b",
            "> [!-]\n> b",
            "> [!NOTE see [docs] here\n> b",
            "> [!NOTE] - item\n> b",
            ">[!NOTE]\n> b",
            "> [!NOTE]\nlazy line",
            "> [!NOTE]\t\n> b",
        ],
        WIDTH,
        &default()
    ));
}

#[test]
fn github_alerts_that_comrak_leaves_quotes_and_quotes_that_only_look_like_markers() {
    assert_snapshot!(pinned_all(
        &[
            ">  [!NOTE]\n> b",
            ">\t[!NOTE]\n> b",
            "> \\[!NOTE]\n> b",
            "> [!] x\n> b",
            "> [!]\n> b",
            "> [!NOTE\n> b",
            "> [!NOTE x\n> b",
            "> [!NOTE]]\n> b",
            "> [!NOTE]text\n> b",
            "> [!NOTE](https://x.y)\n> b",
            "> text\n> [!NOTE]\n> b",
        ],
        WIDTH,
        &default()
    ));
}

#[test]
fn a_type_whose_default_title_is_blank_draws_the_symbol_alone() {
    for marker in ["[!|meta]", "[! ]", "[!|]", "[!-]"] {
        assert_eq!(
            plain(&format!("> {marker}\n> b"), WIDTH, &default()),
            ["┃ ○", "┃ b"],
            "{marker}"
        );
    }
}

// =========================================================
// Titles
// =========================================================

#[test]
fn a_custom_title_replaces_the_name_and_keeps_its_inline_styles() {
    let rendered = lines("> [!WARNING] Back up *first*\n> body");
    assert_eq!(rendered[0].to_string(), "┃ ▲ Back up first");
    let first = rendered[0]
        .spans
        .iter()
        .find(|span| span.text == "first")
        .expect("the emphasis is a span of its own");
    let mut expected = bold_colored(Some(AnsiColor::Yellow));
    expected.italic = true;
    assert_eq!(first.style, expected);
}

#[test]
fn titles_with_inline_markdown() {
    assert_snapshot!(pinned_all(
        &[
            "> [!TIP] Use *this* and **that**\n> b",
            "> [!TIP] Run `cargo test` first\n> b",
            "> [!TIP] See [the docs](https://example.com)\n> b",
            "> [!TIP] As noted[^1]\n> b\n\n[^1]: The note.",
            "> [!TIP] ![logo](logo.png) inside\n> b",
        ],
        WIDTH,
        &default()
    ));
}

#[test]
fn titles_from_the_blocks_on_the_marker_line() {
    assert_snapshot!(pinned_all(
        &[
            "> [!NOTE] - item\n> b",
            "> [!NOTE] # Head\n> b",
            "> [!NOTE] > quoted\n> b",
            "> [!NOTE] > [!TIP]\n> b",
            "> [!NOTE] > [!TIP] inner\n> b",
            "> [!NOTE]     indented code\n> b",
        ],
        WIDTH,
        &default()
    ));
}

#[test]
fn a_title_that_draws_nothing_falls_back_to_the_default() {
    for markdown in [
        "> [!NOTE] ***\n> b",
        "> [!NOTE] ```\n> b",
        "> [!NOTE] <b></b>\n> b",
        "> [!NOTE] <div>\n> b",
        "> [!NOTE] > [! ]\n> b",
    ] {
        assert_eq!(
            plain(markdown, WIDTH, &default())[0],
            "┃ ● Note",
            "{markdown:?}"
        );
    }
}

#[test]
fn an_image_or_an_empty_link_is_a_title() {
    assert_eq!(
        plain("> [!NOTE] ![](x.png)\n> b", WIDTH, &default())[0],
        "┃ ● [image] (x.png)"
    );
    assert_eq!(
        plain("> [!NOTE] [](https://x.y)\n> b", WIDTH, &default())[0],
        "┃ ● (https://x.y)"
    );
}

#[test]
fn literal_html_is_a_title_on_one_line() {
    let options = RenderOptions::new()
        .with_html(Html::Literal)
        .with_soft_breaks(SoftBreaks::Break);
    assert_eq!(
        plain("> [!NOTE] <b></b>\n> b", WIDTH, &options),
        ["┃ ● <b></b>", "┃ b"]
    );
    assert_eq!(
        plain("> [!NOTE] <div>x\n> body", WIDTH, &options),
        ["┃ ● <div>x", "┃ body"]
    );
}

#[test]
fn a_footnote_definition_on_the_marker_line_leaves_the_default_title() {
    assert_eq!(
        plain("> [!NOTE] [^1]: def\n\nref[^1]", WIDTH, &default()),
        ["┃ ● Note", "", "ref[1]", "", "[1] def"]
    );
}

#[test]
fn a_long_title_wraps_under_its_text() {
    assert_eq!(
        plain(
            "> [!WARNING] Back up the whole database before you run this migration",
            30,
            &default()
        ),
        [
            "┃ ▲ Back up the whole database",
            "┃   before you run this",
            "┃   migration",
        ]
    );
}

#[test]
fn the_symbol_stays_on_the_line_of_the_first_word() {
    assert_eq!(
        plain("> [!NOTE] <span></span> Back up first", 6, &default()),
        ["┃ ● Back", "┃   up", "┃   first"]
    );
    assert_eq!(
        plain("> [!IMPORTANT]\n> Body text", 10, &default())[0],
        "┃ ■ Important"
    );
}

// =========================================================
// Bodies
// =========================================================

#[test]
fn an_empty_callout_is_its_title_line() {
    assert_eq!(plain("> [!NOTE]", WIDTH, &default()), ["┃ ● Note"]);
    assert_eq!(
        plain("> [!NOTE]\n\nafter", WIDTH, &default()),
        ["┃ ● Note", "", "after"]
    );
    assert_eq!(
        plain("before\n\n> [!NOTE]\n\nafter", WIDTH, &default()),
        ["before", "", "┃ ● Note", "", "after"]
    );
}

#[test]
fn bodies_of_several_blocks() {
    assert_snapshot!(pinned_all(
        &[
            "> [!NOTE]\n> One.\n>\n> Two.",
            "> [!TIP]\n> - a\n>   - b\n> - c",
            "> [!TIP]\n> ```rust\n> let x = 1;\n> ```",
            "> [!TIP]\n> | a | b |\n> |---|---|\n> | 1 | 2 |",
            "> [!TIP]\n> # Heading\n> text",
            "para\n\n> [!NOTE]\n> body\n\npara",
        ],
        WIDTH,
        &default()
    ));
}

#[test]
fn nested_callouts() {
    assert_snapshot!(pinned_all(
        &[
            "- item\n\n  > [!TIP]\n  > body",
            "- > [!TIP] T\n  > body",
            "> outer\n>\n> > [!WARNING]\n> > body",
            "> [!NOTE]\n> > [!TIP]\n> > inner",
            "> [!NOTE]\n> outer text\n>\n> > [!CAUTION]\n> > inner",
            "a[^1]\n\n[^1]: note\n\n    > [!NOTE]\n    > body",
        ],
        WIDTH,
        &default()
    ));
}

// =========================================================
// Narrow widths and depth
// =========================================================

#[test]
fn past_the_prefix_limit_the_bar_stops_and_the_title_stays() {
    assert_eq!(
        plain("> > > > [!NOTE]\n> > > > body", 24, &default()),
        ["│ │ │ ● Note", "│ │ │ body"]
    );
}

#[test]
fn a_callout_at_the_depth_cap_keeps_its_bar_and_title() {
    let quotes = "> ".repeat(63);
    let markdown = format!("{quotes}> [!NOTE]\n{quotes}> body");
    let rendered = plain(&markdown, 400, &default());
    assert_eq!(rendered.len(), 2);
    assert_eq!(rendered[0], format!("{}┃ ● Note", "│ ".repeat(63)));
    assert_eq!(rendered[1], format!("{}┃ body", "│ ".repeat(63)));
}

#[test]
fn a_callout_past_the_depth_cap_draws_its_title_flat() {
    let quotes = "> ".repeat(64);
    let markdown = format!("{quotes}> [!NOTE] Title\n{quotes}> body");
    let rendered = plain(&markdown, 400, &default());
    let bars = "│ ".repeat(64);
    assert_eq!(
        rendered,
        [
            format!("{bars}● Title"),
            bars.clone(),
            format!("{bars}body"),
        ]
    );
}

// =========================================================
// Escape sequences
// =========================================================

#[test]
fn callout_bytes() {
    let mut output = Vec::new();
    patine::render("> [!NOTE] A *title*\n> body", &mut output, WIDTH)
        .expect("writing to a Vec does not fail");
    assert_snapshot!(String::from_utf8(output).expect("patine writes UTF-8"));
}
