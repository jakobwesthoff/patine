// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! The `patine` binary as a terminal sees it: the bytes it writes for a
//! document that holds control characters. The library hands text to its
//! caller as it is (ADR 17), so the binary is where a control character
//! of a document is made harmless (ADR 21).

use std::io::Write as _;
use std::process::{Command, Stdio};

/// The bytes `patine --width <width>` writes to stdout for `input` on
/// its stdin. A pipe, not a terminal, so the binary renders instead of
/// printing its help.
fn patine_bytes(input: &str, width: usize) -> Vec<u8> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_patine"))
        .args(["--width", &width.to_string()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("the patine binary starts");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(input.as_bytes())
        .expect("patine reads its stdin");
    let output = child.wait_with_output().expect("patine finishes");
    assert!(
        output.status.success(),
        "patine failed: {:?}",
        output.status
    );
    output.stdout
}

/// What `patine` writes for `input`, with the SGR sequences it writes
/// for styles taken out, so a test compares the text a terminal shows.
fn shown(input: &str) -> String {
    without_sgr(&String::from_utf8(patine_bytes(input, 80)).expect("patine writes UTF-8"))
}

/// `text` without the sequences `ESC [ <digits, ; and :> m` that style
/// spans. Any other escape stays in the text, so a test sees it.
fn without_sgr(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("\u{1b}[") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let parameters = after
            .find(|c: char| !c.is_ascii_digit() && c != ';' && c != ':')
            .unwrap_or(after.len());
        if after[parameters..].starts_with('m') {
            rest = &after[parameters + 1..];
        } else {
            out.push_str("\u{1b}[");
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

// =========================================================
// Which controls, and what they become
// =========================================================

#[test]
fn a_raw_escape_in_a_paragraph_is_pictured() {
    assert_eq!(shown("a\u{1b}b\n"), "a\u{241b}b\n");
}

#[test]
fn a_decimal_reference_to_escape_is_pictured() {
    assert_eq!(shown("a&#27;[31mb\n"), "a\u{241b}[31mb\n");
}

#[test]
fn a_hex_reference_to_escape_is_pictured() {
    assert_eq!(shown("a&#x1b;b\n"), "a\u{241b}b\n");
}

#[test]
fn delete_is_pictured_as_its_symbol() {
    assert_eq!(shown("a&#x7f;b\n"), "a\u{2421}b\n");
}

#[test]
fn a_raw_nul_is_pictured() {
    assert_eq!(shown("a\0b\n"), "a\u{2400}b\n");
}

#[test]
fn bell_is_pictured() {
    assert_eq!(shown("a\u{7}b\n"), "a\u{2407}b\n");
}

#[test]
fn a_c1_control_becomes_the_replacement_character() {
    assert_eq!(shown("a&#x80;b\n"), "a\u{fffd}b\n");
}

#[test]
fn a_next_line_control_in_a_code_block_becomes_the_replacement_character() {
    // In running text U+0085 is whitespace and the layout has turned it
    // into a space before the binary sees it; a code block keeps it.
    assert_eq!(shown("```\ne\u{85}f\n```\n"), "  e\u{fffd}f\n");
}

#[test]
fn the_c1_csi_becomes_the_replacement_character() {
    assert_eq!(shown("a&#x9b;[31mb\n"), "a\u{fffd}[31mb\n");
}

#[test]
fn a_vertical_tab_and_form_feed_in_a_code_block_are_pictured() {
    assert_eq!(
        shown("```\na\u{b}b\u{c}c\n```\n"),
        "  a\u{240b}b\u{240c}c\n"
    );
}

#[test]
fn a_tab_stays() {
    assert_eq!(shown("```\na\tb\n```\n"), "  a\tb\n");
}

#[test]
fn a_next_line_control_in_prose_is_already_a_space() {
    assert_eq!(shown("a&#x85;b\n"), "a b\n");
}

// =========================================================
// Every place text appears
// =========================================================

#[test]
fn a_control_in_a_link_text_is_pictured() {
    assert_eq!(shown("[a\u{1b}b](http://x/)\n"), "a\u{241b}b (http://x/)\n");
}

#[test]
fn a_control_in_a_link_url_suffix_is_pictured() {
    assert_eq!(
        shown("[t](http://x/&#27;[31m)\n"),
        "t (http://x/\u{241b}[31m)\n"
    );
}

#[test]
fn a_control_in_a_reference_definition_url_is_pictured() {
    assert_eq!(
        shown("[r]\n\n[r]: http://x/&#27;q\n"),
        "r (http://x/\u{241b}q)\n"
    );
}

#[test]
fn a_control_in_a_www_autolink_is_pictured() {
    let shown = shown("www.x\u{1b}y.com\n");
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
    assert!(shown.contains("www.x\u{241b}y.com"), "{shown:?}");
}

#[test]
fn a_control_in_an_image_alt_and_url_is_pictured() {
    assert_eq!(
        shown("![a&#27;b](http://y/&#27;z)\n"),
        "[image: a\u{241b}b] (http://y/\u{241b}z)\n"
    );
}

#[test]
fn a_control_in_a_heading_is_pictured() {
    assert_eq!(shown("# h\u{1b}i\n"), "h\u{241b}i\n");
}

#[test]
fn a_control_in_inline_code_is_pictured() {
    assert_eq!(shown("`a\u{1b}b`\n"), "a\u{241b}b\n");
}

#[test]
fn a_control_in_a_plain_code_block_is_pictured() {
    assert_eq!(shown("```\na\u{1b}[2Jb\n```\n"), "  a\u{241b}[2Jb\n");
}

#[test]
fn a_control_in_a_highlighted_code_block_is_pictured() {
    let shown = shown("```rust\nfn main() { let s = \"\u{1b}[2J\"; }\n```\n");
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
    assert!(shown.contains("\u{241b}[2J"), "{shown:?}");
}

#[test]
fn a_control_in_a_table_cell_is_pictured() {
    let shown = shown("| h |\n|---|\n| a\u{1b}b |\n");
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
    assert!(shown.contains("a\u{241b}b"), "{shown:?}");
}

#[test]
fn a_control_in_a_table_header_is_pictured() {
    let shown = shown("| h\u{1b}i |\n|---|\n| a |\n");
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
    assert!(shown.contains("h\u{241b}i"), "{shown:?}");
}

#[test]
fn a_control_in_a_footnote_is_pictured() {
    let shown = shown("text[^1]\n\n[^1]: n\u{1b}x\n");
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
    assert!(shown.contains("n\u{241b}x"), "{shown:?}");
}

#[test]
fn a_control_in_yaml_front_matter_is_pictured() {
    let shown = shown("---\ntitle: a\u{1b}b\n---\n\ntext\n");
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
    assert!(shown.contains("a\u{241b}b"), "{shown:?}");
}

#[test]
fn a_control_in_toml_front_matter_is_pictured() {
    let shown = shown("+++\ntitle = \"a\u{1b}b\"\n+++\n\ntext\n");
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
    assert!(shown.contains("a\u{241b}b"), "{shown:?}");
}

#[test]
fn a_control_in_a_quote_and_a_callout_is_pictured() {
    let shown = shown("> q\u{1b}a\n\n> [!NOTE]\n> c\u{1b}b\n");
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
    assert!(shown.contains("q\u{241b}a"), "{shown:?}");
    assert!(shown.contains("c\u{241b}b"), "{shown:?}");
}

#[test]
fn a_control_in_a_list_item_is_pictured() {
    let shown = shown("- a\u{1b}b\n");
    assert!(!shown.contains('\u{1b}'), "{shown:?}");
    assert!(shown.contains("a\u{241b}b"), "{shown:?}");
}

// =========================================================
// Style and layout
// =========================================================

#[test]
fn a_control_keeps_the_style_of_its_span() {
    let bytes = patine_bytes("**a\u{1b}b**\n", 80);
    assert_eq!(
        String::from_utf8(bytes).expect("patine writes UTF-8"),
        "\u{1b}[1ma\u{241b}b\u{1b}[22m\n"
    );
}

#[test]
fn a_picture_takes_the_one_column_the_control_took() {
    let with = |filler: &str| {
        let text = format!("aaaa{filler}bbbb cccc dddd eeee ffff\n");
        without_sgr(&String::from_utf8(patine_bytes(&text, 10)).expect("patine writes UTF-8"))
    };
    assert_eq!(with("\u{1b}").replace('\u{241b}', "x"), with("x"));
}

// =========================================================
// Safety net
// =========================================================

#[test]
fn no_control_character_survives_a_hostile_document() {
    let mut document = String::from("prose: ");
    let mut block = String::new();
    for code in (0u32..0x20).chain(0x7f..0xa0) {
        let control = char::from_u32(code).expect("a control is a char");
        if control == '\t' || control == '\n' || control == '\r' {
            continue;
        }
        document.push(control);
        block.push(control);
    }
    document.push_str("\n\n```\n");
    document.push_str(&block);
    document.push_str("\n```\n\n[t](http://x/&#27;&#13;&#10;&#0;&#x9b;)\n");

    let shown = shown(&document);
    let leftover: Vec<char> = shown
        .chars()
        .filter(|&c| c != '\n' && c != '\t' && c.is_control())
        .collect();
    assert_eq!(leftover, Vec::<char>::new());
}

#[test]
fn a_document_without_controls_is_written_as_the_library_writes_it() {
    let showcase = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/showcase.md"
    ))
    .expect("the showcase fixture exists");
    let mut library = Vec::new();
    patine::render(&showcase, &mut library, 80).expect("rendering to a vector succeeds");

    assert_eq!(patine_bytes(&showcase, 80), library);
}

#[test]
fn a_file_argument_is_filtered_like_stdin() {
    let dir = std::env::temp_dir().join(format!("patine-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the temp dir is writable");
    let file = dir.join("hostile.md");
    std::fs::write(&file, "a\u{1b}b\n").expect("the temp dir is writable");

    let output = Command::new(env!("CARGO_BIN_EXE_patine"))
        .arg(&file)
        .output()
        .expect("the patine binary starts");
    std::fs::remove_dir_all(&dir).expect("the temp dir is removable");

    assert_eq!(
        without_sgr(&String::from_utf8_lossy(&output.stdout)),
        "a\u{241b}b\n"
    );
}

// =========================================================
// Messages on stderr
// =========================================================

/// The exit code and the stderr text of `patine` run with `args`, and with
/// `CLICOLOR_FORCE` set, so clap's own colors are on whether or not stderr
/// is a terminal. The text is without the SGR sequences.
fn failure(args: &[&str]) -> (Option<i32>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_patine"))
        .args(args)
        .env("CLICOLOR_FORCE", "1")
        .stdin(Stdio::null())
        .output()
        .expect("the patine binary starts");
    assert!(!output.status.success(), "patine should fail");
    (
        output.status.code(),
        without_sgr(&String::from_utf8_lossy(&output.stderr)),
    )
}

fn controls_in(text: &str) -> Vec<char> {
    text.chars()
        .filter(|&c| c != '\n' && c.is_control())
        .collect()
}

#[test]
fn a_file_name_with_a_control_in_a_read_error_is_pictured() {
    let (_, stderr) = failure(&["/nonexistent/\u{1b}]0;x\u{7}"]);

    assert_eq!(controls_in(&stderr), Vec::<char>::new(), "{stderr:?}");
    assert!(
        stderr.contains("read /nonexistent/\u{241b}]0;x\u{2407}"),
        "{stderr:?}"
    );
}

#[test]
fn an_argument_with_a_control_in_a_usage_error_is_pictured() {
    let (code, stderr) = failure(&["a", "\u{1b}]0;x\u{7}"]);

    assert_eq!(controls_in(&stderr), Vec::<char>::new(), "{stderr:?}");
    assert!(stderr.contains("\u{241b}]0;x\u{2407}"), "{stderr:?}");
    assert_eq!(code, Some(2));
}

#[test]
fn a_carriage_return_and_line_feed_in_a_file_name_are_pictured() {
    let (_, stderr) = failure(&["/nonexistent/a\r\nb"]);

    assert_eq!(controls_in(&stderr), Vec::<char>::new(), "{stderr:?}");
    assert!(
        stderr.contains("read /nonexistent/a\u{240d}\u{240a}b"),
        "{stderr:?}"
    );
}

#[test]
fn an_argument_without_a_control_in_a_usage_error_is_echoed_as_it_is() {
    let (code, stderr) = failure(&["a", "b"]);

    assert!(
        stderr.contains("unexpected argument 'b' found"),
        "{stderr:?}"
    );
    assert_eq!(code, Some(2));
}
