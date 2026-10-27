// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! CommonMark line endings: `\n`, `\r\n` and a lone `\r`. markdown-rs keeps
//! them in node values as they appear in the source, so every place that
//! turns a value into output lines splits at all three. A `\r` that
//! reached a span would move the terminal's cursor back to the start of
//! the line and let the rest of the line overwrite what came before it.

/// Split `text` at CommonMark line endings. Every line ending separates
/// two pieces, so text that ends in one yields an empty last piece, and
/// empty text yields one empty piece. Running text needs that: the line
/// ending at the end of one text node still breaks the line before the
/// next node.
pub(crate) fn split_line_endings(text: &str) -> impl Iterator<Item = &str> {
	let mut rest = Some(text);
	std::iter::from_fn(move || {
		let current = rest?;
		let Some(end) = current.find(['\n', '\r']) else {
			rest = None;
			return Some(current);
		};
		let ending_width = if current[end..].starts_with("\r\n") { 2 } else { 1 };
		rest = Some(&current[end + ending_width..]);
		Some(&current[..end])
	})
}

/// The lines of verbatim text such as a code block, the way [`str::lines`]
/// counts them but ending at a lone `\r` too: a line ending after the last
/// line adds no empty line, and empty text has no lines.
pub(crate) fn verbatim_lines(text: &str) -> impl Iterator<Item = &str> {
	let without_last_ending =
		text.strip_suffix('\n').map(|rest| rest.strip_suffix('\r').unwrap_or(rest)).or_else(|| text.strip_suffix('\r')).unwrap_or(text);
	(!text.is_empty()).then(|| split_line_endings(without_last_ending)).into_iter().flatten()
}

// =========================================================
// Unit tests
// =========================================================

#[cfg(test)]
mod tests {
	use super::*;

	fn split(text: &str) -> Vec<&str> {
		split_line_endings(text).collect()
	}

	fn verbatim(text: &str) -> Vec<&str> {
		verbatim_lines(text).collect()
	}

	#[test]
	fn split_accepts_every_line_ending() {
		assert_eq!(split("a\nb\r\nc\rd"), ["a", "b", "c", "d"]);
	}

	#[test]
	fn split_keeps_empty_pieces() {
		assert_eq!(split(""), [""]);
		assert_eq!(split("a\n"), ["a", ""]);
		assert_eq!(split("a\r"), ["a", ""]);
		assert_eq!(split("a\r\n"), ["a", ""]);
		assert_eq!(split("\r\n"), ["", ""]);
		assert_eq!(split("\n\r"), ["", "", ""]);
		assert_eq!(split("a\r\rb"), ["a", "", "b"]);
	}

	#[test]
	fn verbatim_lines_accept_every_line_ending() {
		assert_eq!(verbatim("a\nb\r\nc\rd"), ["a", "b", "c", "d"]);
	}

	#[test]
	fn verbatim_lines_drop_one_trailing_line_ending() {
		assert_eq!(verbatim("a\n"), ["a"]);
		assert_eq!(verbatim("a\r\n"), ["a"]);
		assert_eq!(verbatim("a\r"), ["a"]);
		assert_eq!(verbatim("a\n\n"), ["a", ""]);
		assert_eq!(verbatim("a\r\r"), ["a", ""]);
	}

	#[test]
	fn verbatim_lines_of_line_endings_alone_are_empty_lines() {
		assert_eq!(verbatim("\n"), [""]);
		assert_eq!(verbatim("\r\n"), [""]);
		assert_eq!(verbatim("\r"), [""]);
	}

	#[test]
	fn verbatim_lines_of_empty_text_are_none() {
		assert!(verbatim("").is_empty());
	}

	#[test]
	fn verbatim_lines_match_str_lines_without_lone_carriage_returns() {
		for text in ["", "a", "a\n", "a\n\nb", "\n\n", "a\r\nb\r\n", "a\r\n\r\n"] {
			assert_eq!(verbatim(text), text.lines().collect::<Vec<_>>(), "{text:?}");
		}
	}
}
