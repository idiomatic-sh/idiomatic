//! The support of the group's tests.
use crate::domain::format_syntax::{
    interface::FormatSyntax,
    types::{FormatDialect, FormatParseResult},
};
use crate::outbound::format_syntax::adapters::ruff_literal_cformat::grammar::RuffLiteralCFormat;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) static PARSES: AtomicUsize = AtomicUsize::new(0);

pub(super) struct Counting;

impl FormatSyntax for Counting {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-decoded");
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-raw");

    fn parse(
        input: crate::domain::format_syntax::types::FormatInput<'_>,
        dialect: FormatDialect,
    ) -> FormatParseResult {
        let _previous_count = PARSES.fetch_add(1, Ordering::SeqCst);
        RuffLiteralCFormat::parse(input, dialect)
    }
}

pub(super) struct Unavailable;

impl FormatSyntax for Unavailable {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-decoded");
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-raw");

    fn parse(
        input: crate::domain::format_syntax::types::FormatInput<'_>,
        dialect: FormatDialect,
    ) -> FormatParseResult {
        if input.0 == "%s" {
            FormatParseResult::Unavailable
        } else {
            RuffLiteralCFormat::parse(input, dialect)
        }
    }
}
