//! The `walk` contract's cases over the relations' fixtures: for each relation the walk fills, the rows its filler writes for the
//! sources of the relation's fixture are the rows the fixture states, and the fill answers in full.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::today::{Read, as_stated, read, read_keys_all, stated_rows};
use crate::fixtures::walking::{Seen, walked, walked_keys, walked_rows};
use crate::outbound::syntax::fixtures::library::fixture_sources;
use ryo::languages::{self, NodeTable, Relation, WalkError};

/// A relation and the folder of its fixture.
type Input = (Relation, &'static str);
/// What differs, one line for each difference; empty when the rows are the fixture's.
type Value = String;
type Failure = WalkError;

/// The rows of the fixtures' cases: one for each relation the walk fills.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_filler_of_literal_properties_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::LiteralProperties, "literal_properties"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_numeric_literal_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::NumericLiteral, "numeric_literal"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_string_literal_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::StringLiteral, "string_literal"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_bytes_literal_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::BytesLiteral, "bytes_literal"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "the_filler_of_interpolated_string_literal_writes_the_rows_its_fixture_states",
        ),
        word: Word::Succeeds,
        input: || {
            (
                Relation::InterpolatedStringLiteral,
                "interpolated_string_literal",
            )
        },
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_identifier_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::Identifier, "identifier"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_static_import_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::StaticImport, "static_import"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_call_argument_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::CallArgument, "call_argument"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_function_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::Function, "function"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_parameter_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::Parameter, "parameter"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_decorates_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::Decorates, "decorates"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_inherits_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::Inherits, "inherits"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_language_target_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::LanguageTarget, "language_target"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_compiler_feature_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::CompilerFeature, "compiler_feature"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_comment_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::Comment, "comment"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_indent_region_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::IndentRegion, "indent_region"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_token_link_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::TokenLink, "token_link"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_source_location_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::SourceLocation, "source_location"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_format_parse_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::FormatParse, "format_parse"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_format_field_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::FormatField, "format_field"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_brace_format_parse_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::BraceFormatParse, "brace_format_parse"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_brace_format_field_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::BraceFormatField, "brace_format_field"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_raw_format_segment_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::RawFormatSegment, "raw_format_segment"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_raw_format_field_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::RawFormatField, "raw_format_field"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_syntax_error_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::SyntaxError, "syntax_error"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_standard_library_import_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::StandardLibraryImport, "standard_library_import"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_literal_expression_parse_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::LiteralExpressionParse, "literal_expression_parse"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_expression_shape_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::ExpressionShape, "expression_shape"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_expression_type_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::ExpressionType, "expression_type"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_constant_key_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::ConstantKey, "constant_key"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_function_terminal_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::FunctionTerminal, "function_terminal"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_format_specification_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::FormatSpecification, "format_specification"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_filler_of_embedded_expression_writes_the_rows_its_fixture_states"),
        word: Word::Succeeds,
        input: || (Relation::EmbeddedExpression, "embedded_expression"),
        expect: Expect::Value(String::new()),
    },
];

/// The rows of one side the other lacks, shown.
fn lacking_rows(side: &str, have: &[Vec<Seen>], other: &[Vec<Seen>]) -> String {
    have.iter()
        .filter(|each| !other.contains(each))
        .take(3)
        .map(|each| format!("only {side}: {each:?}\n"))
        .collect()
}

/// The name a fixture's rows show for one of its sources: `input.py` is `<folder>/input.py`, `input.<case>.py` is
/// `<folder>/<case>.py`.
fn shown_name(folder: &str, path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    let case = file
        .strip_prefix("input.")
        .and_then(|rest| rest.strip_suffix(".py"))
        .filter(|case| !case.is_empty());
    case.map_or_else(
        || format!("{folder}/input.py"),
        |case| format!("{folder}/{case}.py"),
    )
}

/// What differs between the rows the fixture in `folder` states for `relation` and the rows the walk's filler writes for each of
/// the fixture's sources; a fill that does not answer in full is a difference too.
fn row_differences(relation: Relation, folder: &str) -> Result<String, WalkError> {
    let prefix = format!("relations/{folder}/input");
    let stated = crate::fixtures::expected();
    let expected = stated
        .relations
        .iter()
        .find(|each| each.relation == folder)
        .ok_or(WalkError::Library)?;
    let linked = crate::linked();
    let reasons = crate::domain::language::constants::UNKNOWN_REASONS;
    let mut found = String::new();
    for (path, bytes) in fixture_sources()
        .map_err(|_unread| WalkError::Library)?
        .iter()
        .filter(|(path, _)| path.starts_with(&prefix))
    {
        let shown = shown_name(folder, path);
        // The rows this source gives: a row that names a file names the source it is of.
        let of_source = languages::contract::Expected {
            relation: expected.relation,
            rows: expected
                .rows
                .iter()
                .filter(|row| {
                    row.iter().all(|cell| {
                        !matches!(cell, languages::contract::Cell::File(name) if *name != shown)
                    })
                })
                .cloned()
                .collect(),
        };
        let mut today = Read::default();
        read(
            linked.parse_engine().as_ref(),
            "input.py",
            bytes,
            &[],
            &mut today,
        )
        .map_err(|_refused| WalkError::Library)?;
        let wanted = stated_rows(&of_source, bytes, &read_keys_all(&today.elements), reasons);
        let (mut table, mut scratch) = (NodeTable::default(), Default::default());
        walked(
            &crate::walk(),
            "input.py",
            bytes,
            &[relation],
            &mut table,
            &mut scratch,
            &mut |mut file| {
                let mut words = languages::facts::file_words(&languages::facts::base_words(
                    languages::CheckedNames::default(),
                    &[],
                ));
                let keys = walked_keys(file.table);
                let (written, answer) = walked_rows(&mut file, &keys, relation, &mut words);
                let written: Vec<Vec<Seen>> = written.into_iter().map(as_stated).collect();
                if !matches!(answer, languages::FactAnswer::Answered(_)) {
                    found.push_str(&format!("{shown}: the fill answered {answer:?}\n"));
                }
                found.push_str(&lacking_rows("stated", &wanted, &written));
                found.push_str(&lacking_rows("written", &written, &wanted));
                if found.is_empty() && wanted.len() != written.len() {
                    found.push_str(&format!(
                        "{shown}: {} rows stated, {} written\n",
                        wanted.len(),
                        written.len()
                    ));
                }
            },
        )?;
    }
    Ok(found)
}

fn subject(asked: Input) -> Result<Value, Failure> {
    row_differences(asked.0, asked.1)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
