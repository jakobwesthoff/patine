// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::ffi::OsString;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;

use anyhow::Context;
use clap::{CommandFactory, Parser};
use patine::{StyledLine, StyledSpan};

/// Render Markdown beautifully in the terminal.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Markdown file to render. Reads from stdin if omitted.
    file: Option<PathBuf>,

    /// Override the output width in columns. Defaults to the terminal width.
    #[arg(short, long)]
    width: Option<usize>,
}

fn main() -> anyhow::Result<()> {
    let cli = parse_cli();

    let markdown = match cli.file {
        Some(path) => std::fs::read_to_string(&path)
            .with_context(|| format!("read {}", harmless_text(&path.display().to_string())))?,
        None => {
            if io::stdin().is_terminal() {
                Cli::command().print_help().context("print help")?;
                return Ok(());
            }
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).context("read stdin")?;
            buf
        }
    };

    let terminal_width = cli.width.unwrap_or_else(|| {
        crossterm::terminal::size()
            .map(|(cols, _)| cols as usize)
            .unwrap_or(80)
    });

    let mut stdout = io::stdout().lock();
    let lines = harmless(patine::render_lines(&markdown, terminal_width));
    write!(stdout, "{}", patine::ansi(&lines)).context("write the rendered lines")?;
    stdout.flush().context("flush stdout")?;

    Ok(())
}

// =========================================================
// The command line
// =========================================================

/// The parsed command line, or the end of the program with clap's message.
///
/// clap's message quotes the argument it rejects, and that argument can
/// hold controls. They stay in the file name used to read the file, so the
/// message is made from the arguments with the controls replaced, parsed
/// a second time. Every rule of the command line looks at digits, dashes
/// and names, which the replacement leaves alone, so the second parse
/// fails the way the first did, with the same kind of message in clap's
/// colors.
fn parse_cli() -> Cli {
    if let Ok(cli) = Cli::try_parse() {
        return cli;
    }
    let harmless_arguments = std::env::args_os()
        .map(|argument| OsString::from(harmless_text(&argument.to_string_lossy())));
    Cli::try_parse_from(harmless_arguments)
        .err()
        .expect("replacing controls in arguments does not make a rejected command line valid")
        .exit()
}

// =========================================================
// Control characters
// =========================================================

/// `lines` with every control character of their text replaced by a
/// character that shows instead of acting.
///
/// The library hands span text to its caller as it is (ADR 17), and a
/// document can hold an escape sequence, as a raw byte or as a character
/// reference such as `&#27;`. Written to a terminal, it would run. Every
/// character the binary writes is span text, so mapping the spans covers
/// the whole output.
fn harmless(lines: Vec<StyledLine>) -> Vec<StyledLine> {
    lines
        .into_iter()
        .map(|line| {
            line.spans
                .into_iter()
                .map(|span| StyledSpan::new(harmless_text(&span.text), span.style))
                .collect()
        })
        .collect()
}

/// `text` with each control character other than tab replaced.
///
/// Each replacement is one character that counts one column, as the
/// control did when the lines were laid out, so no line gets wider. CR
/// followed by LF is the one pair that is a single grapheme and counts one
/// column, so it becomes one character as well. Tab stays: the terminal
/// draws it as before, and the layout counts it as one column either way.
fn harmless_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' && chars.peek() == Some(&'\n') {
            chars.next();
            out.push(SYMBOL_FOR_NEWLINE);
        } else {
            out.push(harmless_char(c));
        }
    }
    out
}

/// U+2424, the Control Pictures symbol for a CR LF pair.
const SYMBOL_FOR_NEWLINE: char = '\u{2424}';

/// `c`, or what shows in its place when it is a control character other
/// than tab: the Control Pictures block for C0 and DEL, which keeps the
/// reader able to tell which control the document held, and U+FFFD for C1,
/// which that block has no pictures for.
fn harmless_char(c: char) -> char {
    match u32::from(c) {
        0x09 => c,
        code @ 0x00..0x20 => char::from_u32(0x2400 + code).expect("the Control Pictures block"),
        0x7f => '\u{2421}',
        0x80..0xa0 => char::REPLACEMENT_CHARACTER,
        _ => c,
    }
}
