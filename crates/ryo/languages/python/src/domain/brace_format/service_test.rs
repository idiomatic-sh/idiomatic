//! The `brace_format` service's cases: the state, occurrences and results it reads from a brace-format index, and its clearing.
use super::service::{clear, occurrences, result, state};
use super::types::{BraceFormatResult, BraceFormatState, CachedBraceFormat};
use crate::application::brace_format::service::prepare;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::format_syntax::types::RetainedFormatIndex;
use crate::domain::model::types::Declarations;
use crate::domain::semantic_syntax::{
    service::{positions, visibility},
    types::Visibility,
};
use crate::fixtures::reading::{initialize_model, parsed};
use crate::outbound::brace_format::adapters::ruff_literal_braceformat::grammar::RuffLiteralBraceFormat;
use ryo::languages::{self, ByteSlice, Tree};

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The index the operation leaves for a source.
    Prepared(&'static str),
    /// The same index after it is cleared.
    Cleared(&'static str),
}
/// What a row is given.
type Input = Given;
/// What the service read, joined by `|`.
type Value = &'static str;
/// A source the parser refuses.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_brace_string_has_an_occurrence_and_a_parsed_result"),
        word: Word::Succeeds,
        input: || Given::Prepared("'{0} {x!r}'.format(1, x=2)\n"),
        expect: Expect::Value("Complete|1|parsed"),
    },
    Case {
        name: CaseName("a_malformed_brace_string_keeps_its_result"),
        word: Word::Succeeds,
        input: || Given::Prepared("'{'.format()\n"),
        expect: Expect::Value("Complete|1|Malformed(UnmatchedBracket)"),
    },
    Case {
        name: CaseName("clearing_leaves_a_fresh_empty_index"),
        word: Word::Succeeds,
        input: || Given::Cleared("'{0}'.format(1)\n"),
        expect: Expect::Value("Fresh|0|"),
    },
];

/// Parses `source`, runs the `brace_format` operation over it and reads its index through this service.
fn prepared(source: &'static str, cleared: bool) -> Result<String, Failure> {
    let mut file = parsed(source.as_bytes()).map_err(|_| ())?;
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare::<RuffLiteralBraceFormat>(
        &tree,
        &model.brace_format,
        &mut file.symbols,
        positions(&tree),
        |node, symbols| (visibility(&tree, &model, node, symbols) == Visibility::Visible).into(),
    );
    let mut read = summary(&index);
    drop(index);
    if cleared {
        clear(model.brace_format.get_mut());
        read = summary(&model.brace_format.borrow());
    }
    Ok(read)
}

/// Answers the row.
fn subject(given: Input) -> Result<Value, Failure> {
    let read = match given {
        Given::Prepared(source) => prepared(source, false)?,
        Given::Cleared(source) => prepared(source, true)?,
    };
    Ok(Box::leak(read.into_boxed_str()))
}

/// The state, the occurrence count and each occurrence's result kind.
fn summary(
    index: &RetainedFormatIndex<BraceFormatResult, CachedBraceFormat, BraceFormatState>,
) -> String {
    let kinds: Vec<String> = occurrences(index)
        .iter()
        .map(|each| match result(index, each) {
            Some(BraceFormatResult::Parsed(_)) => "parsed".to_owned(),
            Some(other) => format!("{other:?}"),
            None => "none".to_owned(),
        })
        .collect();
    format!(
        "{:?}|{}|{}",
        state(index),
        occurrences(index).len(),
        kinds.join(",")
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
