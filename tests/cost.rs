// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Rendering costs time and lines in proportion to the size of the input,
//! also for input built to make it cost more.

use std::time::{Duration, Instant};

use patine::{RenderOptions, render_lines_with};

fn render_timed(markdown: &str, width: usize) -> (Vec<String>, Duration) {
    let started = Instant::now();
    let lines = render_lines_with(markdown, width, &RenderOptions::default())
        .iter()
        .map(ToString::to_string)
        .collect();
    (lines, started.elapsed())
}

/// Each level of emphasis wraps two words. A style that is worked out from
/// every enclosing level for every word costs time quadratic in the depth.
/// A debug build renders this in about 0.12 s with the style kept per
/// level and in about 2 s without; the limit is more than ten times the
/// first.
#[test]
fn nested_emphasis_costs_time_in_proportion_to_its_depth() {
    let levels = 20_000;
    let markdown = format!("{}x{}", "*a b ".repeat(levels), "*".repeat(levels));
    let (lines, elapsed) = render_timed(&markdown, 80);
    assert!(!lines.is_empty());
    assert!(
        elapsed < Duration::from_millis(1500),
        "rendering {levels} levels took {elapsed:?}"
    );
}

/// The number of lines and the text on them for `markdown` at `width`.
fn line_count(markdown: &str, width: usize) -> (usize, usize) {
    let lines = render_lines_with(markdown, width, &RenderOptions::default());
    let bytes = lines.iter().map(|line| line.to_string().len()).sum();
    (lines.len(), bytes)
}

/// 70 lists inside each other and a paragraph under them. However deep the
/// lists, a line keeps at least 20 columns of text where the width has them,
/// so a paragraph of short words takes about as many lines as it does
/// without the lists, and not one line per word.
#[test]
fn a_paragraph_under_nested_lists_keeps_room_for_its_text() {
    let words = 20_000;
    let mut markdown: String = (0..70)
        .map(|level| format!("{}- item\n", "  ".repeat(level)))
        .collect();
    markdown.push_str(&format!("\n{}", "  ".repeat(70)));
    markdown.push_str(&"ab ".repeat(words));

    for width in [80, 40] {
        let (lines, _) = line_count(&markdown, width);
        assert!(
            lines < words / 3,
            "{lines} lines for {words} words at width {width}"
        );
    }
    for width in [10, 1] {
        let (lines, _) = line_count(&markdown, width);
        assert!(
            lines < words + 200,
            "{lines} lines for {words} words at width {width}"
        );
    }
}

/// A megabyte of `- > ` is a hundred containers on one line and then a
/// megabyte of words (`-` and `>`) in the innermost one. However deep the
/// containers, the text keeps the columns the width leaves after a prefix
/// of at most 20 less than the width, or 6 columns: 20 columns at widths
/// 80 and 40, 4 at width 10, and the one it has at width 1.
#[test]
fn a_megabyte_of_nested_lists_and_quotes_fills_the_columns_it_has_left() {
    let markdown = "- > ".repeat(262_144);
    for (width, text_columns) in [(80, 20), (40, 20), (10, 4), (1, 1)] {
        let (lines, _) = line_count(&markdown, width);
        let bound = markdown.len() / text_columns * 11 / 10 + 300;
        assert!(lines < bound, "{lines} lines at width {width}");
    }
}
