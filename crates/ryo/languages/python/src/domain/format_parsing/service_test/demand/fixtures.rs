//! The support of the group's tests.
use crate::domain::format_syntax::{
    interface::FormatSyntax,
    types::{
        FormatDialect, FormatErrorKind, FormatParseError, FormatParseResult, FormatSummary,
        ParsedFormat,
    },
};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) static COUNTING_PARSES: AtomicUsize = AtomicUsize::new(0);

pub(super) static MALFORMED_PARSES: AtomicUsize = AtomicUsize::new(0);

pub(super) struct CountingParser;

impl FormatSyntax for CountingParser {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-decoded");
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-raw");

    fn parse(
        _input: crate::domain::format_syntax::types::FormatInput<'_>,
        _dialect: FormatDialect,
    ) -> FormatParseResult {
        let _previous_count = COUNTING_PARSES.fetch_add(1, Ordering::SeqCst);
        FormatParseResult::Parsed(ParsedFormat {
            fields: Vec::new(),
            summary: FormatSummary::default(),
        })
    }
}

pub(super) struct MalformedParser;

impl FormatSyntax for MalformedParser {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-decoded");
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-raw");

    fn parse(
        _input: crate::domain::format_syntax::types::FormatInput<'_>,
        _dialect: FormatDialect,
    ) -> FormatParseResult {
        let _previous_count = MALFORMED_PARSES.fetch_add(1, Ordering::SeqCst);
        FormatParseResult::Malformed(FormatParseError {
            kind: FormatErrorKind::IncompleteFormat,
            character: None,
            index: crate::domain::format_syntax::types::FormatScalarIndex(0),
        })
    }
}

pub(super) struct UnavailableParser;

impl FormatSyntax for UnavailableParser {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-decoded");
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-raw");

    fn parse(
        _input: crate::domain::format_syntax::types::FormatInput<'_>,
        _dialect: FormatDialect,
    ) -> FormatParseResult {
        FormatParseResult::Unavailable
    }
}
