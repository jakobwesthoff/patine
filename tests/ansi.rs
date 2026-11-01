// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! The ANSI output of `render`, as the `patine` binary writes it. Layout
//! and styling are tested on the line model in `rendering.rs`; these
//! tests check that `render` encodes exactly those lines and pin the
//! bytes of a few documents.

use std::io::Write;

use insta::assert_snapshot;
use patine::{
    AnsiColor, Html, QuoteStyle, RenderOptions, SoftBreaks, StyledLine, StyledSpan, TextStyle,
    Underline, ansi, render_lines, render_lines_with,
};

const SHOWCASE_MD: &str = include_str!("fixtures/showcase.md");

// =========================================================
// SGR decoding
// =========================================================

/// One SGR sequence patine writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sgr {
    Bold,
    Dim,
    Italic,
    Underline,
    DoubleUnderline,
    Strikethrough,
    Foreground(u8),
    NormalIntensity,
    NotItalic,
    NotUnderlined,
    NotStrikethrough,
    DefaultForeground,
}

impl Sgr {
    /// Readable name for the snapshots, one per sequence.
    fn tag(self) -> String {
        match self {
            Sgr::Bold => "[bold]".to_string(),
            Sgr::Dim => "[dim]".to_string(),
            Sgr::Italic => "[italic]".to_string(),
            Sgr::Underline => "[underline]".to_string(),
            Sgr::DoubleUnderline => "[double-underline]".to_string(),
            Sgr::Strikethrough => "[strike]".to_string(),
            Sgr::Foreground(index) => format!("[fg{index}]"),
            Sgr::NormalIntensity => "[/intensity]".to_string(),
            Sgr::NotItalic => "[/italic]".to_string(),
            Sgr::NotUnderlined => "[/underline]".to_string(),
            Sgr::NotStrikethrough => "[/strike]".to_string(),
            Sgr::DefaultForeground => "[/color]".to_string(),
        }
    }

    /// Apply this sequence to a terminal's current style.
    fn apply(self, style: &mut TextStyle) {
        match self {
            Sgr::Bold => style.bold = true,
            Sgr::Dim => style.dim = true,
            Sgr::Italic => style.italic = true,
            Sgr::Underline => style.underline = Underline::Single,
            Sgr::DoubleUnderline => style.underline = Underline::Double,
            Sgr::Strikethrough => style.strikethrough = true,
            Sgr::Foreground(index) => style.foreground = Some(palette(index)),
            Sgr::NormalIntensity => (style.bold, style.dim) = (false, false),
            Sgr::NotItalic => style.italic = false,
            Sgr::NotUnderlined => style.underline = Underline::None,
            Sgr::NotStrikethrough => style.strikethrough = false,
            Sgr::DefaultForeground => style.foreground = None,
        }
    }
}

fn palette(index: u8) -> AnsiColor {
    [
        AnsiColor::Black,
        AnsiColor::Red,
        AnsiColor::Green,
        AnsiColor::Yellow,
        AnsiColor::Blue,
        AnsiColor::Magenta,
        AnsiColor::Cyan,
        AnsiColor::White,
    ][usize::from(index)]
}

/// A piece of `render` output: a character or an SGR sequence.
#[derive(Debug)]
enum Token {
    Char(char),
    Sgr(Sgr),
}

/// Split raw output into characters and SGR sequences. Panics on any
/// escape sequence patine is not meant to write, SGR 21 included.
fn decode(raw: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            tokens.push(Token::Char(c));
            continue;
        }
        assert_eq!(chars.next(), Some('['), "only CSI sequences expected");
        let params: String = chars.by_ref().take_while(|&c| c != 'm').collect();
        let sgr = match params.as_str() {
            "1" => Sgr::Bold,
            "2" => Sgr::Dim,
            "3" => Sgr::Italic,
            "4" => Sgr::Underline,
            "4:2" => Sgr::DoubleUnderline,
            "9" => Sgr::Strikethrough,
            "22" => Sgr::NormalIntensity,
            "23" => Sgr::NotItalic,
            "24" => Sgr::NotUnderlined,
            "29" => Sgr::NotStrikethrough,
            "39" => Sgr::DefaultForeground,
            other => match other.strip_prefix("38;5;").map(str::parse::<u8>) {
                Some(Ok(index)) if index <= 7 => Sgr::Foreground(index),
                _ => panic!("unexpected escape sequence {other:?} in {raw:?}"),
            },
        };
        tokens.push(Token::Sgr(sgr));
    }
    tokens
}

fn render_raw(markdown: &str, width: usize) -> String {
    let mut buf = Vec::new();
    patine::render(markdown, &mut buf, width).expect("writing to a Vec cannot fail");
    String::from_utf8(buf).expect("output is UTF-8")
}

/// Raw output with every SGR sequence spelled as its tag.
fn tagged(raw: &str) -> String {
    decode(raw)
        .into_iter()
        .map(|token| match token {
            Token::Char(c) => c.to_string(),
            Token::Sgr(sgr) => sgr.tag(),
        })
        .collect()
}

/// One output line as a terminal shows it: every character with its
/// effective style, and the style still active when the line ends.
struct ScreenLine {
    chars: Vec<(char, TextStyle)>,
    end: TextStyle,
}

/// Replay raw output like a terminal. The style state carries over from
/// one line to the next, as it does on screen. Text after the last
/// newline would be an unterminated line, which `render` never writes.
fn screen_lines(raw: &str) -> Vec<ScreenLine> {
    let mut style = TextStyle::default();
    let mut lines = Vec::new();
    let mut current = Vec::new();
    for token in decode(raw) {
        match token {
            Token::Sgr(sgr) => sgr.apply(&mut style),
            Token::Char('\n') => lines.push(ScreenLine {
                chars: std::mem::take(&mut current),
                end: style,
            }),
            Token::Char(c) => current.push((c, style)),
        }
    }
    assert!(current.is_empty(), "unterminated last line in {raw:?}");
    lines
}

fn styled_chars(line: &StyledLine) -> Vec<(char, TextStyle)> {
    line.spans
        .iter()
        .flat_map(|span| span.text.chars().map(move |c| (c, span.style)))
        .collect()
}

/// Documents that together use every style patine writes.
fn sample_documents() -> Vec<&'static str> {
    vec![
        SHOWCASE_MD,
        "# H1 with a [link](https://example.com) and **bold**",
        "> quote with **bold**, `code` and a [link](https://example.com)\n>\n> > nested",
        "> | col1 | col2 |\n> |------|------|\n> | a    | b    |",
        "- item with ~~a struck phrase that wraps over several lines~~ end",
        "```rust\n// comment\nlet s = \"multi\nline\";\n```\n\n```\nplain code\n```",
    ]
}

// =========================================================
// render encodes the lines of render_lines
// =========================================================

#[test]
fn render_shows_exactly_the_lines_of_render_lines() {
    // Replayed like a terminal, the ANSI output shows the same text in
    // the same style, character by character, as the line model.
    for width in [80, 30] {
        for markdown in sample_documents() {
            let screen = screen_lines(&render_raw(markdown, width));
            let lines = render_lines(markdown, width);
            assert_eq!(screen.len(), lines.len(), "{markdown:?} at {width}");
            for (shown, line) in screen.iter().zip(&lines) {
                assert_eq!(shown.chars, styled_chars(line), "{markdown:?} at {width}");
            }
        }
    }
}

#[test]
fn render_with_shows_exactly_the_lines_of_render_lines_with() {
    let options = RenderOptions::new()
        .with_soft_breaks(SoftBreaks::Break)
        .with_html(Html::Literal);
    let markdown = "> **bold\n> line** <b>tag</b>\n\n<div>\n  block\n</div>";
    let mut buf = Vec::new();
    patine::render_with(markdown, &mut buf, 80, &options).expect("writing to a Vec cannot fail");
    let screen = screen_lines(&String::from_utf8(buf).expect("output is UTF-8"));
    let lines = render_lines_with(markdown, 80, &options);
    assert_eq!(screen.len(), lines.len());
    for (shown, line) in screen.iter().zip(&lines) {
        assert_eq!(shown.chars, styled_chars(line));
        assert_eq!(shown.end, TextStyle::default());
    }
}

#[test]
fn every_line_ends_in_the_default_style() {
    // Each line closes the styles it opens, so nothing leaks into the
    // indent of the next line or into whatever the terminal prints after
    // patine exits.
    for markdown in sample_documents() {
        let raw = render_raw(markdown, 30);
        for line in screen_lines(&raw) {
            assert_eq!(
                line.end,
                TextStyle::default(),
                "a line leaves a style active in {raw:?}"
            );
        }
    }
}

#[test]
fn output_ends_with_exactly_one_newline() {
    assert_eq!(render_raw("", 80), "");
    assert_eq!(render_raw("Hello\n\n\n", 80), "Hello\n");
    assert_eq!(render_raw("# Hello", 80).matches('\n').count(), 1);
}

#[test]
fn bold_and_dim_are_never_switched_off_with_sgr_21() {
    // SGR 21 reads as double underline on many terminals; patine resets
    // intensity with SGR 22. `decode` panics on SGR 21, and the check
    // below states the rule on the raw bytes as well.
    for markdown in sample_documents() {
        let raw = render_raw(markdown, 80);
        assert!(!raw.contains("\x1b[21m"), "{raw:?}");
        decode(&raw);
    }
}

#[test]
fn write_errors_are_reported() {
    struct FailingWriter;
    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("disk full"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let error = patine::render("text", &mut FailingWriter, 80).expect_err("the writer fails");
    assert_eq!(error.to_string(), "write the rendered lines");
    assert_eq!(error.root_cause().to_string(), "disk full");
}

// =========================================================
// ansi encodes lines as render does
// =========================================================

/// Options other than the defaults, for the `_with` functions.
fn other_options() -> RenderOptions {
    RenderOptions::new()
        .with_soft_breaks(SoftBreaks::Break)
        .with_html(Html::Literal)
        .with_quote_style(QuoteStyle::DimBar)
}

/// `lines` with every span's style replaced by `restyle`, as a program
/// that changes the lines between layout and output builds them.
fn restyled(lines: &[StyledLine], restyle: fn(TextStyle) -> TextStyle) -> Vec<StyledLine> {
    lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| StyledSpan::new(span.text.clone(), restyle(span.style)))
                .collect()
        })
        .collect()
}

#[test]
fn render_writes_through_ansi() {
    for width in [80, 30] {
        for markdown in sample_documents() {
            assert_eq!(
                ansi(&render_lines(markdown, width)).to_string(),
                render_raw(markdown, width),
                "{markdown:?} at {width}"
            );

            let options = other_options();
            let mut buf = Vec::new();
            patine::render_with(markdown, &mut buf, width, &options)
                .expect("writing to a Vec cannot fail");
            assert_eq!(
                ansi(&render_lines_with(markdown, width, &options)).to_string(),
                String::from_utf8(buf).expect("output is UTF-8"),
                "{markdown:?} at {width} with options"
            );
        }
    }
}

#[test]
fn ansi_shows_exactly_the_lines() {
    // Replayed like a terminal, the output of `ansi` shows the text of
    // the lines in their styles, character by character, and every line
    // ends in the default style.
    for width in [80, 30] {
        for markdown in sample_documents() {
            for options in [RenderOptions::new(), other_options()] {
                let lines = render_lines_with(markdown, width, &options);
                let screen = screen_lines(&ansi(&lines).to_string());
                assert_eq!(screen.len(), lines.len(), "{markdown:?} at {width}");
                for (shown, line) in screen.iter().zip(&lines) {
                    assert_eq!(shown.chars, styled_chars(line), "{markdown:?} at {width}");
                    assert_eq!(shown.end, TextStyle::default(), "{markdown:?} at {width}");
                }
            }
        }
    }
}

#[test]
fn colorless_lines_keep_every_other_style() {
    let lines = restyled(&render_lines(SHOWCASE_MD, 80), TextStyle::without_colors);
    let screen = screen_lines(&ansi(&lines).to_string());
    assert_eq!(screen.len(), lines.len());
    let mut emphasized = 0;
    for (shown, line) in screen.iter().zip(&lines) {
        for (&(c, style), (expected_c, expected_style)) in
            shown.chars.iter().zip(styled_chars(line))
        {
            assert_eq!(c, expected_c);
            assert_eq!(style, expected_style.without_colors());
            if style.bold || style.dim {
                emphasized += 1;
            }
        }
        assert_eq!(shown.chars.len(), styled_chars(line).len());
    }
    assert!(emphasized > 0, "the showcase has bold or dim text");
}

#[test]
fn plain_lines_write_no_escape_sequence() {
    let lines = render_lines(SHOWCASE_MD, 80);
    let plain = ansi(&restyled(&lines, |_| TextStyle::default())).to_string();
    assert!(!plain.contains('\x1b'), "{plain:?}");
    let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
    assert_eq!(plain, text);
}

// =========================================================
// Bytes of the binary's output
// =========================================================
//
// One tag per escape sequence, so these snapshots pin the exact bytes.

#[test]
fn heading_bytes() {
    assert_snapshot!(tagged(&render_raw(
        "# H1 with a [link](https://example.com) and **bold**",
        80
    )));
}

#[test]
fn blockquote_with_table_bytes() {
    assert_snapshot!(tagged(&render_raw(
        "> quote\n>\n> | col1 | col2 |\n> |------|------|\n> | a    | b    |",
        80
    )));
}

#[test]
fn highlighted_code_bytes() {
    assert_snapshot!(tagged(&render_raw(
        "```rust\n// comment\nlet x: u32 = 42;\n```",
        80
    )));
}

#[test]
fn showcase_document_bytes() {
    assert_snapshot!(tagged(&render_raw(SHOWCASE_MD, 80)));
}
