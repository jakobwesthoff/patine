// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Documents nested as deep as the parser allows render within the stack
//! of a worker thread. A stack overflow aborts the process, so no test
//! could catch one: each test renders on a thread with the smallest stack
//! a Rust thread asks for by default, 2 MiB.

use patine::{RenderOptions, render_lines_with};

const STACK_SIZE: usize = 2 * 1024 * 1024;

/// The plain text of every line of `markdown`, rendered on a thread with a
/// 2 MiB stack.
fn render_on_a_small_stack(markdown: String) -> Vec<String> {
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            render_lines_with(&markdown, 80, &RenderOptions::default())
                .iter()
                .map(ToString::to_string)
                .collect()
        })
        .expect("the OS starts a thread")
        .join()
        .expect("rendering does not panic")
}

#[test]
fn quotes_nested_100000_deep_render_with_the_bars_that_fit() {
    let lines = render_on_a_small_stack(format!("{}text", "> ".repeat(100_000)));
    assert_eq!(lines.len(), 1);
    // At width 80 the bars stop at 60 columns, which leaves 20 for the text.
    assert_eq!(lines[0].matches('│').count(), 30);
    assert!(lines[0].ends_with("text"), "{}", lines[0]);
}

#[test]
fn lists_nested_2000_deep_render_with_64_bullets_and_all_their_text() {
    let markdown: String = (0..2000)
        .map(|level| format!("{}- item {level}\n", "  ".repeat(level)))
        .collect();
    let lines = render_on_a_small_stack(markdown);
    // The deeper items sit past the width, so their words wrap one by one.
    let text = lines
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for level in 0..2000 {
        assert!(text.contains(&format!("item {level}")), "item {level}");
    }
    assert_eq!(text.matches('•').count(), 64);
}

#[test]
fn emphasis_nested_20000_deep_renders_all_its_text() {
    let lines = render_on_a_small_stack(format!("{}x{}", "*a ".repeat(20_000), "*".repeat(20_000)));
    let words: Vec<&str> = lines.iter().flat_map(|line| line.split(' ')).collect();
    assert_eq!(words.iter().filter(|word| **word == "a").count(), 20_000);
    assert!(words.iter().any(|word| word.starts_with('x')));
}

#[test]
fn emphasis_nested_20000_deep_in_a_link_renders_all_its_text_and_the_url() {
    let lines = render_on_a_small_stack(format!(
        "[{}x{}](https://example.com)",
        "*a ".repeat(20_000),
        "*".repeat(20_000)
    ));
    let words: Vec<&str> = lines.iter().flat_map(|line| line.split(' ')).collect();
    assert_eq!(words.iter().filter(|word| **word == "a").count(), 20_000);
    assert!(
        words.contains(&"(https://example.com)"),
        "the URL suffix follows the link text"
    );
}
