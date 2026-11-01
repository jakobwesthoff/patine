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

// =========================================================
// Footnotes that only dropped footnotes reference
// =========================================================

#[test]
fn a_footnote_referenced_only_from_an_unreferenced_footnote() {
    renders_without_panic("text\n\n[^z]: sees[^b]\n\n[^b]: B\n\nlater[^c]\n\n[^c]: C\n");
}

#[test]
fn a_chain_of_footnotes_nothing_in_the_body_references() {
    renders_without_panic("[^z]: [^b]\n\n[^b]: [^c]\n\n[^c]: end\n\nx\n");
}

#[test]
fn a_footnote_nested_in_an_unreferenced_footnote() {
    renders_without_panic("[^a]: outer\n\n    [^b]: nested[^c]\n\n[^c]: C\n\ny");
}

#[test]
fn footnotes_referencing_themselves_and_each_other() {
    renders_without_panic("[^a]: me[^a]\n\nx");
    renders_without_panic("[^a]: [^b]\n\n[^b]: [^a]\n\nx");
    renders_without_panic("[^a]: [^b]\n\n[^b]: [^a]\n\nx[^a]");
}

#[test]
fn references_in_headings_tables_and_lists_of_an_unreferenced_footnote() {
    renders_without_panic(
        "x\n\n[^z]: # head[^b]\n\n    | h[^c] |\n    |--|\n    | c[^d] |\n\n    - item [l](u)[^e]\n\n[^b]: B\n\n[^c]: C\n\n[^d]: D\n\n[^e]: E\n",
    );
}

#[test]
fn a_losing_duplicate_footnote_that_references_another() {
    renders_without_panic("x[^a]\n\n[^a]: one[^b]\n\n[^a]: two\n\n[^b]: B\n");
    renders_without_panic("x\n\n[^a]: one[^b]\n\n[^a]: two\n\n[^b]: B\n");
}

#[test]
fn footnotes_inside_quotes_nested_past_the_block_depth_limit() {
    let quotes = "> ".repeat(70);
    renders_without_panic(&format!("{quotes}x[^a] y\n\n[^a]: A[^b]\n\n[^b]: B\n"));
    renders_without_panic(&format!("{quotes}x\n\n[^z]: Z[^a]\n\n[^a]: A\n"));
}

#[test]
fn references_in_image_alt_text() {
    renders_without_panic("![pic[^a]](u)\n\n[^a]: A\n");
    renders_without_panic("x[^z]\n\n[^z]: ![a[^b]](u.png)\n\n[^b]: B");
}
