// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Whole documents rendered under the variants that change the output:
//! the width, the render options, the line endings and the ANSI encoding.
//! The input of each snapshot is the fixture of the same name in
//! `tests/fixtures/documents/`. A document that renders differently under
//! a variant shows it in that variant's snapshot only.

mod common;

use common::serialize;
use insta::assert_snapshot;
use patine::{Html, QuoteStyle, RenderOptions, SoftBreaks, render, render_lines_with};

const WIDTHS: [usize; 9] = [80, 40, 20, 12, 8, 5, 3, 1, 0];

// =========================================================
// Variants
// =========================================================

/// The text of a document with every line ending `\n`, whatever form the
/// checkout gave the fixture.
fn normalized(markdown: &str) -> String {
    markdown.replace("\r\n", "\n").replace('\r', "\n")
}

fn with_crlf(markdown: &str) -> String {
    normalized(markdown).replace('\n', "\r\n")
}

fn with_cr(markdown: &str) -> String {
    normalized(markdown).replace('\n', "\r")
}

fn break_and_literal() -> RenderOptions {
    RenderOptions::new()
        .with_soft_breaks(SoftBreaks::Break)
        .with_html(Html::Literal)
}

/// A line of text that names the variant above its output.
fn titled(title: &str, body: &str) -> String {
    format!("=== {title}\n{body}")
}

/// The text of every rendered line without styling, one per line. The
/// styles are pinned by the `widths` snapshots.
fn plain(markdown: &str, width: usize, options: &RenderOptions) -> String {
    render_lines_with(markdown, width, options)
        .iter()
        .map(|line| format!("{line}\n"))
        .collect()
}

/// The bytes `render` writes, with the escape character spelled `\e`.
fn ansi(markdown: &str, width: usize, options: &RenderOptions) -> String {
    let mut bytes = Vec::new();
    patine::render_with(markdown, &mut bytes, width, options).expect("a Vec accepts every write");
    String::from_utf8(bytes)
        .expect("patine writes UTF-8")
        .replace('\x1b', "\\e")
}

// =========================================================
// The four snapshots of a document
// =========================================================

fn widths(markdown: &str) -> String {
    WIDTHS
        .iter()
        .map(|&width| {
            let lines = render_lines_with(markdown, width, &RenderOptions::default());
            titled(&format!("width {width}"), &serialize(&lines))
        })
        .collect()
}

fn options(markdown: &str) -> String {
    let mut out = String::new();
    // Soft breaks, HTML and the first line width cross at width 24; the
    // quote style varies for the two extreme combinations only.
    for soft_breaks in [SoftBreaks::Join, SoftBreaks::Break] {
        for html in [Html::Drop, Html::Literal] {
            for first_line_width in [None, Some(9)] {
                let mut options = RenderOptions::new()
                    .with_soft_breaks(soft_breaks)
                    .with_html(html);
                if let Some(first) = first_line_width {
                    options = options.with_first_line_width(first);
                }
                let title =
                    format!("width 24, {soft_breaks:?}, {html:?}, first line {first_line_width:?}");
                out.push_str(&titled(&title, &plain(markdown, 24, &options)));
            }
        }
    }
    for (soft_breaks, html) in [
        (SoftBreaks::Join, Html::Drop),
        (SoftBreaks::Break, Html::Literal),
    ] {
        let options = RenderOptions::new()
            .with_soft_breaks(soft_breaks)
            .with_html(html)
            .with_quote_style(QuoteStyle::DimBar);
        let title = format!(
            "width 24, {soft_breaks:?}, {html:?}, {:?}",
            options.quote_style
        );
        out.push_str(&titled(&title, &plain(markdown, 24, &options)));
    }
    for (soft_breaks, html) in [
        (SoftBreaks::Join, Html::Drop),
        (SoftBreaks::Break, Html::Literal),
    ] {
        let options = RenderOptions::new()
            .with_soft_breaks(soft_breaks)
            .with_html(html);
        let title = format!("width 80, {soft_breaks:?}, {html:?}");
        out.push_str(&titled(&title, &plain(markdown, 80, &options)));
    }
    out
}

fn line_endings(markdown: &str) -> String {
    let mut out = String::new();
    for (name, text) in [("crlf", with_crlf(markdown)), ("cr", with_cr(markdown))] {
        for width in [80, 12] {
            for (options_name, options) in [
                ("default", RenderOptions::default()),
                ("break and literal", break_and_literal()),
            ] {
                let title = format!("{name}, width {width}, {options_name}");
                let rendered = plain(&text, width, &options);
                assert_eq!(
                    rendered,
                    plain(markdown, width, &options),
                    "{title}: the line endings change the output"
                );
                out.push_str(&titled(&title, &rendered));
            }
        }
    }
    out
}

fn ansi_bytes(markdown: &str) -> String {
    let mut out = String::new();
    for width in [80, 10] {
        for (options_name, options) in [
            ("default", RenderOptions::default()),
            ("break and literal", break_and_literal()),
        ] {
            let title = format!("width {width}, {options_name}");
            out.push_str(&titled(&title, &ansi(markdown, width, &options)));
        }
    }
    out
}

macro_rules! document {
    ($name:ident) => {
        #[test]
        fn $name() {
            let markdown = normalized(include_str!(concat!(
                "fixtures/documents/",
                stringify!($name),
                ".md"
            )));
            assert_snapshot!(concat!(stringify!($name), "__widths"), widths(&markdown));
            assert_snapshot!(concat!(stringify!($name), "__options"), options(&markdown));
            assert_snapshot!(
                concat!(stringify!($name), "__line_endings"),
                line_endings(&markdown)
            );
            assert_snapshot!(concat!(stringify!($name), "__ansi"), ansi_bytes(&markdown));
        }
    };
}

document!(readme);
document!(changelog);
document!(notes);
document!(nested);
document!(frontmatter_yaml);
document!(frontmatter_toml);
document!(html);
document!(wide);
document!(tables);
document!(edge_mix_1);
document!(edge_mix_2);
document!(edge_mix_3);
document!(decided_changes);

// =========================================================
// Invariants that hold for every variant
// =========================================================

/// No span carries a line ending: `render` writes one `\n` per line, and a
/// `\r` in a span would move the terminal's cursor back over the line.
#[test]
fn no_span_holds_a_line_ending_in_any_variant() {
    let documents = [
        include_str!("fixtures/documents/readme.md"),
        include_str!("fixtures/documents/html.md"),
        include_str!("fixtures/documents/frontmatter_toml.md"),
        include_str!("fixtures/documents/edge_mix_1.md"),
        include_str!("fixtures/documents/edge_mix_3.md"),
    ];
    for markdown in documents {
        for text in [normalized(markdown), with_crlf(markdown), with_cr(markdown)] {
            for width in WIDTHS {
                for options in [RenderOptions::default(), break_and_literal()] {
                    for line in render_lines_with(&text, width, &options) {
                        for span in &line {
                            assert!(
                                !span.text.contains(['\n', '\r']),
                                "{:?} at width {width}",
                                span.text
                            );
                        }
                    }
                }
            }
        }
    }
}

/// `render` writes what `render_lines_with` returns: it is the same
/// document, encoded.
#[test]
fn render_writes_one_line_per_rendered_line() {
    let markdown = normalized(include_str!("fixtures/documents/notes.md"));
    let mut bytes = Vec::new();
    render(&markdown, &mut bytes, 40).expect("a Vec accepts every write");
    let written = String::from_utf8(bytes).expect("patine writes UTF-8");
    let lines = render_lines_with(&markdown, 40, &RenderOptions::default());
    assert_eq!(written.matches('\n').count(), lines.len());
}
