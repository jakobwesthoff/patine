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
fn lists_nested_2000_deep_render_the_50_levels_the_parser_reads_and_all_their_text() {
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
    // The parser opens a list only below fewer than 100 ancestors, a list
    // and its item being two. A deeper marker is text of the item above,
    // which the renderer keeps, so every item shows but 50 bullets.
    assert_eq!(text.matches('•').count(), 50);
}

#[test]
fn a_list_nested_deep_in_a_footnote_renders_the_49_levels_the_parser_reads_and_all_their_text() {
    // The footnote is one of the 100 ancestors the parser allows, so 49
    // lists fit below it.
    let list: String = (0..100)
        .map(|level| format!("    {}- item {level}\n", "  ".repeat(level)))
        .collect();
    let lines = render_on_a_small_stack(format!("x[^a]\n\n[^a]: note\n\n{list}"));
    let text = lines
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for level in 0..100 {
        assert!(text.contains(&format!("item {level}")), "item {level}");
    }
    assert_eq!(text.matches('•').count(), 49);
}

/// Block quotes and list items that nest past the renderer's 64 block
/// containers render in document order without a bullet of their own. The
/// parser still reads the list here: 60 quotes and 10 lists are 80
/// ancestors.
#[test]
fn a_list_nested_in_60_quotes_flattens_after_four_bullets() {
    let quotes = "> ".repeat(60);
    let list: String = (0..10)
        .map(|level| format!("{quotes}{}- item {level}\n", "  ".repeat(level)))
        .collect();
    let lines = render_on_a_small_stack(list);
    let text = lines
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for level in 0..10 {
        assert!(text.contains(&format!("item {level}")), "item {level}");
    }
    assert_eq!(text.matches('•').count(), 4);
}

/// The parser opens a list only below fewer than 100 ancestors of any
/// kind, block quotes included. Below 120 quotes the marker is text.
#[test]
fn a_list_marker_under_120_quotes_is_text() {
    let quotes = "> ".repeat(120);
    let lines = render_on_a_small_stack(format!("{quotes}para\n{quotes}\n{quotes}- item\n"));
    let text: String = lines
        .concat()
        .chars()
        .filter(|c| c.is_alphabetic() || *c == '-')
        .collect();
    assert!(text.ends_with("-item"), "{text}");
}

/// Each nested image adds the numbers of the footnote references below it
/// to the alt text of the image around it, and nothing else. The outermost
/// image shows the one number there is.
#[test]
fn images_nested_20000_deep_show_the_number_of_the_reference_in_the_innermost() {
    let levels = 20_000;
    let markdown = format!(
        "{}a[^x]{}\n\n[^x]: X\n",
        "![".repeat(levels),
        "](u)".repeat(levels)
    );
    let lines = render_on_a_small_stack(markdown);
    assert_eq!(lines, ["[image: [1]] (u)", "", "[1] X"]);
}

#[test]
fn callouts_nested_2000_deep_render_with_their_titles() {
    let markdown: String = (1..=2000)
        .map(|level| format!("{}[!TIP] level {level}\n", "> ".repeat(level)))
        .collect();
    let lines = render_on_a_small_stack(markdown);
    let text = lines.join(" ");
    for level in 1..=2000 {
        assert!(text.contains(&format!("◆ level {level}")), "level {level}");
    }
}

/// Each marker after the first starts a callout inside the title of the
/// one before it, so the titles nest 100,000 deep in one line. The
/// innermost callout has no title, and its default one becomes the title
/// of the outermost.
#[test]
fn callouts_in_titles_100000_deep_render_as_one_title_line() {
    let lines = render_on_a_small_stack("> [!NOTE] ".repeat(100_000));
    assert_eq!(lines, ["┃ ● Note"]);
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
