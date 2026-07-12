// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod highlight;
mod render;
mod table;
mod wrap;

use std::io::Write;

use anyhow::{Context, Result};

/// Parse and render a Markdown string as styled terminal output, wrapping
/// text at `terminal_width` columns.
pub fn render(input: &str, writer: &mut impl Write, terminal_width: usize) -> Result<()> {
    // GFM plus frontmatter: `Constructs::gfm()` does not enable the
    // frontmatter construct, so a `---`/`+++` block at the document start
    // would otherwise be parsed as ordinary markdown (setext heading,
    // thematic break) and mangle the metadata.
    let options = markdown::ParseOptions {
        constructs: markdown::Constructs {
            frontmatter: true,
            ..markdown::Constructs::gfm()
        },
        ..markdown::ParseOptions::gfm()
    };
    let tree = markdown::to_mdast(input, &options)
        .map_err(|e| anyhow::anyhow!(e))
        .context("parse markdown")?;

    let mut renderer = render::Renderer::new(writer, terminal_width);
    renderer.render_node(&tree)?;
    renderer.finish()
}
