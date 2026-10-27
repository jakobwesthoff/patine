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
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style as SyntectStyle, Theme};
use syntect::parsing::syntax_definition::Pattern;
use syntect::parsing::{Regex, SyntaxReference, SyntaxSet, SyntaxSetBuilder};
use two_face::theme::EmbeddedThemeName;

use crate::line_endings::verbatim_lines;
use crate::styled::{AnsiColor, StyledLine, TextStyle};

// ---------------------------------------------------------
// Lazy singletons
// ---------------------------------------------------------
//
// Building the `SyntaxSet` from the packdump and cloning the theme
// both have noticeable cost. We pay that once, on first use, and
// reuse across every code block in every render.

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);

/// two-face's syntax set with the level lookaheads of its Log syntax
/// anchored (see [`anchor_level_lookaheads`]); only `log` highlights
/// from it. syntect cannot change one syntax of a built set, so this is
/// a second set: rebuilding it takes about 0.2 s and keeps about 7 MB,
/// which only a process that highlights a log pays, once.
static ANCHORED_LOG_SYNTAXES: LazyLock<SyntaxSet> =
	LazyLock::new(|| anchor_level_lookaheads(&two_face::syntax::extra_newlines().into_builder()).build());

static THEME: LazyLock<Theme> = LazyLock::new(|| two_face::theme::extra().get(EmbeddedThemeName::Ansi).clone());

// ---------------------------------------------------------
// Crate API
// ---------------------------------------------------------

/// Resolve a fence language hint to a syntect syntax.
///
/// Returns `None` when `lang` is `None`, empty, whitespace-only, or
/// does not match any known syntax by alias, extension, or name —
/// callers then fall back to unhighlighted text.
///
/// `find_syntax_by_token` handles the common short forms (`rs`,
/// `py`, `js`, `sh`, …) via each grammar's registered file
/// extensions and aliases. `find_syntax_by_name` is the backstop
/// for full names like "Rust" or "Markdown".
fn syntax_for(lang: Option<&str>) -> Option<(&'static SyntaxSet, &'static SyntaxReference)> {
	let name = lang?.trim();
	if name.is_empty() {
		return None;
	}
	let syntax = SYNTAXES.find_syntax_by_token(name).or_else(|| SYNTAXES.find_syntax_by_name(name))?;
	if syntax.name == LOG_SYNTAX {
		let anchored = ANCHORED_LOG_SYNTAXES.find_syntax_by_name(LOG_SYNTAX).expect("the anchored set holds every syntax of the shipped one");
		return Some((&ANCHORED_LOG_SYNTAXES, anchored));
	}
	Some((&SYNTAXES, syntax))
}

// ---------------------------------------------------------
// Anchored level lookaheads of the Log syntax (see ADR 0012)
// ---------------------------------------------------------
//
// The Log syntax's context `main` includes `log_level_lines`, whose
// lookaheads test a line for a level word. They have no anchor: on a
// line without the word, the regex engine tries the lookahead at every
// position from the start of the search, and each try scans to the end
// of the line, so a line takes time quadratic in its length. syntect
// then remembers the miss for the rest of the line.
//
// syntect searches one line at a time, from its current position. If
// such a lookahead holds at some position, it holds at the start of the
// search too, since `.*` reaches every later position of the line. So
// the search finds a match at its start or none, and `\G`, which
// matches only where the search starts, finds the same after one try.

/// Name of two-face's Log syntax.
const LOG_SYNTAX: &str = "log";

/// The context of the Log syntax that tests a line for its level.
const LEVEL_CONTEXT: &str = "log_level_lines";

/// The patterns of [`LEVEL_CONTEXT`] as two-face ships them. Only these
/// exact texts are anchored: a pattern that changes upstream is left as
/// it is rather than rewritten on a guess, and a test fails.
const LEVEL_LOOKAHEADS: [&str; 4] = [
	r"(?=.*\b(?i:fail(?:ure|ed)?|error|exception|fatal|critical)\b)",
	r"(?=.*\b(?i:warn(?:ing)?)\b)",
	r"(?=.*\b(?i:info)\b)",
	r"(?=.*\b(?i:debug)\b)",
];

/// A copy of `syntaxes` whose Log syntax has `\G` in front of each of
/// [`LEVEL_LOOKAHEADS`]. Every syntax definition is copied as it is.
fn anchor_level_lookaheads(syntaxes: &SyntaxSetBuilder) -> SyntaxSetBuilder {
	// A builder gives no mutable access to its definitions, so each one
	// is copied into a new builder. The new builder has no syntaxes by
	// file path, which patine never looks up.
	let mut anchored = SyntaxSetBuilder::new();
	for definition in syntaxes.syntaxes() {
		let mut definition = definition.clone();
		if definition.name == LOG_SYNTAX
			&& let Some(context) = definition.contexts.get_mut(LEVEL_CONTEXT)
		{
			for pattern in &mut context.patterns {
				if let Pattern::Match(pattern) = pattern
					&& LEVEL_LOOKAHEADS.contains(&pattern.regex.regex_str())
				{
					pattern.regex = Regex::new(format!(r"\G{}", pattern.regex.regex_str()));
				}
			}
		}
		anchored.add(definition);
	}
	anchored
}

/// Highlight `code` line by line. Each returned line holds the tokens of
/// one source line, styled with at most a foreground color, bold and
/// italic.
///
/// One `HighlightLines` serves the whole text so that lexer state
/// spanning lines (block comments, multi-line strings) carries over from
/// one line to the next.
///
/// Fails when syntect's regex engine gives up on a line (for example on
/// its backtracking limit); callers then show the text unhighlighted.
fn highlight_code(code: &str, syntaxes: &SyntaxSet, syntax: &SyntaxReference) -> Result<Vec<StyledLine>> {
	let mut highlighter = HighlightLines::new(syntax, &THEME);
	verbatim_lines(code)
		.map(|line| {
			// `extra_newlines` grammars require each input line to
			// terminate in `\n`; the newline is stripped back off the
			// final token below.
			let line_with_newline = format!("{line}\n");
			let tokens = highlighter.highlight_line(&line_with_newline, syntaxes).context("highlight code line")?;
			let mut styled = StyledLine::default();
			for (style, text) in tokens {
				let text = text.strip_suffix('\n').unwrap_or(text);
				styled.push_str(text, token_style(style));
			}
			Ok(styled)
		})
		.collect()
}

/// Highlight `code` as `language`, or `None` when it is to be shown
/// without highlighting, as decided by [`usable_highlighting`].
pub(crate) fn try_highlight(code: &str, language: Option<&str>) -> Option<Vec<StyledLine>> {
	usable_highlighting(syntax_for(language).map(|(syntaxes, syntax)| highlight_code(code, syntaxes, syntax)))
}

/// The highlighted lines of an attempt to highlight, or `None` when the
/// code is to be shown without highlighting: no known syntax matched its
/// language (`None`), or syntect failed on it (`Some(Err(_))`).
///
/// A failure is a limit of syntect's regex engine on this particular
/// text, not a problem of the code, which is still worth showing. Callers
/// show it the way they show code without a language.
fn usable_highlighting(attempt: Option<Result<Vec<StyledLine>>>) -> Option<Vec<StyledLine>> {
	attempt.and_then(Result::ok)
}

fn token_style(style: SyntectStyle) -> TextStyle {
	TextStyle {
		foreground: decode_ansi(style),
		bold: style.font_style.contains(FontStyle::BOLD),
		italic: style.font_style.contains(FontStyle::ITALIC),
		..TextStyle::default()
	}
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
//
// The `Ansi` theme only uses indices 0 to 7. An index beyond that
// falls back to the default foreground, like truecolor does.

fn decode_ansi(style: SyntectStyle) -> Option<AnsiColor> {
	let c = style.foreground;
	match c.a {
		0x00 => AnsiColor::from_index(c.r),
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
	use syntect::parsing::{ParseState, ScopeStackOp};

	fn style_with(r: u8, a: u8) -> Style {
		Style { foreground: SyntectColor { r, g: 0, b: 0, a }, background: SyntectColor { r: 0, g: 0, b: 0, a: 0 }, font_style: FontStyle::empty() }
	}

	#[test]
	fn ansi_indexed_decodes_to_palette_color() {
		for r in 0u8..=7 {
			let got = decode_ansi(style_with(r, 0x00));
			assert_eq!(got.map(AnsiColor::index), Some(r), "palette index {r} should decode to its palette color");
		}
	}

	#[test]
	fn indices_beyond_the_eight_color_palette_decode_to_none() {
		assert!(decode_ansi(style_with(8, 0x00)).is_none());
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
			assert!(syntax_for(Some(token)).is_some(), "expected syntax for {token:?} to resolve");
		}
	}

	#[test]
	fn highlight_code_yields_one_line_per_source_line() {
		let (syntaxes, rust) = syntax_for(Some("rust")).expect("rust is bundled");
		let lines = highlight_code("let x = 1;\n\nlet y = 2;\n", syntaxes, rust).expect("valid rust");
		let texts: Vec<String> = lines.iter().map(|line| line.spans.iter().map(|span| span.text.as_str()).collect()).collect();
		assert_eq!(texts, ["let x = 1;", "", "let y = 2;"]);
		assert!(lines[1].spans.is_empty(), "an empty line has no spans");
	}

	#[test]
	fn highlight_code_colors_tokens_from_the_palette() {
		let (syntaxes, rust) = syntax_for(Some("rust")).expect("rust is bundled");
		let lines = highlight_code("let x = 1;", syntaxes, rust).expect("valid rust");
		let keyword = &lines[0].spans[0];
		assert_eq!(keyword.text, "let");
		assert!(keyword.style.foreground.is_some(), "{keyword:?}");
	}

	#[test]
	fn highlight_code_carries_lexer_state_across_lines() {
		// The second line is still inside the string literal opened on the
		// first, so it is styled like the string, not like code.
		let (syntaxes, rust) = syntax_for(Some("rust")).expect("rust is bundled");
		let lines = highlight_code("let s = \"open\nstill string\";", syntaxes, rust).expect("valid rust");
		let string_style = lines[0].spans.iter().find(|span| span.text.contains("open")).expect("string token").style;
		assert_eq!(lines[1].spans[0].style, string_style);
	}

	fn line(text: &str) -> StyledLine {
		let mut line = StyledLine::default();
		line.push_str(text, TextStyle::default());
		line
	}

	#[test]
	fn successful_highlighting_is_used() {
		let lines = vec![line("let x = 1;")];
		assert_eq!(usable_highlighting(Some(Ok(lines.clone()))), Some(lines));
	}

	#[test]
	fn failed_highlighting_falls_back() {
		let failure = Err(anyhow::anyhow!("backtracking limit"));
		assert_eq!(usable_highlighting(Some(failure)), None);
	}

	#[test]
	fn code_without_a_known_syntax_falls_back() {
		assert_eq!(usable_highlighting(None), None);
	}

	#[test]
	fn try_highlight_highlights_known_languages_only() {
		assert!(try_highlight("let x = 1;", Some("rust")).is_some());
		assert!(try_highlight("let x = 1;", Some("no-such-language")).is_none());
		assert!(try_highlight("let x = 1;", None).is_none());
	}

	// ── The Log syntax's level lookaheads ───────────────────────────────

	/// The regular expressions of the Log syntax's [`LEVEL_CONTEXT`].
	fn level_patterns(syntaxes: &SyntaxSetBuilder) -> Vec<String> {
		let log = syntaxes.syntaxes().iter().find(|definition| definition.name == LOG_SYNTAX).expect("two-face ships the Log syntax");
		log.contexts[LEVEL_CONTEXT]
			.patterns
			.iter()
			.map(|pattern| match pattern {
				Pattern::Match(pattern) => pattern.regex.regex_str().to_string(),
				Pattern::Include(reference) => format!("include {reference:?}"),
			})
			.collect()
	}

	#[test]
	fn the_shipped_log_syntax_has_the_four_level_lookaheads() {
		// When a two-face release changes these patterns, the anchoring no
		// longer applies to them and has to be looked at again.
		let shipped = two_face::syntax::extra_newlines().into_builder();
		assert_eq!(level_patterns(&shipped), LEVEL_LOOKAHEADS);
	}

	#[test]
	fn anchoring_puts_g_in_front_of_each_level_lookahead() {
		let shipped = two_face::syntax::extra_newlines().into_builder();
		assert_eq!(level_patterns(&anchor_level_lookaheads(&shipped)), LEVEL_LOOKAHEADS.map(|pattern| format!(r"\G{pattern}")));
	}

	#[test]
	fn anchoring_leaves_every_other_pattern_alone() {
		let shipped = two_face::syntax::extra_newlines().into_builder();
		let anchored = anchor_level_lookaheads(&shipped);
		assert_eq!(shipped.syntaxes().len(), anchored.syntaxes().len());
		for (before, after) in shipped.syntaxes().iter().zip(anchored.syntaxes()) {
			if before.name != LOG_SYNTAX {
				assert_eq!(before, after);
				continue;
			}
			let mut before = before.clone();
			let mut after = after.clone();
			let level_before = before.contexts.remove(LEVEL_CONTEXT).expect("shipped level context");
			let level_after = after.contexts.remove(LEVEL_CONTEXT).expect("anchored level context");
			assert_eq!(before, after, "the Log syntax outside its level context");
			assert_eq!(level_before.patterns.len(), level_after.patterns.len());
			for (before, after) in level_before.patterns.iter().zip(&level_after.patterns) {
				let (Pattern::Match(before), Pattern::Match(after)) = (before, after) else {
					panic!("the level context holds match patterns only");
				};
				assert_eq!(before.scope, after.scope);
				assert_eq!(before.captures, after.captures);
				assert_eq!(before.has_captures, after.has_captures);
				assert_eq!(before.operation, after.operation);
				assert_eq!(before.with_prototype, after.with_prototype);
			}
		}
	}

	#[test]
	fn log_highlights_from_the_anchored_syntax_set() {
		let (syntaxes, log) = syntax_for(Some("log")).expect("log is bundled");
		assert!(std::ptr::eq(syntaxes, &*ANCHORED_LOG_SYNTAXES));
		assert_eq!(log.name, LOG_SYNTAX);
		let (syntaxes, _) = syntax_for(Some("rust")).expect("rust is bundled");
		assert!(std::ptr::eq(syntaxes, &*SYNTAXES));
	}

	/// The scope operations of each line of `text` under the Log syntax
	/// of `syntaxes`.
	fn log_scope_ops(syntaxes: &SyntaxSet, text: &str) -> Vec<Vec<(usize, ScopeStackOp)>> {
		let log = syntaxes.find_syntax_by_name(LOG_SYNTAX).expect("log is bundled");
		let mut state = ParseState::new(log);
		verbatim_lines(text).map(|line| state.parse_line(&format!("{line}\n"), syntaxes).expect("the Log syntax parses every line")).collect()
	}

	/// How many line annotations, such as `meta.annotation.error-line.log`
	/// for a line holding an error, `ops` pushes.
	fn line_annotations(ops: &[Vec<(usize, ScopeStackOp)>]) -> usize {
		ops.iter()
			.flatten()
			.filter(|(_, op)| match op {
				ScopeStackOp::Push(scope) => scope.build_string().starts_with("meta.annotation."),
				_ => false,
			})
			.count()
	}

	/// Log lines covering every level word in several spellings and
	/// places, words that only contain one, quoted strings spanning
	/// lines, URLs, addresses, dates and numbers.
	const LEVELS_LOG: &str = include_str!("../tests/fixtures/levels.log");

	#[test]
	fn anchored_log_parses_like_the_shipped_syntax() {
		// The `Ansi` theme colors none of the scopes the level lookaheads
		// push, so the scope operations are compared, and the colors too.
		let shipped = log_scope_ops(&SYNTAXES, LEVELS_LOG);
		let anchored = log_scope_ops(&ANCHORED_LOG_SYNTAXES, LEVELS_LOG);
		assert_eq!(shipped.len(), anchored.len());
		for (number, (shipped, anchored)) in shipped.iter().zip(&anchored).enumerate() {
			assert_eq!(shipped, anchored, "line {}", number + 1);
		}
		assert!(line_annotations(&anchored) > 0);
		assert_eq!(line_annotations(&shipped), line_annotations(&anchored));

		let colors = |syntaxes: &SyntaxSet| {
			let log = syntaxes.find_syntax_by_name(LOG_SYNTAX).expect("log is bundled");
			highlight_code(LEVELS_LOG, syntaxes, log).expect("the fixture highlights")
		};
		assert_eq!(colors(&SYNTAXES), colors(&ANCHORED_LOG_SYNTAXES));
	}

	#[test]
	fn anchored_lookaheads_still_find_the_level_of_a_line() {
		let error_line = log_scope_ops(&ANCHORED_LOG_SYNTAXES, "worker 3 ERROR: disk full\n");
		assert!(
			error_line.iter().flatten().any(|(_, op)| matches!(
				op,
				ScopeStackOp::Push(scope)
					if scope.build_string() == "meta.annotation.error-line.log"
			)),
			"{error_line:?}"
		);
		let plain_line = log_scope_ops(&ANCHORED_LOG_SYNTAXES, "request served in 3 ms\n");
		assert_eq!(line_annotations(&plain_line), 0, "{plain_line:?}");
	}

	#[test]
	fn syntax_for_frontmatter_languages_resolve() {
		// Frontmatter rendering routes YAML (`---`) and TOML (`+++`)
		// blocks through the highlighter with these fixed hints; both
		// grammars must be present in the embedded syntax set.
		for token in ["yaml", "toml"] {
			assert!(syntax_for(Some(token)).is_some(), "expected syntax for {token:?} to resolve");
		}
	}
}
