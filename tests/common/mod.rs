// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Helpers shared by the test files that pin rendered output as snapshots.

#![allow(dead_code)]

use patine::{AnsiColor, RenderOptions, StyledLine, TextStyle, Underline, render_lines_with};

/// Write lines in a canonical text form: plain spans as their bare text,
/// styled spans as `[attributes]text[]` with the span's complete style,
/// and each line ended by a newline.
pub fn serialize(lines: &[StyledLine]) -> String {
    let mut out = String::new();
    for line in lines {
        for span in line {
            let attributes = attribute_names(span.style);
            if attributes.is_empty() {
                out.push_str(&span.text);
            } else {
                out.push_str(&format!("[{}]{}[]", attributes.join(","), span.text));
            }
        }
        out.push('\n');
    }
    out
}

fn attribute_names(style: TextStyle) -> Vec<&'static str> {
    let mut names = Vec::new();
    if style.bold {
        names.push("bold");
    }
    if style.dim {
        names.push("dim");
    }
    if style.italic {
        names.push("italic");
    }
    match style.underline {
        Underline::None => {}
        Underline::Single => names.push("underline"),
        Underline::Double => names.push("double-underline"),
    }
    if style.strikethrough {
        names.push("strike");
    }
    if let Some(color) = style.foreground {
        names.push(match color {
            AnsiColor::Black => "black",
            AnsiColor::Red => "red",
            AnsiColor::Green => "green",
            AnsiColor::Yellow => "yellow",
            AnsiColor::Blue => "blue",
            AnsiColor::Magenta => "magenta",
            AnsiColor::Cyan => "cyan",
            AnsiColor::White => "white",
        });
    }
    names
}

/// The input as a Rust string literal, then the rendered lines in the
/// canonical form, so a snapshot shows what was rendered next to its result.
pub fn pinned(markdown: &str, width: usize, options: &RenderOptions) -> String {
    format!(
        "{markdown:?}\n{}",
        serialize(&render_lines_with(markdown, width, options))
    )
}

/// [`pinned`] for every input, one after the other, separated by a rule.
pub fn pinned_all(inputs: &[&str], width: usize, options: &RenderOptions) -> String {
    inputs
        .iter()
        .map(|markdown| pinned(markdown, width, options))
        .collect::<Vec<_>>()
        .join("---\n")
}
