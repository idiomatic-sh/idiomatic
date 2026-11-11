//! A minimal adapter of Ruff 0.16.5's `ruff_python_literal::cformat` parser.
//!
//! The upstream grammar is adapted from `crates/ruff_python_literal/src/cformat.rs` in the
//! Ruff 0.16.5 source tree (MIT license, Astral Software Inc. and contributors). This adapter
//! keeps only decoded string parsing and the fields needed to describe argument consumption;
//! parser-library types and lint policy do not cross this boundary.

use super::service::parse_percent_string;
use super::types::Failure;
use crate::domain::format_syntax::types::{FormatDialect, FormatInput, FormatParseResult};
use crate::outbound::format_syntax::interface::FormatSyntaxPort;

/// The parser used by the Python language composition.
pub(crate) struct RuffLiteralCFormat;

impl FormatSyntaxPort for RuffLiteralCFormat {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::constants::DECODED_GRAMMAR;
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::constants::RAW_GRAMMAR;

    fn parse(&self, input: FormatInput<'_>, dialect: FormatDialect) -> FormatParseResult {
        match dialect {
            FormatDialect::PercentString => match parse_percent_string(input) {
                Ok(parsed) => FormatParseResult::Parsed(parsed),
                Err(Failure::Malformed(error)) => FormatParseResult::Malformed(error),
                Err(Failure::Unavailable) => FormatParseResult::Unavailable,
            },
        }
    }
}
