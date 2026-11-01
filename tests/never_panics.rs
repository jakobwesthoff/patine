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

// =========================================================
// Footnote graphs
// =========================================================

/// The footnote labels the generator picks from. Each is written in a
/// random case, as the parser matches labels without regard to case. A
/// label with a space is no footnote label to the parser, so the pool has
/// none.
const LABELS: [&str; 6] = ["a", "b", "c", "d", "e", "f_1"];

/// How many footnote graphs a run renders unless `PATINE_FUZZ_GRAPHS`
/// says otherwise.
const DEFAULT_GRAPHS: usize = 500;

/// A footnote reference the generator wrote. Its text, `r<id>`, sits
/// right before the reference, so the rendered output shows which number
/// the reference got.
struct Reference {
    id: usize,
    label: usize,
    /// Whether the reference sits in the alt text of an image, which
    /// draws no reference.
    in_alt_text: bool,
}

/// A piece of the document, in the order written. A definition carries
/// `D<id>:` as the first word of its text.
enum Piece {
    Block {
        text: String,
        references: Vec<Reference>,
    },
    Definition {
        id: usize,
        label: usize,
        text: String,
        references: Vec<Reference>,
    },
}

struct GraphBuilder<'a> {
    random: &'a mut Random,
    next_reference: usize,
}

impl GraphBuilder<'_> {
    fn label_spelling(&mut self, label: usize) -> String {
        if self.random.below(2) == 0 {
            LABELS[label].to_string()
        } else {
            LABELS[label].to_uppercase()
        }
    }

    /// One to three inline pieces, each holding a reference: plain, in
    /// link text, in emphasis or in the alt text of an image.
    fn inline(&mut self, references: &mut Vec<Reference>) -> String {
        let pieces = 1 + self.random.below(3);
        let mut text = Vec::new();
        for _ in 0..pieces {
            let id = self.next_reference;
            self.next_reference += 1;
            let label = self.random.below(LABELS.len());
            let spelling = self.label_spelling(label);
            let reference = format!("r{id}[^{spelling}]");
            let kind = self.random.below(4);
            references.push(Reference {
                id,
                label,
                in_alt_text: kind == 3,
            });
            text.push(match kind {
                0 => reference,
                1 => format!("[{reference}](u)"),
                2 => format!("*{reference}*"),
                _ => format!("![{reference}](u.png)"),
            });
        }
        text.join(" ")
    }

    /// A block of the kinds in which a reference has to be found: a
    /// paragraph, the two heading forms, a table, a nested list, a quote,
    /// a callout and a paragraph whose link title spans two lines.
    fn block(&mut self, references: &mut Vec<Reference>, indent: &str) -> String {
        // Half of the blocks hold no reference, so that definitions the
        // body does not reach are common.
        let block = match self.random.below(16) {
            8..=15 => "plain words".to_string(),
            0 => self.inline(references),
            1 => format!("# head {}", self.inline(references)),
            2 => format!("{}\n===", self.inline(references)),
            3 => {
                let header = self.inline(references);
                let cell = self.inline(references);
                format!("| {header} |\n|--|\n| {cell} |")
            }
            4 => {
                let item = self.inline(references);
                let nested = self.inline(references);
                format!("- item {item}\n  - nested {nested}")
            }
            5 => format!("> quote {}", self.inline(references)),
            6 => {
                let title = self.inline(references);
                let body = self.inline(references);
                format!("> [!NOTE] title {title}\n> {body}")
            }
            _ => {
                let after = self.inline(references);
                format!("[w](u \"t\nt\") {after}")
            }
        };
        block
            .lines()
            .map(|line| format!("{indent}{line}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A document made of body blocks and footnote definitions in random
/// places. The labels a definition has are a random subset of the pool,
/// some of them twice, and references may point at a label without one.
fn footnote_graph(random: &mut Random) -> Vec<Piece> {
    let defined: Vec<usize> = (0..LABELS.len()).filter(|_| random.below(3) != 0).collect();
    let mut builder = GraphBuilder {
        random,
        next_reference: 0,
    };
    let mut pieces = Vec::new();

    for _ in 0..1 + builder.random.below(4) {
        let mut references = Vec::new();
        let text = builder.block(&mut references, "");
        pieces.push(Piece::Block { text, references });
    }

    let definitions = if defined.is_empty() {
        0
    } else {
        2 + builder.random.below(6)
    };
    for id in 0..definitions {
        let label = defined[builder.random.below(defined.len())];
        let spelling = builder.label_spelling(label);
        let mut references = Vec::new();
        let mut text = format!("[^{spelling}]: D{id}: {}", builder.inline(&mut references));
        if builder.random.below(2) == 0 {
            text.push_str("\n\n");
            text.push_str(&builder.block(&mut references, "    "));
        }
        let at = builder.random.below(pieces.len() + 1);
        pieces.insert(
            at,
            Piece::Definition {
                id,
                label,
                text,
                references,
            },
        );
    }
    pieces
}

fn markdown_of(pieces: &[Piece]) -> String {
    pieces
        .iter()
        .map(|piece| match piece {
            Piece::Block { text, .. } | Piece::Definition { text, .. } => text.as_str(),
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// What the renderer has to draw for a document, worked out from the
/// order the pieces were written in.
struct Expected {
    /// The number each reference shows, by reference id; none when the
    /// reference draws no number.
    numbers: Vec<(usize, Option<usize>)>,
    /// The definitions drawn as notes, with their numbers, in the order
    /// they appear.
    notes: Vec<(usize, usize)>,
    /// Definitions that must not appear at all.
    dropped: Vec<usize>,
    /// Every label a reference of the document is numbered under.
    label_numbers: Vec<(usize, usize)>,
    /// Whether every reference counts and every definition is drawn, so
    /// the parser's own numbers have to agree.
    all_drawn: bool,
}

fn expected(pieces: &[Piece]) -> Expected {
    // The last definition of a label is the one the parser keeps. An
    // earlier one stays where it was written and draws as plain text, so
    // the references in it belong to the body.
    let mut winner: Vec<Option<usize>> = vec![None; LABELS.len()];
    for piece in pieces {
        if let Piece::Definition { id, label, .. } = piece {
            winner[*label] = Some(*id);
        }
    }

    // Each reference with the definition it sits in, when that one is a
    // winner. A reference to a label without a definition is text.
    let mut references: Vec<(&Reference, Option<usize>)> = Vec::new();
    let mut references_of: Vec<(usize, Vec<&Reference>)> = Vec::new();
    let mut losers = 0;
    for piece in pieces {
        match piece {
            Piece::Block {
                references: found, ..
            } => references.extend(found.iter().map(|reference| (reference, None))),
            Piece::Definition {
                id,
                label,
                references: found,
                ..
            } => {
                let owner = (winner[*label] == Some(*id)).then_some(*id);
                if owner.is_none() {
                    losers += 1;
                }
                references.extend(found.iter().map(|reference| (reference, owner)));
                references_of.push((*id, found.iter().collect()));
            }
        }
    }
    let counts =
        |reference: &Reference| !reference.in_alt_text && winner[reference.label].is_some();

    // A definition is drawn when a counted reference of the body, or of a
    // drawn definition, points to it.
    let mut drawn: Vec<usize> = Vec::new();
    let mut pending: Vec<usize> = references
        .iter()
        .filter(|(reference, owner)| owner.is_none() && counts(reference))
        .filter_map(|(reference, _)| winner[reference.label])
        .collect();
    while let Some(id) = pending.pop() {
        if drawn.contains(&id) {
            continue;
        }
        drawn.push(id);
        let (_, inside) = references_of
            .iter()
            .find(|(owner, _)| *owner == id)
            .expect("every definition has a reference list");
        pending.extend(
            inside
                .iter()
                .filter(|reference| counts(reference))
                .filter_map(|reference| winner[reference.label]),
        );
    }

    // Labels count from 1 in the order of their first drawn reference.
    let mut label_numbers: Vec<(usize, usize)> = Vec::new();
    let mut numbers = Vec::new();
    for (reference, owner) in &references {
        let shown = counts(reference) && owner.is_none_or(|id| drawn.contains(&id));
        let number = shown.then(|| {
            let known = label_numbers
                .iter()
                .find(|(label, _)| *label == reference.label);
            match known {
                Some((_, number)) => *number,
                None => {
                    let number = label_numbers.len() + 1;
                    label_numbers.push((reference.label, number));
                    number
                }
            }
        });
        numbers.push((reference.id, number));
    }

    let number_of = |label: usize| {
        label_numbers
            .iter()
            .find(|(known, _)| *known == label)
            .map(|(_, number)| *number)
    };
    let mut notes: Vec<(usize, usize)> = pieces
        .iter()
        .filter_map(|piece| match piece {
            Piece::Definition { id, label, .. } if drawn.contains(id) => Some((
                number_of(*label).expect("a drawn definition has a number"),
                *id,
            )),
            _ => None,
        })
        .map(|(number, id)| (id, number))
        .collect();
    notes.sort_by_key(|(_, number)| *number);

    let dropped = pieces
        .iter()
        .filter_map(|piece| match piece {
            Piece::Definition { id, label, .. }
                if winner[*label] == Some(*id) && !drawn.contains(id) =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    let all_drawn = dropped.is_empty()
        && losers == 0
        && references.iter().all(|(reference, _)| counts(reference));
    Expected {
        numbers,
        notes,
        dropped,
        label_numbers,
        all_drawn,
    }
}

/// The number that follows `r<id>` in `output` in square brackets, if
/// the reference drew one.
fn number_after(output: &str, id: usize) -> Option<usize> {
    let marker = format!("r{id}[");
    let start = output.find(&marker)? + marker.len();
    let digits: String = output[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if output[start + digits.len()..].starts_with(']') {
        digits.parse().ok()
    } else {
        None
    }
}

/// The notes in `lines`, as `(definition id, number)` in the order they
/// appear: the lines that start with `[<number>] D<id>:`.
fn notes_in(lines: &[String]) -> Vec<(usize, usize)> {
    lines
        .iter()
        .filter_map(|line| {
            let rest = line.strip_prefix('[')?;
            let (number, rest) = rest.split_once("] D")?;
            let (id, _) = rest.split_once(':')?;
            Some((id.parse().ok()?, number.parse().ok()?))
        })
        .collect()
}

/// The parser's own numbers by label, for a document in which the
/// renderer has to keep them.
fn parser_numbers(markdown: &str) -> Vec<(usize, usize)> {
    use comrak::nodes::NodeValue;

    // The extensions patine parses with, as `parse_options` in
    // `src/lib.rs` sets them.
    let mut options = comrak::Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.ofm_callouts = true;

    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, markdown, &options);
    let mut numbers: Vec<(usize, usize)> = Vec::new();
    for node in root.descendants() {
        if let NodeValue::FootnoteReference(reference) = &node.data.borrow().value {
            let label = LABELS
                .iter()
                .position(|label| label.eq_ignore_ascii_case(&reference.name))
                .expect("a reference name is a label of the pool");
            let number = usize::try_from(reference.ix).expect("a number fits a usize");
            if !numbers.contains(&(label, number)) {
                numbers.push((label, number));
            }
        }
    }
    numbers.sort_unstable();
    numbers
}

#[test]
fn random_footnote_graphs_draw_what_the_body_reaches() {
    let graphs = std::env::var("PATINE_FUZZ_GRAPHS")
        .ok()
        .and_then(|count| count.parse().ok())
        .unwrap_or(DEFAULT_GRAPHS);
    let started = Instant::now();
    let mut random = Random(SEED);

    for number in 0..graphs {
        let pieces = footnote_graph(&mut random);
        let markdown = markdown_of(&pieces);
        let expected = expected(&pieces);

        // A width no line of the document wraps at, so that every note is
        // one line.
        let lines: Vec<String> = std::panic::catch_unwind(|| {
            render_lines_with(&markdown, 1000, &RenderOptions::default())
                .iter()
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_else(|panic| {
            eprintln!("graph {number} panicked: {markdown:?}");
            std::panic::resume_unwind(panic)
        });
        let output = lines.join("\n");
        let context = format!("graph {number}: {markdown:?} rendered as {output:?}");

        for (id, number) in &expected.numbers {
            assert_eq!(number_after(&output, *id), *number, "r{id} in {context}");
        }
        assert_eq!(notes_in(&lines), expected.notes, "{context}");
        let numbers: Vec<usize> = expected.notes.iter().map(|(_, number)| *number).collect();
        assert_eq!(
            numbers,
            (1..=numbers.len()).collect::<Vec<_>>(),
            "{context}"
        );
        for id in &expected.dropped {
            assert!(!output.contains(&format!("D{id}:")), "D{id} in {context}");
        }

        if expected.all_drawn {
            let mut label_numbers = expected.label_numbers.clone();
            label_numbers.sort_unstable();
            assert_eq!(parser_numbers(&markdown), label_numbers, "{context}");
        }

        assert!(
            started.elapsed() < TIME_LIMIT,
            "{number} graphs took longer than {TIME_LIMIT:?}"
        );
    }
}
