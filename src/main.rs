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

    /// Output width in columns. On a terminal the output is at most as wide
    /// as the terminal; redirected output is exactly this wide. Defaults to
    /// the terminal width.
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

    let output_width = output_width(cli.width, io::stdout().is_terminal(), || {
        crossterm::terminal::size()
            .ok()
            .map(|(columns, _)| usize::from(columns))
    });

    let mut stdout = io::stdout().lock();
    let lines = harmless(patine::render_lines(&markdown, output_width));
    write!(stdout, "{}", patine::ansi(&lines)).context("write the rendered lines")?;
    stdout.flush().context("flush stdout")?;

    Ok(())
}

// =========================================================
// Output width
// =========================================================

/// The width the output is laid out for.
///
/// `requested` is the width from `--width` or `PATINE_DEFAULT_WIDTH`. On a
/// terminal it is a maximum: the output is no wider than the terminal. When
/// stdout is not a terminal it is the width as given.
///
/// The decision looks at stdout and not at whether a terminal size can be
/// read, because `terminal_width` asks the controlling terminal (`/dev/tty`,
/// `CONOUT$`) and not stdout, so it finds a size in a run whose output goes
/// to a file. The closure is called only when its answer is needed.
///
/// A size of 0 is no size: a terminal whose size was never set reports
/// 0 columns, and capping at it would lay everything out for 0 columns.
/// Without a readable size there is nothing to cap against, so a requested
/// width stands, and without a request the width is 80.
fn output_width(
    requested: Option<usize>,
    stdout_is_terminal: bool,
    terminal_width: impl FnOnce() -> Option<usize>,
) -> usize {
    let readable_width = || terminal_width().filter(|&columns| columns > 0);
    match requested {
        Some(requested) if !stdout_is_terminal => requested,
        Some(requested) => readable_width().map_or(requested, |columns| columns.min(requested)),
        None => readable_width().unwrap_or(80),
    }
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
fn harmless_text(text: &str) -> String {
    text.chars().map(harmless_char).collect()
}

/// `c`, or what shows in its place when it is a control character other
/// than tab: the Control Pictures block for C0 and DEL, which keeps the
/// reader able to tell which control the document held, and U+FFFD for C1,
/// which that block has no pictures for. Each replacement counts one column,
/// as the control did when the lines were laid out.
fn harmless_char(c: char) -> char {
    match u32::from(c) {
        0x09 => c,
        code @ 0x00..0x20 => char::from_u32(0x2400 + code).expect("the Control Pictures block"),
        0x7f => '\u{2421}',
        0x80..0xa0 => char::REPLACEMENT_CHARACTER,
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::output_width;

    #[test]
    fn a_terminal_narrower_than_the_requested_width_wins() {
        assert_eq!(output_width(Some(120), true, || Some(80)), 80);
    }

    #[test]
    fn a_requested_width_narrower_than_the_terminal_wins() {
        assert_eq!(output_width(Some(60), true, || Some(80)), 60);
    }

    #[test]
    fn a_requested_width_equal_to_the_terminal_gives_that_width() {
        assert_eq!(output_width(Some(80), true, || Some(80)), 80);
    }

    #[test]
    fn a_requested_width_wider_than_the_terminal_is_exact_when_stdout_is_not_a_terminal() {
        assert_eq!(output_width(Some(120), false, || Some(80)), 120);
    }

    #[test]
    fn a_requested_width_narrower_than_the_terminal_is_exact_when_stdout_is_not_a_terminal() {
        assert_eq!(output_width(Some(40), false, || Some(80)), 40);
    }

    #[test]
    fn a_requested_width_does_not_ask_for_the_terminal_when_stdout_is_not_a_terminal() {
        let width = output_width(Some(120), false, || {
            panic!("the terminal width is not needed")
        });
        assert_eq!(width, 120);
    }

    #[test]
    fn without_a_requested_width_the_terminal_width_is_used() {
        assert_eq!(output_width(None, true, || Some(100)), 100);
    }

    #[test]
    fn without_a_requested_width_the_terminal_width_is_used_when_stdout_is_not_a_terminal() {
        assert_eq!(output_width(None, false, || Some(100)), 100);
    }

    #[test]
    fn without_a_requested_width_and_without_a_size_the_width_is_80() {
        assert_eq!(output_width(None, true, || None), 80);
        assert_eq!(output_width(None, false, || None), 80);
    }

    #[test]
    fn a_requested_width_stands_on_a_terminal_without_a_size() {
        assert_eq!(output_width(Some(120), true, || None), 120);
    }

    #[test]
    fn a_requested_width_stands_when_stdout_is_not_a_terminal_and_there_is_no_size() {
        assert_eq!(output_width(Some(120), false, || None), 120);
    }

    #[test]
    fn a_size_of_zero_is_no_size_without_a_requested_width() {
        assert_eq!(output_width(None, true, || Some(0)), 80);
    }

    #[test]
    fn a_size_of_zero_does_not_cap_a_requested_width() {
        assert_eq!(output_width(Some(120), true, || Some(0)), 120);
    }
}
