// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Transliteration to ASCII for the `ascii` render option.
//!
//! [`asciify`] replaces non-ASCII characters with ASCII equivalents:
//! accented Latin letters decompose to their base letters (`é` → `e`),
//! common symbols map to ASCII (`…` → `.`, `─` → `-`), and anything
//! without an equivalent becomes `?`, one per display column, so layout
//! stays put.

use unicode_width::UnicodeWidthChar;

fn decompose_nfkd(c: char) -> String {
	match c {
		// Common Latin letters with diacritics: strip the accent and keep the base letter.
		'a' | 'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' | 'ǎ' | 'ȁ' => "a".to_string(),
		'A' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'Ā' | 'Ă' | 'Ą' | 'Ǎ' | 'Ȁ' => "A".to_string(),
		'e' | 'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e".to_string(),
		'E' | 'È' | 'É' | 'Ê' | 'Ë' | 'Ē' | 'Ĕ' | 'Ė' | 'Ę' | 'Ě' => "E".to_string(),
		'i' | 'ì' | 'í' | 'î' | 'ï' | 'ī' | 'ĭ' | 'į' | 'ı' | 'ǐ' => "i".to_string(),
		'I' | 'Ì' | 'Í' | 'Î' | 'Ï' | 'Ī' | 'Ĭ' | 'Į' | 'İ' | 'Ǐ' => "I".to_string(),
		'o' | 'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' | 'ǒ' => "o".to_string(),
		'O' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' | 'Ō' | 'Ŏ' | 'Ő' | 'Ǒ' => "O".to_string(),
		'u' | 'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ŭ' | 'ų' | 'ů' | 'ű' | 'ǔ' => "u".to_string(),
		'U' | 'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ū' | 'Ŭ' | 'Ų' | 'Ů' | 'Ű' | 'Ǔ' => "U".to_string(),
		'y' | 'ý' | 'ÿ' => "y".to_string(),
		'Y' | 'Ý' | 'Ÿ' => "Y".to_string(),
		'c' | 'ç' => "c".to_string(),
		'C' | 'Ç' => "C".to_string(),
		'n' | 'ñ' => "n".to_string(),
		'N' | 'Ñ' => "N".to_string(),
		's' | 'ß' => "s".to_string(),
		'S' | 'ẞ' => "S".to_string(),
		'z' | 'ž' | 'ź' | 'ż' => "z".to_string(),
		'Z' | 'Ž' | 'Ź' | 'Ż' => "Z".to_string(),
		'ﬁ' => "fi".to_string(),
		'ﬂ' => "fl".to_string(),
		_ => c.to_string(),
	}
}

use crate::styled::StyledLine;

/// Rewrite `line` in place: every span's text is passed through
/// [`asciify`], and adjacent spans that now share a style are merged
/// again (the [`StyledLine`] invariant).
pub(crate) fn asciify_line(line: &mut StyledLine) {
	let old = std::mem::take(&mut line.spans);
	for span in old {
		let text = asciify(&span.text);
		line.push_str(&text, span.style);
	}
}

/// Replace every non-ASCII character in `text` with an ASCII equivalent.
pub(crate) fn asciify(text: &str) -> String {
	if text.is_ascii() {
		return text.to_string();
	}
	let mut out = String::with_capacity(text.len());
	for c in text.chars() {
		if c.is_ascii() {
			out.push(c);
			continue;
		}
		if let Some(mapped) = direct(c) {
			out.push_str(mapped);
			continue;
		}
		// NFKD-like decomposition for the common Latin accents and ligatures
		// used by the app. This avoids depending on an external Unicode
		// normalization crate while still keeping the display width stable.
		let decomposed = decompose_nfkd(c);
		if !decomposed.is_empty() && decomposed.is_ascii() {
			out.push_str(&decomposed);
			continue;
		}
		// No ASCII form: one `?` per display column so wide characters
		// (CJK, emoji) do not shift the layout. Zero-width characters
		// (variation selectors, stray combining marks) vanish.
		for _ in 0..UnicodeWidthChar::width(c).unwrap_or(0) {
			out.push('?');
		}
	}
	out
}

/// ASCII replacements for characters that do not decompose to ASCII under
/// NFKD, plus common typographic and structural symbols.
fn direct(c: char) -> Option<&'static str> {
	Some(match c {
		// Letters with no canonical decomposition
		'ß' => "ss",
		'ẞ' => "SS",
		'æ' => "ae",
		'Æ' => "AE",
		'œ' => "oe",
		'Œ' => "OE",
		'ø' => "o",
		'Ø' => "O",
		'ł' => "l",
		'Ł' => "L",
		'đ' | 'ð' => "d",
		'Đ' | 'Ð' => "D",
		'þ' => "th",
		'Þ' => "TH",
		'ı' => "i",
		// Typographic symbols
		'…' => ".",
		'—' | '–' | '─' | '━' => "-",
		'“' | '”' | '„' => "\"",
		'‘' | '’' | '‚' => "'",
		'«' | '»' => "\"",
		// Space variants
		'\u{00A0}' | '\u{2007}' | '\u{2009}' | '\u{200A}' | '\u{202F}' => " ",
		// Box-drawing and symbols that may reach text content
		'│' | '┃' => "|",
		'┌' | '┐' | '└' | '┘' | '├' | '┤' | '┬' | '┴' | '┼' => "+",
		'┏' | '┓' | '┗' | '┛' | '┣' | '┫' | '┳' | '┻' | '╋' => "+",
		'•' => "-",
		'☐' => "O",
		'☑' => "X",
		_ => return None,
	})
}

// =========================================================
// Tests
// =========================================================

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ascii_passes_through_unchanged() {
		assert_eq!(asciify("hello, world! 123"), "hello, world! 123");
	}

	#[test]
	fn accented_latin_decomposes_to_base_letters() {
		assert_eq!(asciify("café"), "cafe");
		assert_eq!(asciify("naïve"), "naive");
		assert_eq!(asciify("Über"), "Uber");
		assert_eq!(asciify("Señor"), "Senor");
		assert_eq!(asciify("Ça va"), "Ca va");
	}

	#[test]
	fn ligatures_and_eszett_expand() {
		assert_eq!(asciify("ﬁle"), "file");
		assert_eq!(asciify("Straße"), "Strasse");
		assert_eq!(asciify("Ægir"), "AEgir");
		assert_eq!(asciify("cœur"), "coeur");
	}

	#[test]
	fn symbols_map_to_ascii() {
		assert_eq!(asciify("a…b"), "a.b");
		assert_eq!(asciify("a—b"), "a-b");
		assert_eq!(asciify("“quoted”"), "\"quoted\"");
		assert_eq!(asciify("─"), "-");
		assert_eq!(asciify("• item"), "- item");
	}

	#[test]
	fn non_latin_becomes_one_question_mark_per_column() {
		assert_eq!(asciify("中文"), "????");
		assert_eq!(asciify("a中b"), "a??b");
	}

	#[test]
	fn combining_marks_alone_are_dropped() {
		// A bare combining acute accent carries no visible width.
		assert_eq!(asciify("e\u{0301}"), "e");
		assert_eq!(asciify("\u{0301}"), "");
	}

	#[test]
	fn asciify_line_preserves_style_and_merges_adjacent_spans() {
		use crate::styled::TextStyle;
		let bold = TextStyle { bold: true, ..TextStyle::default() };
		let mut line = StyledLine::default();
		line.push_str("café", bold);
		line.push_str(" au lait", TextStyle::default());
		asciify_line(&mut line);
		assert_eq!(line.to_string(), "cafe au lait");
		assert_eq!(line.spans.len(), 2);
		assert_eq!(line.spans[0].style, bold);
		assert_eq!(line.spans[0].text, "cafe");
	}
}
