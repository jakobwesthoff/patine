// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Front matter: a block of metadata at the very start of a document,
//! between two fence lines of `---` (YAML) or `+++` (TOML). The renderer
//! shows it as a highlighted block and parses only what follows it.
//!
//! A fence is three markers and optional spaces or tabs, alone on its
//! line, and a line ends at `\n`, `\r\n` or a lone `\r`. The closing fence
//! is the first later line that is a fence of the same marker, which may be
//! the line right after the opening one, and which may end the document.
//! A block without a closing fence is no front matter.

/// The kind of front matter, which the fence marker tells apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fence {
    /// `---`
    Yaml,
    /// `+++`
    Toml,
}

impl Fence {
    /// The fence line without trailing whitespace.
    pub(crate) fn marker(self) -> &'static str {
        match self {
            Fence::Yaml => "---",
            Fence::Toml => "+++",
        }
    }

    /// The language the block is highlighted as.
    pub(crate) fn language(self) -> &'static str {
        match self {
            Fence::Yaml => "yaml",
            Fence::Toml => "toml",
        }
    }
}

/// The block of front matter at the start of a document.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct FrontMatter<'a> {
    pub(crate) fence: Fence,
    /// The lines between the fences, with the line endings they have in
    /// the document, but not the one before the closing fence.
    pub(crate) body: &'a str,
}

/// Split the front matter off the start of `input`. Returns it, if the
/// document starts with some, and the rest of the document, which is the
/// whole of `input` when there is none.
pub(crate) fn split(input: &str) -> (Option<FrontMatter<'_>>, &str) {
    // A byte order mark in front of the opening fence does not keep it
    // from opening.
    let text = input.strip_prefix('\u{feff}').unwrap_or(input);

    let (opening, after_opening) = next_line(text);
    let Some(fence) = fence_of(opening) else {
        return (None, input);
    };
    let Some(mut rest) = after_opening else {
        return (None, input);
    };

    let body_start = text.len() - rest.len();
    let mut body_end = body_start;
    loop {
        let (line, after_line) = next_line(rest);
        if fence_of(line) == Some(fence) {
            let body = &text[body_start..body_end];
            return (Some(FrontMatter { fence, body }), after_line.unwrap_or(""));
        }
        let Some(after_line) = after_line else {
            return (None, input);
        };
        body_end = (text.len() - rest.len()) + line.len();
        rest = after_line;
    }
}

/// The first line of `text` without its line ending, and the text after
/// that ending, or `None` when the line ends the text.
fn next_line(text: &str) -> (&str, Option<&str>) {
    match text.find(['\n', '\r']) {
        None => (text, None),
        Some(end) => {
            let ending_width = if text[end..].starts_with("\r\n") {
                2
            } else {
                1
            };
            (&text[..end], Some(&text[end + ending_width..]))
        }
    }
}

/// The fence a line is, if it is one.
fn fence_of(line: &str) -> Option<Fence> {
    match line.trim_end_matches([' ', '\t']) {
        "---" => Some(Fence::Yaml),
        "+++" => Some(Fence::Toml),
        _ => None,
    }
}

// =========================================================
// Unit tests
// =========================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn front_matter(input: &str) -> Option<(Fence, &str)> {
        split(input).0.map(|front| (front.fence, front.body))
    }

    #[test]
    fn a_block_between_fences_is_front_matter_and_the_rest_follows() {
        assert_eq!(
            split("---\ntitle: x\n---\nBody"),
            (
                Some(FrontMatter {
                    fence: Fence::Yaml,
                    body: "title: x"
                }),
                "Body"
            )
        );
        assert_eq!(
            split("+++\na = 1\nb = 2\n+++\n\nBody"),
            (
                Some(FrontMatter {
                    fence: Fence::Toml,
                    body: "a = 1\nb = 2"
                }),
                "\nBody"
            )
        );
    }

    #[test]
    fn the_body_keeps_its_own_line_endings() {
        assert_eq!(
            front_matter("---\r\na: 1\r\nb: 2\r\n---\r\n"),
            Some((Fence::Yaml, "a: 1\r\nb: 2"))
        );
        assert_eq!(
            front_matter("---\ra: 1\rb: 2\r---\r"),
            Some((Fence::Yaml, "a: 1\rb: 2"))
        );
        assert_eq!(
            front_matter("---\na: 1\r\nb: 2\r---\n"),
            Some((Fence::Yaml, "a: 1\r\nb: 2"))
        );
    }

    #[test]
    fn an_empty_block_is_front_matter() {
        assert_eq!(front_matter("---\n---\nBody"), Some((Fence::Yaml, "")));
        assert_eq!(front_matter("+++\n+++"), Some((Fence::Toml, "")));
        assert_eq!(front_matter("---\n\n---"), Some((Fence::Yaml, "")));
    }

    #[test]
    fn the_closing_fence_may_end_the_document() {
        assert_eq!(split("---\na: 1\n---"), (front_matter_of("a: 1"), ""));
        assert_eq!(split("---\na: 1\n---\n"), (front_matter_of("a: 1"), ""));
    }

    fn front_matter_of(body: &str) -> Option<FrontMatter<'_>> {
        Some(FrontMatter {
            fence: Fence::Yaml,
            body,
        })
    }

    #[test]
    fn fences_may_end_in_spaces_and_tabs() {
        assert_eq!(
            front_matter("--- \t\na: 1\n---\t \nBody"),
            Some((Fence::Yaml, "a: 1"))
        );
    }

    #[test]
    fn a_byte_order_mark_before_the_opening_fence_is_skipped() {
        assert_eq!(
            split("\u{feff}---\na: 1\n---\nBody"),
            (front_matter_of("a: 1"), "Body")
        );
    }

    #[test]
    fn the_first_matching_fence_closes_the_block() {
        assert_eq!(
            split("---\na\n---\nb\r\n---\r\nBody"),
            (front_matter_of("a"), "b\r\n---\r\nBody")
        );
    }

    #[test]
    fn only_a_fence_of_the_same_marker_closes_the_block() {
        assert_eq!(
            front_matter("---\na\n+++\nb\n---"),
            Some((Fence::Yaml, "a\n+++\nb"))
        );
    }

    #[test]
    fn what_is_not_a_fence_is_content() {
        assert_eq!(
            front_matter("---\na\n----\n--- x\n--\n---"),
            Some((Fence::Yaml, "a\n----\n--- x\n--"))
        );
    }

    #[test]
    fn without_a_closing_fence_there_is_no_front_matter() {
        for input in [
            "---\na: 1",
            "---\na: 1\n",
            "+++\na = 1\n---",
            "---",
            "---\n",
        ] {
            assert_eq!(split(input), (None, input), "{input:?}");
        }
    }

    #[test]
    fn the_opening_fence_must_start_the_document_and_stand_alone() {
        for input in [
            "\n---\na\n---",
            " ---\na\n---",
            "----\na\n----",
            "-- -\na\n---",
            "--- x\na\n---",
            "text\n---\na\n---",
            "",
        ] {
            assert_eq!(split(input), (None, input), "{input:?}");
        }
    }
}
