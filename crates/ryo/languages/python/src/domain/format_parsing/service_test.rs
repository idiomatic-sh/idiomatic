//! Cache and demand guards for the generic decoded-format reader service.
use super::service::{prepare, result, state};
use crate::domain::format_syntax::{
    interface::FormatSyntax,
    types::{
        FormatDialect, FormatErrorKind, FormatParseError, FormatParseResult, FormatParseState,
        FormatSummary, ParsedFormat,
    },
};
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::{initialize_model, parsed};
use ryo::languages::{self, ByteSlice, Symbols, Tree};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTING_PARSES: AtomicUsize = AtomicUsize::new(0);
static MALFORMED_PARSES: AtomicUsize = AtomicUsize::new(0);

struct CountingParser;

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

struct MalformedParser;

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

struct UnavailableParser;

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

fn duplicate_inputs_are_parsed_once_and_retained_until_source_reset() {
    COUNTING_PARSES.store(0, Ordering::SeqCst);
    let file = parsed(b"first = '%s'\nsecond = '%s'\n").expect("source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let mut symbols = Symbols::default();
    assert_eq!(
        state(&model.format_parsing.borrow()),
        FormatParseState::Fresh
    );
    assert_eq!(
        COUNTING_PARSES.load(Ordering::SeqCst),
        0,
        "model setup does not parse formats"
    );

    let first = prepare::<CountingParser>(&tree, &model, &mut symbols);
    assert_eq!(COUNTING_PARSES.load(Ordering::SeqCst), 1);
    assert_eq!(first.occurrences.len(), 2);
    assert_eq!(
        result(&first, &first.occurrences[0]),
        result(&first, &first.occurrences[1])
    );
    drop(first);
    let second = prepare::<CountingParser>(&tree, &model, &mut symbols);
    assert_eq!(
        COUNTING_PARSES.load(Ordering::SeqCst),
        1,
        "both readers share the retained parse"
    );
    drop(second);

    let replacement = parsed(b"third = '%d'\n").expect("replacement source parses");
    let replacement_tree = Tree {
        elements: &replacement.elements,
        source: ByteSlice(replacement.bytes),
        file: languages::FileId(0),
    };
    initialize_model(&replacement_tree, &mut model);
    assert_eq!(
        state(&model.format_parsing.borrow()),
        FormatParseState::Fresh,
        "source replacement clears the retained format index"
    );
    let _again = prepare::<CountingParser>(&replacement_tree, &model, &mut symbols);
    assert_eq!(
        COUNTING_PARSES.load(Ordering::SeqCst),
        2,
        "source reset invalidates cached values"
    );
}

fn malformed_results_are_cached_and_unavailable_results_mark_partial() {
    MALFORMED_PARSES.store(0, Ordering::SeqCst);
    let file = parsed(b"first = '%*'\nsecond = '%*'\n").expect("source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let mut symbols = Symbols::default();
    let malformed = prepare::<MalformedParser>(&tree, &model, &mut symbols);
    assert_eq!(MALFORMED_PARSES.load(Ordering::SeqCst), 1);
    assert_eq!(malformed.state, FormatParseState::Complete);
    assert_eq!(malformed.values.len(), 1);
    assert!(matches!(
        &malformed.values[0].result,
        FormatParseResult::Malformed(_)
    ));
    drop(malformed);

    crate::domain::format_parsing::service::clear(model.format_parsing.get_mut());
    let unavailable = prepare::<UnavailableParser>(&tree, &model, &mut symbols);
    assert_eq!(unavailable.state, FormatParseState::Partial);
    assert_eq!(unavailable.values.len(), 1);
    assert_eq!(
        &unavailable.values[0].result,
        &FormatParseResult::Unavailable
    );
}

use crate::cases::{Case, CaseName, Expect, Word, run};
type Scenario = fn();
type Failure = std::convert::Infallible;
const CASES: &[Case<Scenario, (), Failure>] = &[
    Case {
        name: CaseName("duplicate_inputs_are_parsed_once_and_retained_until_source_reset"),
        word: Word::Succeeds,
        input: || duplicate_inputs_are_parsed_once_and_retained_until_source_reset as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("malformed_results_are_cached_and_unavailable_results_mark_partial"),
        word: Word::Succeeds,
        input: || malformed_results_are_cached_and_unavailable_results_mark_partial as fn(),
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Scenario) -> Result<(), Failure> {
    scenario();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
