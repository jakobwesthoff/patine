// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Writes styled lines as text with ANSI SGR escape sequences, the output
//! of the `patine` binary and of `patine::render`.
//!
//! Every span switches its attributes on, writes its text and switches
//! them off again, so each span and each line starts from the terminal's
//! default style. Nothing carries over a newline, which keeps line
//! prefixes (indents, blockquote bars) free of the content's style.

use std::io::Write;

use anyhow::{Context, Result};

use crate::styled::{StyledLine, TextStyle, Underline};

/// Write `lines` to `writer`, each followed by a newline.
pub(crate) fn write_lines(lines: &[StyledLine], writer: &mut impl Write) -> Result<()> {
	for line in lines {
		for span in &line.spans {
			write!(writer, "{}{}{}", sgr_on(span.style), span.text, sgr_off(span.style)).context("write styled span")?;
		}
		writeln!(writer).context("write newline")?;
	}
	Ok(())
}

/// Escape sequences that switch on every attribute of `style`, one
/// sequence per attribute.
fn sgr_on(style: TextStyle) -> String {
	let mut out = String::new();
	if style.bold {
		out.push_str("\x1b[1m");
	}
	if style.dim {
		out.push_str("\x1b[2m");
	}
	if style.italic {
		out.push_str("\x1b[3m");
	}
	match style.underline {
		Underline::None => {}
		Underline::Single => out.push_str("\x1b[4m"),
		// The `4:2` sub-parameter form (from Kitty) is the double
		// underline; terminals that do not know it fall back to a single
		// underline.
		Underline::Double => out.push_str("\x1b[4:2m"),
	}
	if style.strikethrough {
		out.push_str("\x1b[9m");
	}
	if let Some(color) = style.foreground {
		out.push_str(&format!("\x1b[38;5;{}m", color.index()));
	}
	out
}

/// Escape sequences that switch off every attribute `sgr_on` switched on,
/// in reverse order.
///
/// Bold and dim share one reset, SGR 22 (normal intensity). The dedicated
/// "not bold" code, SGR 21, means double underline on many terminals.
fn sgr_off(style: TextStyle) -> String {
	let mut out = String::new();
	if style.foreground.is_some() {
		out.push_str("\x1b[39m");
	}
	if style.strikethrough {
		out.push_str("\x1b[29m");
	}
	if style.underline != Underline::None {
		out.push_str("\x1b[24m");
	}
	if style.italic {
		out.push_str("\x1b[23m");
	}
	if style.bold || style.dim {
		out.push_str("\x1b[22m");
	}
	out
}

// =========================================================
// Unit tests
// =========================================================

#[cfg(test)]
mod tests {
	use super::*;
	use crate::styled::AnsiColor;

	fn written(lines: &[StyledLine]) -> String {
		let mut buf = Vec::new();
		write_lines(lines, &mut buf).expect("writing to a Vec cannot fail");
		String::from_utf8(buf).expect("spans hold UTF-8 text")
	}

	fn line(spans: &[(&str, TextStyle)]) -> StyledLine {
		let mut line = StyledLine::default();
		for &(text, style) in spans {
			line.push_str(text, style);
		}
		line
	}

	#[test]
	fn no_lines_write_nothing() {
		assert_eq!(written(&[]), "");
	}

	#[test]
	fn every_line_ends_with_a_newline() {
		let plain = TextStyle::default();
		assert_eq!(written(&[line(&[("one", plain)]), StyledLine::default()]), "one\n\n");
	}

	#[test]
	fn plain_text_carries_no_escape_sequences() {
		assert_eq!(written(&[line(&[("plain", TextStyle::default())])]), "plain\n");
	}

	#[test]
	fn each_attribute_is_switched_on_and_off_around_its_span() {
		let cases = [
			(TextStyle { bold: true, ..TextStyle::default() }, "\x1b[1mx\x1b[22m"),
			(TextStyle { dim: true, ..TextStyle::default() }, "\x1b[2mx\x1b[22m"),
			(TextStyle { italic: true, ..TextStyle::default() }, "\x1b[3mx\x1b[23m"),
			(TextStyle { underline: Underline::Single, ..TextStyle::default() }, "\x1b[4mx\x1b[24m"),
			(TextStyle { underline: Underline::Double, ..TextStyle::default() }, "\x1b[4:2mx\x1b[24m"),
			(TextStyle { strikethrough: true, ..TextStyle::default() }, "\x1b[9mx\x1b[29m"),
			(TextStyle { foreground: Some(AnsiColor::Yellow), ..TextStyle::default() }, "\x1b[38;5;3mx\x1b[39m"),
		];
		for (style, expected) in cases {
			assert_eq!(written(&[line(&[("x", style)])]), format!("{expected}\n"), "{style:?}");
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
		assert_eq!(written(&[line(&[("x", style)])]), "\x1b[1m\x1b[2m\x1b[3m\x1b[4m\x1b[9m\x1b[38;5;5mx\x1b[39m\x1b[29m\x1b[24m\x1b[23m\x1b[22m\n");
	}

	#[test]
	fn each_span_closes_its_style_before_the_next_opens() {
		// Two spans sharing an attribute still switch it off and on
		// again: every span starts from the default style.
		let bold = TextStyle { bold: true, ..TextStyle::default() };
		let bold_italic = TextStyle { italic: true, ..bold };
		assert_eq!(
			written(&[line(&[("a", bold), ("b", bold_italic), ("c", TextStyle::default())])]),
			"\x1b[1ma\x1b[22m\x1b[1m\x1b[3mb\x1b[23m\x1b[22mc\n"
		);
	}

	#[test]
	fn styles_are_reset_before_every_newline() {
		let underline = TextStyle { underline: Underline::Single, ..TextStyle::default() };
		assert_eq!(written(&[line(&[("a", underline)]), line(&[("b", underline)])]), "\x1b[4ma\x1b[24m\n\x1b[4mb\x1b[24m\n");
	}

	#[test]
	fn bold_and_dim_never_use_sgr_21() {
		// SGR 21 reads as double underline on many terminals.
		let style = TextStyle { bold: true, dim: true, ..TextStyle::default() };
		assert!(!written(&[line(&[("x", style)])]).contains("\x1b[21m"));
	}

	#[test]
	fn write_errors_carry_context() {
		struct FailingWriter;
		impl Write for FailingWriter {
			fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
				Err(std::io::Error::other("disk full"))
			}
			fn flush(&mut self) -> std::io::Result<()> {
				Ok(())
			}
		}
		let error = write_lines(&[line(&[("x", TextStyle::default())])], &mut FailingWriter).expect_err("the writer fails");
		assert_eq!(error.to_string(), "write styled span");

		let error = write_lines(&[StyledLine::default()], &mut FailingWriter).expect_err("the writer fails");
		assert_eq!(error.to_string(), "write newline");
	}
}
