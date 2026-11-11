//! The `raw_format` service's cases: the body of a raw token, and the state, segments and results it reads from a raw-format index, and its
//! clearing.
use super::service::{body, clear, result, segments, state};
use super::types::{RawFormatIndex, RawFormatToken};
use crate::application::raw_format::service::prepare;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::types::Declarations;
use crate::domain::semantic_syntax::{
    service::{positions, visibility},
    types::Visibility,
};
use crate::fixtures::reading::{initialize_model, parsed};
use crate::outbound::format_syntax::adapters::ruff_literal_cformat::grammar::RuffLiteralCFormat;
use ryo::languages::{self, ByteSlice, Tree};

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The index the operation leaves for a source.
    Prepared(&'static str),
    /// The same index after it is cleared.
    Cleared(&'static str),
    /// The body of a raw string token.
    Body(&'static str),
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
        name: CaseName("a_percent_string_has_segments_with_results"),
        word: Word::Succeeds,
        input: || Given::Prepared("'%s' 'x%d' % (a, b)\n"),
        expect: Expect::Value("Complete|2/2"),
    },
    Case {
        name: CaseName("clearing_leaves_a_fresh_empty_index"),
        word: Word::Succeeds,
        input: || Given::Cleared("'%s' % a\n"),
        expect: Expect::Value("Fresh|"),
    },
    Case {
        name: CaseName("a_bytes_token_has_no_format_body"),
        word: Word::Succeeds,
        input: || Given::Body("rb'%d'"),
        expect: Expect::Value("None"),
    },
    Case {
        name: CaseName("a_triple_quoted_token_has_its_inner_body"),
        word: Word::Succeeds,
        input: || Given::Body("'''%s'''"),
        expect: Expect::Value("Some(\"%s\")"),
    },
];

/// Parses `source`, runs the `raw_format` operation over it and reads its index through this service.
fn prepared(source: &'static str, cleared: bool) -> Result<String, Failure> {
    let mut file = parsed(source.as_bytes()).map_err(|_| ())?;
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare(
        &RuffLiteralCFormat,
        &tree,
        &model.raw_format,
        &mut file.symbols,
        positions(&tree),
        |node, symbols| (visibility(&tree, &model, node, symbols) == Visibility::Visible).into(),
    );
    let mut read = summary(&index);
    drop(index);
    if cleared {
        clear(model.raw_format.get_mut());
        read = summary(&model.raw_format.borrow());
    }
    Ok(read)
}

/// Answers the row.
fn subject(given: Input) -> Result<Value, Failure> {
    let read = match given {
        Given::Prepared(source) => prepared(source, false)?,
        Given::Cleared(source) => prepared(source, true)?,
        Given::Body(token) => format!("{:?}", body(RawFormatToken(token)).map(|each| each.0)),
    };
    Ok(Box::leak(read.into_boxed_str()))
}

/// The state, the segments of each occurrence and how many of them have a result.
fn summary(index: &RawFormatIndex) -> String {
    let counts: Vec<String> = index
        .occurrences
        .iter()
        .map(|each| {
            let all = segments(index, each);
            let read = all
                .iter()
                .filter(|segment| result(index, segment).is_some())
                .count();
            format!("{}/{read}", all.len())
        })
        .collect();
    format!("{:?}|{}", state(index), counts.join(","))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
