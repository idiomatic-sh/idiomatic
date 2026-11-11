//! The `literal_expression_parsing` service's cases: the name of each outcome, and the state of a literal index before and after it is
//! cleared.
use super::service::{clear, outcome_name};
use super::types::{LiteralParseIndex, LiteralParseOutcome};
use crate::application::literal_expression_parsing::service::prepare;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::types::Declarations;
use crate::domain::semantic_syntax::{
    service::{positions, visibility},
    types::Visibility,
};
use crate::fixtures::reading::{initialize_model, parsed};
use crate::outbound::literal_expression::adapters::ruff_string_annotation::grammar::RuffStringAnnotation;
use ryo::languages::{self, ByteSlice, Tree};

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The index the operation leaves for a source.
    Prepared(&'static str),
    /// The same index after it is cleared.
    Cleared(&'static str),
    /// The name of every outcome.
    OutcomeNames,
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
        name: CaseName("each_outcome_has_its_name"),
        word: Word::Succeeds,
        input: || Given::OutcomeNames,
        expect: Expect::Value("parsed|malformed|unavailable"),
    },
    Case {
        name: CaseName("an_annotation_string_is_parsed_and_counted"),
        word: Word::Succeeds,
        input: || Given::Prepared("x: 'int'\ny: 'int +'\n"),
        expect: Expect::Value("Complete|parsed,malformed"),
    },
    Case {
        name: CaseName("clearing_leaves_a_fresh_empty_index"),
        word: Word::Succeeds,
        input: || Given::Cleared("x: 'int'\n"),
        expect: Expect::Value("Fresh|"),
    },
];

/// Parses `source`, runs the `literal_expression_parsing` operation over it and reads its index through this service.
fn prepared(source: &'static str, cleared: bool) -> Result<String, Failure> {
    let mut file = parsed(source.as_bytes()).map_err(|_| ())?;
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare::<RuffStringAnnotation>(
        &tree,
        &model.literal_expression_parsing,
        &mut file.symbols,
        positions(&tree),
        |node, symbols| (visibility(&tree, &model, node, symbols) == Visibility::Visible).into(),
    );
    let mut read = summary(&index);
    drop(index);
    if cleared {
        clear(model.literal_expression_parsing.get_mut());
        read = summary(&model.literal_expression_parsing.borrow());
    }
    Ok(read)
}

/// Answers the row.
fn subject(given: Input) -> Result<Value, Failure> {
    let read = match given {
        Given::Prepared(source) => prepared(source, false)?,
        Given::Cleared(source) => prepared(source, true)?,
        Given::OutcomeNames => [
            LiteralParseOutcome::Parsed,
            LiteralParseOutcome::Malformed,
            LiteralParseOutcome::Unavailable,
        ]
        .into_iter()
        .map(|each| outcome_name(each).0)
        .collect::<Vec<_>>()
        .join("|"),
    };
    Ok(Box::leak(read.into_boxed_str()))
}

/// The state, the occurrence count and each occurrence's outcome name.
fn summary(index: &LiteralParseIndex) -> String {
    let outcomes: Vec<&str> = index
        .occurrences
        .iter()
        .map(|each| outcome_name(each.outcome).0)
        .collect();
    format!("{:?}|{}", index.state, outcomes.join(","))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
