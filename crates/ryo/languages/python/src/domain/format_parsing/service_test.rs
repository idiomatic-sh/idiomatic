//! The `format_parsing` service's cases: which nodes are string occurrences, their decoded text, and the state, occurrences and results
//! it reads from a percent-format index, and its clearing.
use super::service::{clear, decoded_into, is_string_occurrence, occurrences, result, state};
use crate::application::format_parsing::service::prepare;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::format_syntax::types::{
    CachedFormat, FormatParseResult, FormatParseState, FormatScratch, RetainedFormatIndex,
    StringOccurrence,
};
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
    /// Every string occurrence of a source and its decoded text.
    Strings(&'static str),
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
        name: CaseName("a_percent_string_has_an_occurrence_and_a_parsed_result"),
        word: Word::Succeeds,
        input: || Given::Prepared("'%s %d' % (a, b)\n"),
        expect: Expect::Value("Complete|1|parsed"),
    },
    Case {
        name: CaseName("clearing_leaves_a_fresh_empty_index"),
        word: Word::Succeeds,
        input: || Given::Cleared("'%s' % a\n"),
        expect: Expect::Value("Fresh|0|"),
    },
    Case {
        name: CaseName("strings_decode_escapes_fstrings_are_dynamic_and_bytes_are_no_occurrence"),
        word: Word::Succeeds,
        input: || Given::Strings("'a\\\\tb'\nb'x'\nf'{v}'\n"),
        expect: Expect::Value("Ok(Static):\"a\\\\tb\"|Ok(Dynamic):\"\""),
    },
];

/// Parses `source`, runs the `format_parsing` operation over it and reads its index through this service.
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
        &model.format_parsing,
        &mut file.symbols,
        positions(&tree),
        |node, symbols| (visibility(&tree, &model, node, symbols) == Visibility::Visible).into(),
    );
    let mut read = summary(&index);
    drop(index);
    if cleared {
        clear(model.format_parsing.get_mut());
        read = summary(&model.format_parsing.borrow());
    }
    Ok(read)
}

/// Answers the row.
fn subject(given: Input) -> Result<Value, Failure> {
    let read = match given {
        Given::Prepared(source) => prepared(source, false)?,
        Given::Cleared(source) => prepared(source, true)?,
        Given::Strings(source) => {
            let file = parsed(source.as_bytes()).map_err(|_| ())?;
            let tree = Tree {
                elements: &file.elements,
                source: ByteSlice(file.bytes),
                file: languages::FileId(0),
            };
            positions(&tree)
                .filter(|at| is_string_occurrence(&tree, *at) == StringOccurrence::String)
                .map(|at| {
                    let mut text = String::new();
                    let decoded = decoded_into(&tree, at, FormatScratch(&mut text));
                    format!("{decoded:?}:{text:?}")
                })
                .collect::<Vec<_>>()
                .join("|")
        }
    };
    Ok(Box::leak(read.into_boxed_str()))
}

/// The state, the occurrence count and each occurrence's result kind.
fn summary(
    index: &RetainedFormatIndex<FormatParseResult, CachedFormat, FormatParseState>,
) -> String {
    let kinds: Vec<String> = occurrences(index)
        .iter()
        .map(|each| match result(index, each) {
            Some(FormatParseResult::Parsed(_)) => "parsed".to_owned(),
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
