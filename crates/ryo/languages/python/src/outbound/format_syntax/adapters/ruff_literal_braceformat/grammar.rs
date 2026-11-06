//! A minimal adapter of Ruff 0.16.5's `ruff_python_literal::format` grammar.
//!
//! The field and brace grammar is adapted from `crates/ruff_python_literal/src/format.rs`
//! (MIT license, Astral Software Inc. and contributors). This adapter keeps only field roots,
//! nesting, summary counts and generic parser categories needed by Python brace-format readers.
use super::service::parse;
use crate::domain::brace_format::interface::BraceFormatSyntax;
use crate::domain::brace_format::types::BraceFormatResult;
use crate::domain::format_syntax::types::FormatInput;

/// The parser selected for decoded Python brace-format strings.
pub(crate) struct RuffLiteralBraceFormat;

impl BraceFormatSyntax for RuffLiteralBraceFormat {
    const GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::brace_format::constants::GRAMMAR;

    fn parse(input: FormatInput<'_>) -> BraceFormatResult {
        match parse(input) {
            Ok(parsed) => BraceFormatResult::Parsed(parsed),
            Err(error) => BraceFormatResult::Malformed(error),
        }
    }
}
