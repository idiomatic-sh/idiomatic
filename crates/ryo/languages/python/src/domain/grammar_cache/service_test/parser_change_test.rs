//! A parser implementation change invalidates completed active results.
use super::super::service;
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;
use crate::domain::format_syntax::interface::FormatSyntax;
use crate::domain::format_syntax::types::FormatDialect;
use crate::domain::format_syntax::types::FormatErrorKind;
use crate::domain::format_syntax::types::FormatParseError;
use crate::domain::format_syntax::types::FormatParseResult;
use crate::domain::format_syntax::types::FormatSummary;
use crate::domain::format_syntax::types::ParsedFormat;

struct ParsedGrammar;
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

struct RejectedGrammar;
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

fn completed_active_results_are_invalidated_when_parser_implementation_changes() {
    use crate::domain::{format_parsing::service::prepare, model::types::Declarations};
    use crate::fixtures::reading::{initialize_model, parsed};
    use ryo::languages::{self, ByteSlice, Symbols, Tree};
    let file = parsed(b"value = 'same input'\n").expect("source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    let mut symbols = Symbols::default();
    initialize_model(&tree, &mut model);
    assert!(matches!(
        prepare::<ParsedGrammar>(
            &tree,
            &model.format_parsing,
            &mut symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| (crate::domain::semantic_syntax::service::visibility(
                &tree, &model, at, symbols
            ) == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        )
        .values[0]
            .result,
        FormatParseResult::Parsed(_)
    ));
    assert!(matches!(
        prepare::<RejectedGrammar>(
            &tree,
            &model.format_parsing,
            &mut symbols,
            crate::domain::semantic_syntax::service::positions(&tree),
            |at, symbols| (crate::domain::semantic_syntax::service::visibility(
                &tree, &model, at, symbols
            ) == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        )
        .values[0]
            .result,
        FormatParseResult::Malformed(_)
    ));
}

/// Each check below states its own source and asserts its own outcome.
const CASES: &[Case<fn(), (), ()>] = &[Case {
    name: CaseName("completed_active_results_are_invalidated_when_parser_implementation_changes"),
    word: Word::Succeeds,
    input: || completed_active_results_are_invalidated_when_parser_implementation_changes,
    expect: Expect::Value(()),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |check| {
        check();
        Ok(())
    });
}
