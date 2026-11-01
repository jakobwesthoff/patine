// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! The encoding of styled lines as text with ANSI SGR escape sequences,
//! public as `patine::ansi`. `patine::render` writes through it, and so
//! does the `patine` binary.
//!
//! Every span switches its attributes on, writes its text and switches
//! them off again, so each span and each line starts from the terminal's
//! default style. Nothing carries over a newline, which keeps line
//! prefixes (indents, blockquote bars) free of the content's style.

use std::fmt;

use crate::styled::{StyledLine, TextStyle, Underline};

/// `lines` as text with ANSI escape sequences, the encoding
/// [`render`](crate::render) writes.
///
/// Each span switches the attributes of its style on, one SGR sequence
/// per attribute, then comes its text, then the same attributes are
/// switched off again in reverse order; every line ends with a newline.
/// So no style carries over a line break or past the end of the output.
/// For the lines of [`render_lines`](crate::render_lines) the bytes are
/// the ones `render` writes.
///
/// The result is a [`fmt::Display`] value: `to_string` or `write!` with
/// [`fmt::Write`] gives a `String`, and `write!` writes to any
/// [`std::io::Write`]. Writing fails only when the destination fails.
///
/// Lines a caller builds are written as given. An empty span gets its
/// on and off sequences around nothing. A span holding a line ending
/// writes that line ending with its style still on, so the style reaches
/// into the next line until the span ends. Text is written as it is:
/// control characters and tabs reach the output unchanged, so a program
/// that shows text it does not trust replaces them first.
///
/// # Examples
///
/// A program that changes the lines between layout and output, here to
/// write them without colors and as plain text:
///
/// ```
/// use std::io::Write as _;
///
/// use patine::{StyledLine, StyledSpan, TextStyle, ansi, render_lines};
///
/// let lines = render_lines("Run `cargo test` **now**.", 80);
///
/// let restyled = |restyle: fn(TextStyle) -> TextStyle| -> Vec<StyledLine> {
///     lines
///         .iter()
///         .map(|line| {
///             line.spans
///                 .iter()
///                 .map(|span| StyledSpan::new(span.text.clone(), restyle(span.style)))
///                 .collect()
///         })
///         .collect()
/// };
///
/// let colorless = ansi(&restyled(TextStyle::without_colors)).to_string();
/// assert_eq!(colorless, "Run cargo test \x1b[1mnow\x1b[22m.\n");
///
/// let mut plain = Vec::new();
/// write!(plain, "{}", ansi(&restyled(|_| TextStyle::default())))?;
/// assert_eq!(plain, b"Run cargo test now.\n");
/// # Ok::<(), std::io::Error>(())
/// ```
#[must_use]
pub fn ansi(lines: &[StyledLine]) -> AnsiLines<'_> {
    AnsiLines { lines }
}

/// Lines encoded with ANSI escape sequences, as [`ansi`] returns them.
///
/// The lines are encoded when the value is formatted, each time it is
/// formatted.
#[derive(Clone, Copy, Debug)]
pub struct AnsiLines<'a> {
    lines: &'a [StyledLine],
}

impl fmt::Display for AnsiLines<'_> {
    // Every error returned here comes from a write into `f`.
    // `io::Write::write_fmt` panics when a `Display` implementation reports
    // an error its destination did not.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in self.lines {
            for span in &line.spans {
                sgr_on(span.style, f)?;
                f.write_str(&span.text)?;
                sgr_off(span.style, f)?;
            }
            f.write_str("\n")?;
        }
        Ok(())
    }
}

/// Write the escape sequences that switch on every attribute of `style`,
/// one sequence per attribute.
fn sgr_on(style: TextStyle, out: &mut impl fmt::Write) -> fmt::Result {
    if style.bold {
        out.write_str("\x1b[1m")?;
    }
    if style.dim {
        out.write_str("\x1b[2m")?;
    }
    if style.italic {
        out.write_str("\x1b[3m")?;
    }
    match style.underline {
        Underline::None => {}
        Underline::Single => out.write_str("\x1b[4m")?,
        // The `4:2` sub-parameter form (from Kitty) is the double
        // underline; terminals that do not know it fall back to a single
        // underline.
        Underline::Double => out.write_str("\x1b[4:2m")?,
    }
    if style.strikethrough {
        out.write_str("\x1b[9m")?;
    }
    if let Some(color) = style.foreground {
        write!(out, "\x1b[38;5;{}m", color.index())?;
    }
    Ok(())
}

/// Write the escape sequences that switch off every attribute `sgr_on`
/// switched on, in reverse order.
///
/// Bold and dim share one reset, SGR 22 (normal intensity). The dedicated
/// "not bold" code, SGR 21, means double underline on many terminals.
fn sgr_off(style: TextStyle, out: &mut impl fmt::Write) -> fmt::Result {
    if style.foreground.is_some() {
        out.write_str("\x1b[39m")?;
    }
    if style.strikethrough {
        out.write_str("\x1b[29m")?;
    }
    if style.underline != Underline::None {
        out.write_str("\x1b[24m")?;
    }
    if style.italic {
        out.write_str("\x1b[23m")?;
    }
    if style.bold || style.dim {
        out.write_str("\x1b[22m")?;
    }
    Ok(())
}

// =========================================================
// Unit tests
// =========================================================

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::styled::{AnsiColor, StyledSpan};

    fn written(lines: &[StyledLine]) -> String {
        ansi(lines).to_string()
    }

    /// A line built through `push_str`, which merges spans of one style
    /// and drops empty text, as patine's renderer does.
    fn line(spans: &[(&str, TextStyle)]) -> StyledLine {
        let mut line = StyledLine::default();
        for &(text, style) in spans {
            line.push_str(text, style);
        }
        line
    }

    /// A line made of exactly these spans, as a caller builds one.
    fn spans(spans: &[(&str, TextStyle)]) -> StyledLine {
        StyledLine::new(
            spans
                .iter()
                .map(|&(text, style)| StyledSpan::new(text, style))
                .collect(),
        )
    }

    const BOLD: TextStyle = TextStyle {
        bold: true,
        italic: false,
        dim: false,
        strikethrough: false,
        underline: Underline::None,
        foreground: None,
    };

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("disk full"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn no_lines_write_nothing() {
        assert_eq!(written(&[]), "");
    }

    #[test]
    fn every_line_ends_with_a_newline() {
        let plain = TextStyle::default();
        assert_eq!(
            written(&[line(&[("one", plain)]), StyledLine::default()]),
            "one\n\n"
        );
    }

    #[test]
    fn plain_text_carries_no_escape_sequences() {
        assert_eq!(
            written(&[line(&[("plain", TextStyle::default())])]),
            "plain\n"
        );
    }

    #[test]
    fn each_attribute_is_switched_on_and_off_around_its_span() {
        let cases = [
            (BOLD, "\x1b[1mx\x1b[22m"),
            (
                TextStyle {
                    dim: true,
                    ..TextStyle::default()
                },
                "\x1b[2mx\x1b[22m",
            ),
            (
                TextStyle {
                    italic: true,
                    ..TextStyle::default()
                },
                "\x1b[3mx\x1b[23m",
            ),
            (
                TextStyle {
                    underline: Underline::Single,
                    ..TextStyle::default()
                },
                "\x1b[4mx\x1b[24m",
            ),
            (
                TextStyle {
                    underline: Underline::Double,
                    ..TextStyle::default()
                },
                "\x1b[4:2mx\x1b[24m",
            ),
            (
                TextStyle {
                    strikethrough: true,
                    ..TextStyle::default()
                },
                "\x1b[9mx\x1b[29m",
            ),
            (
                TextStyle {
                    foreground: Some(AnsiColor::Yellow),
                    ..TextStyle::default()
                },
                "\x1b[38;5;3mx\x1b[39m",
            ),
        ];
        for (style, expected) in cases {
            assert_eq!(
                written(&[line(&[("x", style)])]),
                format!("{expected}\n"),
                "{style:?}"
            );
        }
    }

    #[test]
    fn combined_attributes_open_in_order_and_close_in_reverse() {
        let style = TextStyle {
            bold: true,
            dim: true,
            italic: true,
            strikethrough: true,
            underline: Underline::Single,
            foreground: Some(AnsiColor::Magenta),
        };
        assert_eq!(
            written(&[line(&[("x", style)])]),
            "\x1b[1m\x1b[2m\x1b[3m\x1b[4m\x1b[9m\x1b[38;5;5mx\x1b[39m\x1b[29m\x1b[24m\x1b[23m\x1b[22m\n"
        );
    }

    #[test]
    fn each_span_closes_its_style_before_the_next_opens() {
        // Two spans sharing an attribute still switch it off and on
        // again: every span starts from the default style.
        let bold_italic = TextStyle {
            italic: true,
            ..BOLD
        };
        assert_eq!(
            written(&[line(&[
                ("a", BOLD),
                ("b", bold_italic),
                ("c", TextStyle::default())
            ])]),
            "\x1b[1ma\x1b[22m\x1b[1m\x1b[3mb\x1b[23m\x1b[22mc\n"
        );
    }

    #[test]
    fn styles_are_reset_before_every_newline() {
        let underline = TextStyle {
            underline: Underline::Single,
            ..TextStyle::default()
        };
        assert_eq!(
            written(&[line(&[("a", underline)]), line(&[("b", underline)])]),
            "\x1b[4ma\x1b[24m\n\x1b[4mb\x1b[24m\n"
        );
    }

    #[test]
    fn bold_and_dim_never_use_sgr_21() {
        // SGR 21 reads as double underline on many terminals.
        let style = TextStyle { dim: true, ..BOLD };
        assert!(!written(&[line(&[("x", style)])]).contains("\x1b[21m"));
    }

    // ── Lines a caller builds ───────────────────────────────────────────

    #[test]
    fn an_empty_span_writes_its_on_and_off_sequences() {
        assert_eq!(written(&[spans(&[("", BOLD)])]), "\x1b[1m\x1b[22m\n");
    }

    #[test]
    fn a_line_ending_inside_a_span_is_written_inside_its_style() {
        assert_eq!(
            written(&[spans(&[("a\nb", BOLD)])]),
            "\x1b[1ma\nb\x1b[22m\n"
        );
    }

    #[test]
    fn plain_and_multi_byte_text_pass_through() {
        let plain = TextStyle::default();
        assert_eq!(written(&[spans(&[("", plain)])]), "\n");
        assert_eq!(
            written(&[spans(&[("one ", plain), ("two", plain)])]),
            "one two\n"
        );
        assert_eq!(
            written(&[spans(&[("é 漢 🦀", BOLD)])]),
            "\x1b[1mé 漢 🦀\x1b[22m\n"
        );
    }

    // ── The adapter ─────────────────────────────────────────────────────

    #[test]
    fn formatting_twice_writes_the_same_text() {
        let lines = [line(&[("x", BOLD)])];
        let encoded = ansi(&lines);
        assert_eq!(encoded.to_string(), encoded.to_string());
    }

    #[test]
    fn an_io_writer_gets_the_same_bytes_and_its_own_error() {
        let lines = [line(&[("x", BOLD)]), StyledLine::default()];
        let mut bytes = Vec::new();
        write!(bytes, "{}", ansi(&lines)).expect("writing to a Vec cannot fail");
        assert_eq!(bytes, written(&lines).into_bytes());

        // The span's first write fails, and for a line without spans the
        // newline's.
        for lines in [&lines[..1], &lines[1..]] {
            let error = write!(FailingWriter, "{}", ansi(lines)).expect_err("the writer fails");
            assert_eq!(error.kind(), std::io::ErrorKind::Other);
            assert_eq!(error.to_string(), "disk full");
        }
    }

    #[test]
    fn ansi_lines_is_copy_clone_and_debug() {
        fn assert_traits<T: Copy + Clone + fmt::Debug>(_: T) {}
        assert_traits(ansi(&[]));
    }
}
