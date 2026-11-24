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
const FRAGMENTS: [&str; 72] = [
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
    "&#10;",
    "&#13;",
    "&#x0D;&#x0A;",
    "[a](u&#10;v)",
    "![i](u&#13;v)",
    "[a]: u&#13;&#10;v",
    "[m](mailto:a&#10;b)",
    "<http://x/&#10;>",
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
            references.push(Reference { id, label });
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
/// order the pieces were written in and the rules of the parser: the first
/// definition of a label is the one that counts, and a definition is drawn
/// when a reference in the body, or in a drawn definition, names its
/// label. The footnotes are numbered from 1 in the order of the first
/// drawn reference in the source, a reference in a definition counting
/// where the definition is written.
struct Expected {
    /// The number each reference shows, by reference id; none when the
    /// reference is not drawn.
    numbers: Vec<(usize, Option<usize>)>,
    /// The definitions drawn as notes, with their numbers, in the order
    /// they appear.
    notes: Vec<(usize, usize)>,
    /// Definitions that must not appear at all.
    dropped: Vec<usize>,
    /// The labels of the definitions drawn.
    drawn_labels: Vec<usize>,
    /// Every label that has a number, by number.
    label_numbers: Vec<(usize, usize)>,
    /// Whether no definition holds a reference. The parser numbers the
    /// references of the body before those inside definitions, so its
    /// numbers are the model's only for such a document.
    plain: bool,
}

fn expected(pieces: &[Piece]) -> Expected {
    // The first definition of a label is the one the parser keeps. A later
    // one is detached from the document with everything in it, so the
    // references in it do not exist.
    let mut winner: Vec<Option<usize>> = vec![None; LABELS.len()];
    for piece in pieces {
        if let Piece::Definition { id, label, .. } = piece {
            winner[*label].get_or_insert(*id);
        }
    }

    // Each reference with the definition it sits in, in the order of the
    // source. A reference to a label without a definition is text.
    let mut references: Vec<(&Reference, Option<usize>)> = Vec::new();
    let mut references_of: Vec<(usize, Vec<&Reference>)> = Vec::new();
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
            } if winner[*label] == Some(*id) => {
                references.extend(found.iter().map(|reference| (reference, Some(*id))));
                references_of.push((*id, found.iter().collect()));
            }
            Piece::Definition { .. } => {}
        }
    }
    let counts = |reference: &Reference| winner[reference.label].is_some();

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
            .expect("every kept definition has a reference list");
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
    let mut notes: Vec<(usize, usize)> = Vec::new();
    let mut drawn_labels = Vec::new();
    for piece in pieces {
        if let Piece::Definition { id, label, .. } = piece
            && drawn.contains(id)
        {
            let number = number_of(*label).expect("a drawn definition has a number");
            notes.push((*id, number));
            drawn_labels.push(*label);
        }
    }
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

    let plain = references_of.iter().all(|(_, inside)| inside.is_empty());
    Expected {
        numbers,
        notes,
        dropped,
        drawn_labels,
        label_numbers,
        plain,
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

/// What the parser makes of the footnotes of `markdown`: the number of
/// each label its references carry, and the labels of the definitions it
/// leaves at the end of the document, which are the footnotes it draws.
fn parser_footnotes(markdown: &str) -> (Vec<(usize, usize)>, Vec<usize>) {
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

    let label_of = |name: &str| {
        LABELS
            .iter()
            .position(|label| label.eq_ignore_ascii_case(name))
            .expect("a footnote name is a label of the pool")
    };

    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, markdown, &options);
    let mut numbers: Vec<(usize, usize)> = Vec::new();
    for node in root.descendants() {
        if let NodeValue::FootnoteReference(reference) = &node.data.borrow().value {
            let number = usize::try_from(reference.ix).expect("a number fits a usize");
            let entry = (label_of(&reference.name), number);
            if !numbers.contains(&entry) {
                numbers.push(entry);
            }
        }
    }
    numbers.sort_unstable();
    let mut labels: Vec<usize> = root
        .children()
        .filter_map(|node| match &node.data.borrow().value {
            NodeValue::FootnoteDefinition(definition) => Some(label_of(&definition.name)),
            _ => None,
        })
        .collect();
    labels.sort_unstable();
    (numbers, labels)
}

/// The option sets and widths a graph renders with. At 1000 columns no
/// line of the document wraps, so every note is one line. At 12 the
/// tables fall back to records, and `Break` turns a line ending in a
/// header cell into a line break.
fn renderings() -> [(usize, RenderOptions); 3] {
    [
        (1000, RenderOptions::default()),
        (12, RenderOptions::default()),
        (12, RenderOptions::new().with_soft_breaks(SoftBreaks::Break)),
    ]
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

        // The parser draws the footnotes the model works out from its
        // rules. Its numbers differ from the ones shown by design, as it
        // numbers the references of the body first, and agree with the
        // model's where no definition holds a reference.
        let (parser_numbers, parser_labels) = parser_footnotes(&markdown);
        let mut drawn_labels = expected.drawn_labels.clone();
        drawn_labels.sort_unstable();
        assert_eq!(parser_labels, drawn_labels, "graph {number}: {markdown:?}");
        if expected.plain {
            let mut label_numbers = expected.label_numbers.clone();
            label_numbers.sort_unstable();
            assert_eq!(
                parser_numbers, label_numbers,
                "graph {number}: {markdown:?}"
            );
        }

        for (width, options) in renderings() {
            let lines: Vec<String> = std::panic::catch_unwind(|| {
                render_lines_with(&markdown, width, &options)
                    .iter()
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_else(|panic| {
                eprintln!("graph {number} panicked at width {width}: {markdown:?}");
                std::panic::resume_unwind(panic)
            });
            let output = lines.join("\n");
            let context =
                format!("graph {number} at width {width}: {markdown:?} rendered as {output:?}");

            for (id, number) in &expected.numbers {
                assert_eq!(number_after(&output, *id), *number, "r{id} in {context}");
            }
            assert_eq!(notes_in(&lines), expected.notes, "{context}");
            for id in &expected.dropped {
                assert!(!output.contains(&format!("D{id}:")), "D{id} in {context}");
            }
        }

        assert!(
            started.elapsed() < TIME_LIMIT,
            "{number} graphs took longer than {TIME_LIMIT:?}"
        );
    }
}

// =========================================================
// Footnote shapes without a model
// =========================================================

/// How many documents of shapes the footnote graphs do not write a run
/// renders.
const SHAPED_DOCUMENTS: usize = 1500;

const SHAPE_LABELS: [&str; 5] = ["a", "b", "c", "d", "e"];

const SHAPE_WIDTHS: [usize; 5] = [1, 7, 12, 30, 1000];

/// Writes documents of the shapes the footnote graphs leave out: a
/// definition inside a definition, a list item, a quote, a callout and a
/// callout title, a nested image, and a link in alt text. Every definition
/// starts with `D<id>:`.
struct ShapeBuilder<'a> {
    random: &'a mut Random,
    next_reference: usize,
    next_definition: usize,
}

impl ShapeBuilder<'_> {
    fn reference(&mut self) -> String {
        let id = self.next_reference;
        self.next_reference += 1;
        let label = SHAPE_LABELS[self.random.below(SHAPE_LABELS.len())];
        format!("r{id}[^{label}]")
    }

    fn inline(&mut self, depth: usize) -> String {
        match self.random.below(if depth > 2 { 2 } else { 9 }) {
            0 | 1 => self.reference(),
            2 => format!("[{}](u)", self.inline(depth + 1)),
            3 => format!("*{}*", self.inline(depth + 1)),
            4 => format!("![{}](u.png)", self.inline(depth + 1)),
            5 => format!("![x ![{}](v.png)](u.png)", self.inline(depth + 1)),
            6 => format!(
                "![{} [l {}](u)](u.png)",
                self.inline(depth + 1),
                self.inline(depth + 1)
            ),
            7 => format!("[![{}](i.png)](u)", self.inline(depth + 1)),
            _ => format!("~~{}~~ `c`", self.inline(depth + 1)),
        }
    }

    fn line(&mut self) -> String {
        let count = 1 + self.random.below(3);
        (0..count)
            .map(|_| self.inline(0))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn definition(&mut self, depth: usize) -> String {
        let id = self.next_definition;
        self.next_definition += 1;
        let label = SHAPE_LABELS[self.random.below(SHAPE_LABELS.len())];
        let mut definition = format!("[^{label}]: D{id}: {}", self.line());
        if depth < 3 && self.random.below(2) == 0 {
            definition.push_str("\n\n");
            definition.push_str(&prefixed(&self.block(depth + 1), "    "));
        }
        definition
    }

    fn block(&mut self, depth: usize) -> String {
        match self.random.below(if depth > 3 { 3 } else { 12 }) {
            0 => self.line(),
            1 => "plain".to_string(),
            2 => self.definition(depth),
            3 => format!("# h {}", self.line()),
            4 => format!("| h {} |\n|--|\n| {} |", self.line(), self.line()),
            5 => format!(
                "- {}\n\n{}",
                self.line(),
                prefixed(&self.block(depth + 1), "  ")
            ),
            6 => prefixed(&self.block(depth + 1), "> "),
            7 => format!(
                "> [!NOTE] {}\n{}",
                self.line(),
                prefixed(&self.block(depth + 1), "> ")
            ),
            8 => {
                let title = self.definition(depth + 1);
                let title = title.lines().next().expect("a definition has a line");
                format!("> [!NOTE] {title}\n> {}", self.line())
            }
            9 => format!("- {}", self.definition(depth + 1).replace('\n', "\n  ")),
            10 => format!("{}\n===", self.line()),
            _ => format!(
                "1. {}\n   {}",
                self.line(),
                self.block(depth + 1).replace('\n', "\n   ")
            ),
        }
    }
}

/// `text` with `prefix` in front of every line that has text.
fn prefixed(text: &str, prefix: &str) -> String {
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every number in square brackets in `output` that does not start a note,
/// and the numbers of the notes. A number that wraps to the start of a line
/// is not a note: a note is a line of the form `[n] D<id>:`.
fn shown_and_note_numbers(lines: &[String]) -> (Vec<usize>, Vec<usize>) {
    let mut shown = Vec::new();
    let mut notes = Vec::new();
    for line in lines {
        let note = line.strip_prefix('[').and_then(|rest| {
            let (number, rest) = rest.split_once("] D")?;
            let (id, _) = rest.split_once(':')?;
            id.chars().all(|c| c.is_ascii_digit()).then_some(number)
        });
        let mut rest = line.as_str();
        if let Some(number) = note {
            notes.push(number.parse().expect("a note number is a number"));
            rest = &line[number.len() + 2..];
        }
        for (start, c) in rest.char_indices() {
            if c != '[' {
                continue;
            }
            let digits: String = rest[start + 1..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            if !digits.is_empty() && rest[start + 1 + digits.len()..].starts_with(']') {
                shown.push(digits.parse().expect("digits are a number"));
            }
        }
    }
    shown.sort_unstable();
    shown.dedup();
    notes.sort_unstable();
    (shown, notes)
}

/// Whatever the document holds, the numbers the references show and the
/// numbers of the notes are the same set, 1 to the number of notes, and
/// nothing panics at any width.
#[test]
fn footnotes_in_nested_shapes_show_the_numbers_of_their_notes() {
    let documents = std::env::var("PATINE_FUZZ_SHAPES")
        .ok()
        .and_then(|count| count.parse().ok())
        .unwrap_or(SHAPED_DOCUMENTS);
    let started = Instant::now();
    let mut random = Random(SEED ^ 0x5eed);

    for number in 0..documents {
        let mut builder = ShapeBuilder {
            random: &mut random,
            next_reference: 0,
            next_definition: 0,
        };
        let blocks = 1 + builder.random.below(6);
        let markdown = (0..blocks)
            .map(|_| builder.block(0))
            .collect::<Vec<_>>()
            .join("\n\n");

        for options in [
            RenderOptions::default(),
            RenderOptions::new().with_soft_breaks(SoftBreaks::Break),
        ] {
            for width in SHAPE_WIDTHS {
                let lines: Vec<String> = std::panic::catch_unwind(|| {
                    render_lines_with(&markdown, width, &options)
                        .iter()
                        .map(ToString::to_string)
                        .collect()
                })
                .unwrap_or_else(|panic| {
                    eprintln!("document {number} panicked at width {width}: {markdown:?}");
                    std::panic::resume_unwind(panic)
                });
                // In a narrow column, such as a table cell in a quote, a word
                // breaks between its characters, and a number no longer shows
                // as one.
                if width < 30 {
                    continue;
                }
                let (shown, notes) = shown_and_note_numbers(&lines);
                let all: Vec<usize> = (1..=notes.len()).collect();
                assert_eq!(notes, all, "notes of document {number}: {markdown:?}");
                assert_eq!(
                    shown, all,
                    "numbers shown in document {number} at width {width}: {markdown:?} as {lines:?}"
                );
            }
        }

        assert!(
            started.elapsed() < TIME_LIMIT,
            "{number} documents took longer than {TIME_LIMIT:?}"
        );
    }
}
