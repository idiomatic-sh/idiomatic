//! The support of the group's tests.
use crate::domain::format_syntax::interface::FormatSyntax;
use crate::domain::format_syntax::types::FormatDialect;
use crate::domain::format_syntax::types::FormatErrorKind;
use crate::domain::format_syntax::types::FormatParseError;
use crate::domain::format_syntax::types::FormatParseResult;
use crate::domain::format_syntax::types::FormatSummary;
use crate::domain::format_syntax::types::ParsedFormat;

pub(super) struct ParsedGrammar;

impl FormatSyntax for ParsedGrammar {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-decoded");
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-raw");

    fn parse(
        _: crate::domain::format_syntax::types::FormatInput<'_>,
        _: FormatDialect,
    ) -> FormatParseResult {
        FormatParseResult::Parsed(ParsedFormat {
            fields: Vec::new(),
            summary: FormatSummary::default(),
        })
    }
}

pub(super) struct RejectedGrammar;

impl FormatSyntax for RejectedGrammar {
    const DECODED_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-decoded");
    const RAW_GRAMMAR: crate::domain::format_syntax::types::GrammarIdentity =
        crate::domain::format_syntax::types::GrammarIdentity("test-percent-raw");

    fn parse(
        _: crate::domain::format_syntax::types::FormatInput<'_>,
        _: FormatDialect,
    ) -> FormatParseResult {
        FormatParseResult::Malformed(FormatParseError {
            kind: FormatErrorKind::IncompleteFormat,
            character: None,
            index: crate::domain::format_syntax::types::FormatScalarIndex(0),
        })
    }
}
