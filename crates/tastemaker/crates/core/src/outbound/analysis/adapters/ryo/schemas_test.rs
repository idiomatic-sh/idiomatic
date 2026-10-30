//! What the ryo adapter's schemas bind a param to, and what they refuse of a rule.

use super::{cell, parameter, placed, request};
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::applications::types::PlannedRule;
use crate::domain::applications::types::{GlobText, LintRuleId};
use crate::domain::definitions::types::QueryText;
use crate::domain::entries::types::Value as Param;
use crate::domain::files::types::MachinePath;
use crate::domain::scalars::types::{TextValue, ValueKey, WholeValue};
use crate::outbound::analysis::errors::AnalysisError;
use crate::outbound::analysis::types::{AnalysisRequest, Corpus};

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    Numbers,
    Texts,
    Sets,
    Tables,
    BadGlob,
    Cells,
    /// The `spec` cell of a placed row, with a spec and without one.
    PlacedSpec,
}

/// The question a row asks.
type Input = Question;
/// What the param binds, as its debug text, when it is not the one the row expects; empty when it is.
type Value = String;
type Failure = AnalysisError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_list_of_whole_numbers_binds_numbers"),
        word: Word::Succeeds,
        input: || Question::Numbers,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_list_of_text_binds_texts"),
        word: Word::Succeeds,
        input: || Question::Texts,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_list_of_lists_binds_sets_of_text_in_order"),
        word: Word::Succeeds,
        input: || Question::Sets,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_list_of_tables_binds_rows_of_text_by_key"),
        word: Word::Succeeds,
        input: || Question::Tables,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "a_cell_is_read_by_the_kind_the_sdk_typed_it_and_a_string_that_looks_like_a_span_stays_text",
        ),
        word: Word::Succeeds,
        input: || Question::Cells,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName(
            "a_placed_row_of_no_spec_crosses_as_an_empty_spec_cell_and_one_of_a_spec_as_its_name",
        ),
        word: Word::Succeeds,
        input: || Question::PlacedSpec,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_glob_that_does_not_compile_is_refused"),
        word: Word::Refuses,
        input: || Question::BadGlob,
        expect: Expect::Refusal(|error| {
            if matches!(error, AnalysisError::Rule { .. }) {
                Selection::Matches
            } else {
                Selection::Differs
            }
        }),
    },
];

fn subject(question: Input) -> Result<Value, Failure> {
    let (said, expected) = match question {
        Question::Numbers => (
            format!(
                "{:?}",
                parameter(&Param::List(vec![Param::Whole(WholeValue(1))]))
            ),
            "Numbers(ParameterNumbers([1]))",
        ),
        Question::Texts => (
            format!(
                "{:?}",
                parameter(&Param::List(vec![Param::Text(TextValue("a".into()))]))
            ),
            "Texts([ParameterText(\"a\")])",
        ),
        Question::Sets => (
            format!(
                "{:?}",
                parameter(&Param::List(vec![
                    Param::List(vec![Param::Text(TextValue("app".into()))]),
                    Param::List(vec![Param::Text(TextValue("leaf".into()))]),
                ]))
            ),
            "TextSets([[ParameterText(\"app\")], [ParameterText(\"leaf\")]])",
        ),
        Question::Tables => (
            format!(
                "{:?}",
                parameter(&Param::List(vec![Param::Table(
                    [(
                        ValueKey("path".into()),
                        Param::Text(TextValue("a::B".into()))
                    )]
                    .into_iter()
                    .collect()
                )]))
            ),
            "Tables([ParameterTable([ParameterCell { key: ParameterText(\"path\"), value: ParameterText(\"a::B\") }])])",
        ),
        Question::Cells => {
            use ryo::linter::{Cell, HostNumber, HostText, RelativePath, Slot};
            let file = crate::domain::paths::types::RepoPath("src/a.rs".into());
            let cell_of = |value: Slot| {
                cell(
                    &file,
                    crate::domain::definitions::types::ColumnName("c".into()),
                    value,
                )
                .value
            };
            let said = format!(
                "{:?}|{:?}|{:?}|{:?}|{:?}",
                cell_of(Slot::Is(Cell::Text(HostText("12..40".into())))),
                cell_of(Slot::Is(Cell::File(RelativePath("src/a.rs".into())))),
                cell_of(Slot::Is(Cell::File(RelativePath("src/b.rs".into())))),
                cell_of(Slot::Is(Cell::Number(HostNumber(7)))),
                cell_of(Slot::Nothing),
            );
            (
                said,
                "Text(CellText(\"12..40\"))|File(RepoPath(\"src/a.rs\"))|Text(CellText(\"src/b.rs\"))|Text(CellText(\"7\"))|Text(CellText(\"\"))",
            )
        }
        Question::PlacedSpec => placed_spec(),
        Question::BadGlob => {
            drop(request(AnalysisRequest {
                root: MachinePath("/repo".into()),
                rules: vec![PlannedRule {
                    id: LintRuleId("rule-0".into()),
                    query: QueryText(
                        "let x = select * from \"comment\"; emit x subject \"file\" key(\"span\");"
                            .into(),
                    ),
                    parameters: std::collections::BTreeMap::new(),
                    include: vec![GlobText("[".into())],
                    exclude: Vec::new(),
                }],
                placed: Vec::new(),
                corpus: Corpus::Walked,
                beside: Vec::new(),
                hosted: crate::outbound::analysis::types::Hosted::Nothing,
            })?);
            (String::new(), "")
        }
    };
    Ok(if said == expected {
        String::new()
    } else {
        said
    })
}

/// The `spec` cell of a placed row of no spec and of one of a spec, and what each should be.
fn placed_spec() -> (String, &'static str) {
    use crate::domain::applications::types::{PlacedRow, PlacingSpec};
    use crate::domain::packages::types::{DefinitionName, PackageName, VersionText};
    let row = |spec: Option<&str>| PlacedRow {
        path: crate::domain::paths::types::RepoPath("src/a.txt".into()),
        entry: DefinitionName("p/errors".into()),
        package: PackageName("p".into()),
        template: DefinitionName("errors".into()),
        version: VersionText("0.1.0".into()),
        spec: spec.map_or(PlacingSpec::Workspace, |name| {
            PlacingSpec::Instance(DefinitionName(name.into()))
        }),
    };
    let relation = placed(&[row(None), row(Some("part"))]);
    let said = relation
        .rows
        .iter()
        .map(|each| format!("{:?}", each.0.last()))
        .collect::<Vec<_>>()
        .join("|");
    (
        said,
        "Some(Text(HostText(\"\")))|Some(Text(HostText(\"part\")))",
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
