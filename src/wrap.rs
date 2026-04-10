// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Pure text-wrapping primitives used by the table layout pass.
//!
//! `write_word` in `render.rs` is an online streaming wrap that writes
//! directly to the output and carries per-call state on the `Renderer`.
//! Table cells need the opposite shape: an offline, stateless function
//! that, given a string and a width, returns the list of wrapped lines
//! without touching any output sink. This module provides that shape.
//!
//! The same greedy-fill heuristic exists in both places. Unifying them
//! would require rewriting `write_word` to buffer lines before emitting
//! — explicitly out of scope (see the design discussion in the todo 7
//! plan).

use unicode_width::UnicodeWidthStr;

/// Wrap `text` so that no resulting line exceeds `width` display columns.
///
/// Splits on ASCII whitespace and greedy-fills each line. A single
/// whitespace-delimited token that is wider than `width` is written on
/// its own line and accepts the overflow — tokens are never broken
/// mid-string. This mirrors the philosophy used elsewhere in the
/// renderer: overflow a too-narrow container before making content
/// unreadable (see the long-URL-on-its-own-line behavior in
/// `write_dimmed_url_suffix`).
///
/// Returns at least one line. An empty or whitespace-only input yields
/// `vec![String::new()]` so that callers laying out fixed-height rows
/// can treat every cell as having ≥1 line without special-casing.
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    // Defensive: never divide the world into 0-wide slots. A width of 0
    // would cause every token to "not fit" (since any non-empty token
    // has width ≥ 1), which reduces to the `.max(1)` floor we already
    // apply to `effective_width()` elsewhere.
    let width = width.max(1);

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width: usize = 0;

    for token in text.split_whitespace() {
        let token_width = UnicodeWidthStr::width(token);

        if current.is_empty() {
            // First token on a line — always write it, even if it
            // itself exceeds `width`. Overflow is preferred over
            // breaking the token mid-string.
            current.push_str(token);
            current_width = token_width;
        } else if current_width + 1 + token_width <= width {
            // Fits alongside the previous tokens: separate by one
            // space and extend the running width.
            current.push(' ');
            current.push_str(token);
            current_width += 1 + token_width;
        } else {
            // Doesn't fit — flush the current line and start a new one
            // with this token as the seed.
            lines.push(std::mem::take(&mut current));
            current.push_str(token);
            current_width = token_width;
        }
    }

    // Flush whatever is still accumulating. Empty-input or whitespace-
    // only input falls through here with `current == ""`, producing a
    // single empty line as documented.
    lines.push(current);
    lines
}

/// Display width of the widest whitespace-delimited token in `text`.
///
/// Used by the table layout pass to compute each column's minimum
/// width: a column can't wrap narrower than its widest unbreakable
/// token (otherwise wrapping would either overflow unhelpfully or
/// produce a column where every line overflows). Returns 0 for empty
/// or whitespace-only input.
pub fn widest_word(text: &str) -> usize {
    text.split_whitespace()
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0)
}

// =========================================================
// Unit tests
// =========================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── wrap_text ────────────────────────────────────────────────────────

    #[test]
    fn wrap_empty_string_yields_single_empty_line() {
        assert_eq!(wrap_text("", 10), vec![String::new()]);
    }

    #[test]
    fn wrap_whitespace_only_yields_single_empty_line() {
        assert_eq!(wrap_text("   \t  ", 10), vec![String::new()]);
    }

    #[test]
    fn wrap_short_text_fits_in_one_line() {
        assert_eq!(wrap_text("hello world", 20), vec!["hello world"]);
    }

    #[test]
    fn wrap_long_text_greedy_fills_lines() {
        // "one two three four five" at width 9:
        //   "one two" = 7, + " three" would be 13 → wrap.
        //   "three" = 5, + " four" would be 10 → wrap.
        //   "four" = 4, + " five" would be 9 → fits.
        assert_eq!(
            wrap_text("one two three four five", 9),
            vec!["one two", "three", "four five"],
        );
    }

    #[test]
    fn wrap_single_word_wider_than_width_overflows_its_own_line() {
        // "antidisestablishmentarianism" is 28 wide; width is 10.
        // We do not break the word — it gets its own overflowing line.
        assert_eq!(
            wrap_text("antidisestablishmentarianism", 10),
            vec!["antidisestablishmentarianism"],
        );
    }

    #[test]
    fn wrap_mixes_wide_and_narrow_words() {
        // The wide word overflows on its own line; surrounding short
        // words pack greedily around it.
        assert_eq!(
            wrap_text("a b incomprehensibilities c d", 6),
            vec!["a b", "incomprehensibilities", "c d"],
        );
    }

    #[test]
    fn wrap_unicode_width_respects_display_columns_not_bytes() {
        // Each CJK character has display width 2. "日本語" is 6 columns
        // wide but 9 bytes. At width 4 it does not fit with another
        // token alongside it.
        assert_eq!(
            wrap_text("日本語 ok", 4),
            vec!["日本語", "ok"],
        );
    }

    #[test]
    fn wrap_zero_width_is_floored_and_does_not_panic() {
        // A defensive call with width = 0 should behave like width = 1:
        // every token ends up on its own line but nothing panics.
        assert_eq!(
            wrap_text("a bb ccc", 0),
            vec!["a", "bb", "ccc"],
        );
    }

    // ── widest_word ──────────────────────────────────────────────────────

    #[test]
    fn widest_word_empty_is_zero() {
        assert_eq!(widest_word(""), 0);
    }

    #[test]
    fn widest_word_whitespace_only_is_zero() {
        assert_eq!(widest_word("   \t \n "), 0);
    }

    #[test]
    fn widest_word_returns_longest_token() {
        assert_eq!(widest_word("hi there everyone"), 8);
    }

    #[test]
    fn widest_word_uses_display_width_for_unicode() {
        // "日本語" = 6 columns, "hi" = 2.
        assert_eq!(widest_word("hi 日本語"), 6);
    }
}
