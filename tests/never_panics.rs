// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Rendering documents made of random Markdown fragments never panics.
//! The generator is seeded, so a run always renders the same documents.
//! `PATINE_FUZZ_DOCS` sets how many documents a run renders.

use std::time::{Duration, Instant};

use patine::{Html, RenderOptions, SoftBreaks, render_lines_with};

const SEED: u64 = 0x9e37_79b9_7f4a_7c15;

/// The number of documents a run renders unless `PATINE_FUZZ_DOCS` says
/// otherwise.
const DEFAULT_DOCUMENTS: usize = 3000;

/// The longest a run may take.
const TIME_LIMIT: Duration = Duration::from_secs(30);

const WIDTHS: [usize; 3] = [1, 12, 80];

/// Pieces of Markdown that start, continue or break constructs: fences,
/// markers, definitions, front matter fences, list and table syntax,
/// inline syntax and plain words.
const FRAGMENTS: [&str; 64] = [
    "text",
    "word another",
    "# heading",
    "## ",
    "Setext\n===",
    "-",
    "- ",
    "- item",
    "* ",
    "+ ",
    "1. ",
    "1) ",
    "10. item",
    "- [ ] ",
    "- [x]",
    "> ",
    "> quote",
    ">",
    "---",
    "+++",
    "***",
    "===",
    "~~~",
    "~~~rust",
    "```",
    "```rust\nfn main() {}\n```",
    "    indented",
    "\t",
    "|",
    "| a | b |",
    "|---|---|",
    "|:-:|--:|",
    "| 1 | 2 | 3 |",
    "[a]:b",
    "[a]: /url \"title\"",
    "[a]",
    "[a][b]",
    "[a](u)",
    "![img](u)",
    "![img][a]",
    "[^a]:",
    "[^a]: note",
    "[^a]",
    "*",
    "**",
    "_",
    "~~",
    "`",
    "``",
    "\\",
    "&amp;",
    "&#x41;",
    "<b>",
    "</b>",
    "<div>",
    "<!--",
    "-->",
    "<https://example.com>",
    "https://example.com",
    "www.example.com",
    "a@b.co",
    "日本語",
    "👩‍👩‍👧‍👦",
    "  ",
];

/// xorshift64: small, and the same on every platform.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % bound as u64).expect("a value below a usize fits a usize")
    }
}

fn document(random: &mut Random) -> String {
    let separators = ["\n", "\r\n", "\r", "", " ", "\n\n"];
    let pieces = 1 + random.below(12);
    let mut document = String::new();
    for _ in 0..pieces {
        document.push_str(FRAGMENTS[random.below(FRAGMENTS.len())]);
        document.push_str(separators[random.below(separators.len())]);
    }
    document
}

#[test]
fn random_documents_render_without_a_panic() {
    let documents = std::env::var("PATINE_FUZZ_DOCS")
        .ok()
        .and_then(|count| count.parse().ok())
        .unwrap_or(DEFAULT_DOCUMENTS);
    let option_sets = [
        RenderOptions::default(),
        RenderOptions::new()
            .with_soft_breaks(SoftBreaks::Break)
            .with_html(Html::Literal),
    ];
    let started = Instant::now();
    let mut random = Random(SEED);

    for number in 0..documents {
        let markdown = document(&mut random);
        for options in &option_sets {
            for width in WIDTHS {
                let lines =
                    std::panic::catch_unwind(|| render_lines_with(&markdown, width, options))
                        .unwrap_or_else(|panic| {
                            eprintln!("document {number} at width {width} panicked: {markdown:?}");
                            std::panic::resume_unwind(panic)
                        });
                for line in &lines {
                    for span in line {
                        assert!(
                            !span.text.contains(['\n', '\r']),
                            "{:?} in document {number} at width {width}: {markdown:?}",
                            span.text
                        );
                    }
                }
            }
        }
        assert!(
            started.elapsed() < TIME_LIMIT,
            "{number} documents took longer than {TIME_LIMIT:?}"
        );
    }
}
