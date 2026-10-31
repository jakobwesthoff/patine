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
