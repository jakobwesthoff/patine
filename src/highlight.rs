// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// =========================================================
// Syntax highlighting for fenced code blocks
// =========================================================
//
// Powered by `syntect` via the `two-face` crate, which repackages
// the bat project's curated grammar and theme set. We intentionally
// restrict ourselves to the `Ansi` theme (ANSI palette indices 0-7),
// so that highlight colors resolve against the user's terminal
// theme rather than fixed truecolor RGB. Design rationale lives in
// `docs/adr/0005-syntax-highlighting-via-syntect-with-ansi-theme.md`.

use std::sync::LazyLock;

use anyhow::{Context, Result};
use crossterm::style::Color;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style as SyntectStyle, Theme};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use two_face::theme::EmbeddedThemeName;

// ---------------------------------------------------------
// Lazy singletons
// ---------------------------------------------------------
//
// Building the `SyntaxSet` from the packdump and cloning the theme
// both have noticeable cost. We pay that once, on first use, and
// reuse across every code block in every render.

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);

static THEME: LazyLock<Theme> =
    LazyLock::new(|| two_face::theme::extra().get(EmbeddedThemeName::Ansi).clone());

// ---------------------------------------------------------
// Public token representation
// ---------------------------------------------------------

/// A single highlighted token: the text slice, its foreground color
/// (if any) decoded onto the ANSI palette, and bold/italic flags.
///
/// `fg == None` means "use the terminal's default foreground" — the
/// theme explicitly wants no color for this scope.
#[derive(Debug)]
pub(crate) struct Token<'a> {
    pub fg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub text: &'a str,
}

// ---------------------------------------------------------
// Public API
// ---------------------------------------------------------

/// Resolve a fence language hint to a syntect syntax.
///
/// Returns `None` when `lang` is `None`, empty, whitespace-only, or
/// does not match any known syntax by alias, extension, or name —
/// callers then fall back to the flat `CODE_COLOR` rendering path.
///
/// `find_syntax_by_token` handles the common short forms (`rs`,
/// `py`, `js`, `sh`, …) via each grammar's registered file
/// extensions and aliases. `find_syntax_by_name` is the backstop
/// for full names like "Rust" or "Markdown".
pub(crate) fn syntax_for(lang: Option<&str>) -> Option<&'static SyntaxReference> {
    let name = lang?.trim();
    if name.is_empty() {
        return None;
    }
    SYNTAXES
        .find_syntax_by_token(name)
        .or_else(|| SYNTAXES.find_syntax_by_name(name))
}

/// Construct a highlighter for a single code block. Reuse across
/// all lines of that block so that block-scoped lexer state (nested
/// comments, multi-line strings) is preserved line-to-line.
pub(crate) fn new_highlighter(syntax: &SyntaxReference) -> HighlightLines<'static> {
    HighlightLines::new(syntax, &THEME)
}

/// Highlight a single line and return the resulting tokens.
///
/// The caller must pass the line with a trailing `\n` — that is
/// the contract of `extra_newlines()` grammars. The terminating
/// newline is stripped from the final token's text so downstream
/// code can write each token unmodified without having to handle
/// the newline specially.
pub(crate) fn highlight_line<'a>(
    hl: &mut HighlightLines<'_>,
    line_with_newline: &'a str,
) -> Result<Vec<Token<'a>>> {
    let raw = hl
        .highlight_line(line_with_newline, &SYNTAXES)
        .context("highlight code line")?;

    Ok(raw
        .into_iter()
        .map(|(style, text)| {
            let text = text.strip_suffix('\n').unwrap_or(text);
            Token {
                fg: decode_ansi(style),
                bold: style.font_style.contains(FontStyle::BOLD),
                italic: style.font_style.contains(FontStyle::ITALIC),
                text,
            }
        })
        // Empty segments are common at line boundaries; skip them so
        // we do not emit zero-width SGR pairs.
        .filter(|t| !t.text.is_empty())
        .collect())
}

// ---------------------------------------------------------
// ANSI palette decoding (see ADR 0005)
// ---------------------------------------------------------
//
// The bat/base16 convention, as implemented by the `Ansi` theme,
// encodes ANSI palette colors inside a standard RGBA value:
//
//   - `a == 0x00` — ANSI-indexed; `r` carries the palette index.
//   - `a == 0x01` — use the terminal's default foreground.
//   - anything else — truecolor RGB. The `Ansi` theme never emits
//     this, so we treat it as "default" defensively.

fn decode_ansi(style: SyntectStyle) -> Option<Color> {
    let c = style.foreground;
    match c.a {
        0x00 => Some(Color::AnsiValue(c.r)),
        0x01 => None,
        _ => None,
    }
}

// =========================================================
// Tests
// =========================================================

#[cfg(test)]
mod tests {
    use super::*;
    use syntect::highlighting::{Color as SyntectColor, FontStyle, Style};

    fn style_with(r: u8, a: u8) -> Style {
        Style {
            foreground: SyntectColor { r, g: 0, b: 0, a },
            background: SyntectColor { r: 0, g: 0, b: 0, a: 0 },
            font_style: FontStyle::empty(),
        }
    }

    #[test]
    fn ansi_indexed_decodes_to_ansi_value() {
        for r in 0u8..=7 {
            let got = decode_ansi(style_with(r, 0x00));
            assert!(
                matches!(got, Some(Color::AnsiValue(n)) if n == r),
                "palette index {r} should decode to AnsiValue({r}), got {got:?}"
            );
        }
    }

    #[test]
    fn default_foreground_decodes_to_none() {
        assert!(decode_ansi(style_with(0, 0x01)).is_none());
    }

    #[test]
    fn truecolor_decodes_to_none_as_safe_fallback() {
        // `a == 0xff` is the conventional "opaque truecolor" marker
        // used by non-ANSI themes. The `Ansi` theme never produces
        // it; this branch only exists to avoid emitting garbage if
        // a future theme swap slips through.
        assert!(decode_ansi(style_with(0x80, 0xff)).is_none());
    }

    #[test]
    fn syntax_for_none_returns_none() {
        assert!(syntax_for(None).is_none());
    }

    #[test]
    fn syntax_for_empty_returns_none() {
        assert!(syntax_for(Some("")).is_none());
        assert!(syntax_for(Some("   ")).is_none());
    }

    #[test]
    fn syntax_for_unknown_returns_none() {
        assert!(syntax_for(Some("definitely-not-a-language-xyz")).is_none());
    }

    #[test]
    fn syntax_for_common_aliases_resolve() {
        // Short forms widely used in fence hints.
        for token in ["rust", "rs", "py", "python", "js", "ts", "sh", "bash", "json"] {
            assert!(
                syntax_for(Some(token)).is_some(),
                "expected syntax for {token:?} to resolve"
            );
        }
    }
}
