// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

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
    let cli = Cli::parse();

    let markdown = match cli.file {
        Some(path) => {
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?
        }
        None => {
            if io::stdin().is_terminal() {
                Cli::parse_from(["patine", "--help"]);
                unreachable!();
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
    patine::render(&markdown, &mut stdout, terminal_width)?;
    stdout.flush().context("flush stdout")?;

    Ok(())
}
