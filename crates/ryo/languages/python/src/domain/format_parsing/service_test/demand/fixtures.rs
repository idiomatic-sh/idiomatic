//! The support of the group's tests.
use super::super::super::service::{prepare, result, state};
use crate::domain::format_syntax::{
    interface::FormatSyntax,
    types::{
        FormatDialect, FormatErrorKind, FormatParseError, FormatParseResult, FormatSummary,
        ParsedFormat,
    },
};
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::{initialize_model, parsed};
use ryo::languages::{self, ByteSlice, Symbols, Tree};
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

pub(super) fn duplicate_inputs_are_parsed_once_and_retained_until_source_reset() -> String {
    let mut observed: Vec<String> = Vec::new();
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
    observed.push(format!("{:?}", state(&model.format_parsing.borrow())));
    observed.push(format!("{:?}", COUNTING_PARSES.load(Ordering::SeqCst)));

    let visited = std::cell::Cell::new(0_usize);
    let visible = std::cell::Cell::new(0_usize);
    let first = prepare::<CountingParser>(
        &tree,
        &model.format_parsing,
        &mut symbols,
        crate::domain::semantic_syntax::service::positions(&tree)
            .inspect(|_| visited.set(visited.get() + 1)),
        |at, symbols| {
            visible.set(visible.get() + 1);
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    observed.push(format!("{:?}", COUNTING_PARSES.load(Ordering::SeqCst)));
    observed.push(format!("{:?}", first.occurrences.len()));
    observed.push(format!("{:?}", visible.get()));
    observed.push(format!("{:?}", result(&first, &first.occurrences[0])));
    drop(first);
    let prepared_visits = visited.get();
    let second = prepare::<CountingParser>(
        &tree,
        &model.format_parsing,
        &mut symbols,
        crate::domain::semantic_syntax::service::positions(&tree)
            .inspect(|_| visited.set(visited.get() + 1)),
        |at, symbols| {
            visible.set(visible.get() + 1);
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    observed.push(format!("{:?}", COUNTING_PARSES.load(Ordering::SeqCst)));
    observed.push(format!("{:?}", visited.get()));
    observed.push(format!("{:?}", visible.get()));
    drop(second);

    let replacement = parsed(b"third = '%d'\n").expect("replacement source parses");
    let replacement_tree = Tree {
        elements: &replacement.elements,
        source: ByteSlice(replacement.bytes),
        file: languages::FileId(0),
    };
    initialize_model(&replacement_tree, &mut model);
    observed.push(format!("{:?}", state(&model.format_parsing.borrow())));
    let _again = prepare::<CountingParser>(
        &replacement_tree,
        &model.format_parsing,
        &mut symbols,
        crate::domain::semantic_syntax::service::positions(&replacement_tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(
                &replacement_tree,
                &model,
                at,
                symbols,
            ) == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    observed.push(format!("{:?}", COUNTING_PARSES.load(Ordering::SeqCst)));
    observed.join("|")
}

pub(super) fn malformed_results_are_cached_and_unavailable_results_mark_partial() -> String {
    let mut observed: Vec<String> = Vec::new();
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
    let malformed = prepare::<MalformedParser>(
        &tree,
        &model.format_parsing,
        &mut symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    observed.push(format!("{:?}", MALFORMED_PARSES.load(Ordering::SeqCst)));
    observed.push(format!("{:?}", malformed.state));
    observed.push(format!("{:?}", malformed.values.len()));
    observed.push(format!(
        "{}",
        matches!(&malformed.values[0].result, FormatParseResult::Malformed(_))
    ));
    drop(malformed);

    crate::domain::format_parsing::service::clear(model.format_parsing.get_mut());
    let unavailable = prepare::<UnavailableParser>(
        &tree,
        &model.format_parsing,
        &mut symbols,
        crate::domain::semantic_syntax::service::positions(&tree),
        |at, symbols| {
            (crate::domain::semantic_syntax::service::visibility(&tree, &model, at, symbols)
                == crate::domain::semantic_syntax::types::Visibility::Visible)
                .into()
        },
    );
    observed.push(format!("{:?}", unavailable.state));
    observed.push(format!("{:?}", unavailable.values.len()));
    observed.push(format!("{:?}", &unavailable.values[0].result));
    observed.join("|")
}
