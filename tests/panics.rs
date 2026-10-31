// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Inputs that made the previous parser panic. Each is rendered at
//! several widths under every combination of the options that change how
//! the document is read.

use patine::{Html, RenderOptions, SoftBreaks, render_lines_with};

const WIDTHS: [usize; 4] = [0, 1, 8, 80];

/// Render `markdown` under every option combination and width. The lines
/// carry no line ending: `render` ends each with one `\n`, and a `\r` in a
/// span would move the terminal's cursor back over the line.
fn renders_without_panic(markdown: &str) {
    for soft_breaks in [SoftBreaks::Join, SoftBreaks::Break] {
        for html in [Html::Drop, Html::Literal] {
            let options = RenderOptions::new()
                .with_soft_breaks(soft_breaks)
                .with_html(html);
            for width in WIDTHS {
                for line in render_lines_with(markdown, width, &options) {
                    for span in &line {
                        assert!(
                            !span.text.contains(['\n', '\r']),
                            "{:?} at width {width} for {markdown:?}",
                            span.text
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn a_definition_before_a_setext_underline_and_a_carriage_return() {
    renders_without_panic("[a]:b\n=\n=+\r-");
}

#[test]
fn an_unclosed_fence_in_an_ordered_item_before_a_bullet() {
    renders_without_panic("1. ~~~\n*");
}

#[test]
fn an_unclosed_fence_in_a_bullet_item_before_an_ordered_item() {
    renders_without_panic("- ~~~\n1.");
}

#[test]
fn front_matter_followed_by_a_lone_dash() {
    renders_without_panic("---\n\n-");
}

#[test]
fn front_matter_followed_by_a_footnote_definition_without_text() {
    renders_without_panic("---\n\n[^a]:");
}

#[test]
fn toml_front_matter_marker_followed_by_a_list_item_with_spaces() {
    renders_without_panic("+++\n\n- *   ");
}
