//! Cache and demand guards for the generic decoded-format reader service.
use super::super::super::service::{prepare, result, state};
use super::fixtures::{
    COUNTING_PARSES, CountingParser, MALFORMED_PARSES, MalformedParser, UnavailableParser,
};
use crate::domain::format_syntax::types::{FormatParseResult, FormatParseState};
use crate::domain::model::types::Declarations;
use crate::fixtures::reading::{initialize_model, parsed};
use ryo::languages::{self, ByteSlice, Symbols, Tree};
use std::sync::atomic::Ordering;

#[test]
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
    assert_eq!(COUNTING_PARSES.load(Ordering::SeqCst), 1);
    assert_eq!(first.occurrences.len(), 2);
    assert_eq!(
        visible.get(),
        2,
        "only candidate strings ask for visibility"
    );
    assert_eq!(
        result(&first, &first.occurrences[0]),
        result(&first, &first.occurrences[1])
    );
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
    assert_eq!(
        COUNTING_PARSES.load(Ordering::SeqCst),
        1,
        "both readers share the retained parse"
    );
    assert_eq!(
        visited.get(),
        prepared_visits,
        "prepared indexes do not traverse again"
    );
    assert_eq!(
        visible.get(),
        2,
        "prepared indexes do not repeat semantic visibility"
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
    assert_eq!(
        COUNTING_PARSES.load(Ordering::SeqCst),
        2,
        "source reset invalidates cached values"
    );
}

#[test]
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
    assert_eq!(MALFORMED_PARSES.load(Ordering::SeqCst), 1);
    assert_eq!(malformed.state, FormatParseState::Complete);
    assert_eq!(malformed.values.len(), 1);
    assert!(matches!(
        &malformed.values[0].result,
        FormatParseResult::Malformed(_)
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
    assert_eq!(unavailable.state, FormatParseState::Partial);
    assert_eq!(unavailable.values.len(), 1);
    assert_eq!(
        &unavailable.values[0].result,
        &FormatParseResult::Unavailable
    );
}
