//! Physical completeness survives file-frontier conversion.
use super::service::file_frontier_coverage;
use crate::domain::completion::service::partial_any;
use crate::domain::completion::types::CoverageState;
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::selects;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::records::types::{FrontierAffected, FrontierCoverage, FrontierMissing},
};
struct FrontierCase {
    coverage: CoverageState,
    expected: FrontierCoverage,
}
/// What a row of the cases converts.
enum Conversion {
    /// A coverage, and the frontier coverage it becomes.
    Coverage(FrontierCase),
    /// A partial read kept as a frontier proof and restored.
    Restored,
    /// A kind partial cleared by a select that stops the kind.
    KindPartial,
}
/// What a row of the cases asks.
type Input = Conversion;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_kind_partial_is_cleared_by_a_select_that_stops_the_kind_and_by_nothing_else"),
        word: Word::Succeeds,
        input: || Conversion::KindPartial,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_restored_frontier_proof_clears_for_an_untouched_kind_and_stays_for_a_touched_one_in_its_own_file"),
        word: Word::Succeeds,
        input: || Conversion::Restored,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("complete"),
        word: Word::Succeeds,
        input: || Conversion::Coverage(FrontierCase {
            coverage: CoverageState::Complete,
            expected: FrontierCoverage::Complete,
        }),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("current_file"),
        word: Word::Succeeds,
        input: || Conversion::Coverage(FrontierCase {
            coverage: CoverageState::Partial {
                files: vec![Some(FileId(0))],
                lacks: vec![],
            },
            expected: FrontierCoverage::Partial(vec![FrontierAffected::CurrentFile(
                FrontierMissing::Any,
            )]),
        }),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("repository"),
        word: Word::Succeeds,
        input: || Conversion::Coverage(FrontierCase {
            coverage: partial_any(vec![None]),
            expected: FrontierCoverage::Partial(vec![FrontierAffected::Repository(
                FrontierMissing::Any,
            )]),
        }),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("duplicate_causes"),
        word: Word::Succeeds,
        input: || {
            let mut causes = vec![
                FrontierAffected::CurrentFile(FrontierMissing::Any),
                FrontierAffected::Repository(FrontierMissing::Any),
            ];
            causes.sort_unstable();
            Conversion::Coverage(FrontierCase {
                coverage: CoverageState::Partial {
                    files: vec![Some(FileId(0)), Some(FileId(0)), None],
                    lacks: vec![],
                },
                expected: FrontierCoverage::Partial(causes),
            })
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("foreign_file"),
        word: Word::Refuses,
        input: || Conversion::Coverage(FrontierCase {
            coverage: CoverageState::Partial {
                files: vec![Some(FileId(1))],
                lacks: vec![],
            },
            expected: FrontierCoverage::Complete,
        }),
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, OperatingError::ObservationIncomplete { reason } if reason.as_str() == "file frontier carries a foreign physical partition"),
            )
        }),
    },
    Case {
        name: CaseName("empty_partial"),
        word: Word::Refuses,
        input: || Conversion::Coverage(FrontierCase {
            coverage: partial_any(vec![]),
            expected: FrontierCoverage::Complete,
        }),
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, OperatingError::ObservationIncomplete { reason } if reason.as_str() == "partial frontier has no affected physical partition"),
            )
        }),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Conversion::Coverage(case) => {
            let actual = file_frontier_coverage(case.coverage, FileId(0))?;
            assert_eq!(actual, case.expected);
            Ok(())
        }
        Conversion::KindPartial => {
            a_kind_partial_is_cleared_by_a_select_that_stops_the_kind_and_by_nothing_else();
            Ok(())
        }
        Conversion::Restored => {
            a_restored_frontier_proof_clears_for_an_untouched_kind_and_stays_for_a_touched_one_in_its_own_file();
            Ok(())
        }
    }
}


fn a_restored_frontier_proof_clears_for_an_untouched_kind_and_stays_for_a_touched_one_in_its_own_file() {
    use super::service::restored_lacks;
    use crate::domain::completion::service::partial_of_lacks;
    use crate::domain::completion::types::{FileLacks, MissingRows};
    use crate::domain::output_coverage::service::output_coverage;
    use crate::domain::output_coverage::types::{InputCoverage, OperandOrdinal};
    use crate::domain::coverage::types::LackingKind;
    use ryo_relations::domain::relations::types::{CellKind, Relation};
    use ryo_values::domain::{
        plan_parts::types::{
            ColumnIndex, ComparisonOrigin, Input, LiteralText, Operand, PlanComparison,
            QueryLiteral, StepId, Test,
        },
        plans::types::Operation,
    };
    let kind = crate::domain::trust::service::kind_place(Relation::Defines).unwrap();
    let kinds = [CellKind::Node, CellKind::Symbol, CellKind::Symbol];
    let lacks = |file: u32| FileLacks {
        file: Some(FileId(file)),
        missing: MissingRows::Kinds {
            column: kind,
            kinds: vec![LackingKind("variable".into())],
        },
    };
    let stored = super::service::file_frontier_coverage(partial_of_lacks(vec![lacks(3)]), FileId(3)).unwrap();
    assert_eq!(
        stored,
        FrontierCoverage::Partial(vec![FrontierAffected::CurrentFile(FrontierMissing::Kinds {
            column: kind,
            kinds: vec![ryo_values::domain::stored_facts::types::PartialKind("variable".into())],
        })]),
        "the proof carries the kinds"
    );
    let FrontierCoverage::Partial(affected) = stored else {
        panic!("a partial proof");
    };
    let restored = partial_of_lacks(restored_lacks(FileId(3), &affected));
    assert_eq!(restored, partial_of_lacks(vec![lacks(3)]));
    let select = |text: &str| Operation::Select {
        from: Input::Step(StepId(0)),
        comparisons: vec![PlanComparison {
            test: Test::Equal,
            left: Operand::Column(ColumnIndex(kind.0)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: Vec::new(),
    };
    let through = |text: &str| {
        output_coverage(
            &select(text),
            &kinds,
            &[InputCoverage {
                ordinal: OperandOrdinal(0),
                kinds: &kinds,
                coverage: restored.clone(),
            }],
        )
        .unwrap()
    };
    assert_eq!(through("enum"), CoverageState::Complete);
    assert_eq!(through("variable"), restored);
    // A proof of another file names that file only.
    assert_eq!(
        partial_of_lacks(restored_lacks(FileId(4), &affected)),
        partial_of_lacks(vec![lacks(4)])
    );
}
/// A select, a project and a join over a relation partly read for rows of one kind in file 1 and for any row in file 0: a select that
/// stops the missing kind clears file 1 only, one that lets it through keeps it, and no operator that is not a select clears either.
fn a_kind_partial_is_cleared_by_a_select_that_stops_the_kind_and_by_nothing_else() {
    use crate::domain::output_coverage::service::output_coverage;
    use crate::domain::output_coverage::types::InputCoverage;
    use crate::domain::completion::types::{FileLacks, MissingRows};
    use crate::domain::coverage::types::LackingKind;
    use ryo_relations::domain::relations::types::{CellKind, Relation};
    use ryo_values::domain::{
        plan_parts::types::{
            ColumnIndex, ComparisonOrigin, Input, JoinColumn, LiteralText, Operand, PlanComparison,
            QueryLiteral, RepeatedRows, Side, StepId, Test,
        },
        plans::types::Operation,
    };
    let kind = crate::domain::trust::service::kind_place(Relation::Defines).unwrap();
    let kinds = [CellKind::Node, CellKind::Symbol, CellKind::Symbol];
    let missing = |file: u32, name: &str| FileLacks {
        file: Some(FileId(file)),
        missing: MissingRows::Kinds {
            column: kind,
            kinds: vec![LackingKind(name.into())],
        },
    };
    let partial = |lacks: Vec<FileLacks>| crate::domain::completion::service::partial_of_lacks(lacks);
    let select = |test: Test, text: &str, column: ColumnIndex| Operation::Select {
        from: Input::Step(StepId(0)),
        comparisons: vec![PlanComparison {
            test,
            left: Operand::Column(column),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
            origin: ComparisonOrigin::Unrecorded,
        }],
        kept: Vec::new(),
    };
    let through_at =
        |ordinal: usize, operation: &Operation, coverage: CoverageState, out: &[CellKind]| {
            output_coverage(
                operation,
                out,
                &[InputCoverage {
                    ordinal: crate::domain::output_coverage::types::OperandOrdinal(ordinal),
                    kinds: &kinds,
                    coverage,
                }],
            )
            .unwrap()
        };
    let through = |operation: &Operation, coverage: CoverageState, out: &[CellKind]| {
        through_at(0, operation, coverage, out)
    };
    // File 1 lacks `variable` rows only; file 0 lacks any row.
    let both = || CoverageState::Partial {
        files: vec![Some(FileId(0)), Some(FileId(1))],
        lacks: vec![missing(1, "variable")],
    };
    let only_variable = || partial(vec![missing(1, "variable")]);
    assert_eq!(
        through(&select(Test::Equal, "enum", kind), only_variable(), &kinds),
        CoverageState::Complete,
        "a select on another kind never reads the missing rows"
    );
    assert_eq!(
        through(
            &select(Test::Equal, "variable", kind),
            only_variable(),
            &kinds
        ),
        only_variable(),
        "a select that keeps the missing kind stays partial for that file"
    );
    assert_eq!(
        through(
            &select(Test::Equal, "enum", ColumnIndex(kind.0 + 1)),
            only_variable(),
            &kinds
        ),
        only_variable(),
        "a select on a column that is not the kind column rules nothing out"
    );
    assert_eq!(
        through(&select(Test::Equal, "enum", kind), both(), &kinds),
        partial_any(vec![Some(FileId(0))]),
        "the file that lacks any row stays partial and is the only one: never the repository"
    );
    assert_eq!(
        through(
            &Operation::Project {
                from: Input::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Dropped,
            },
            only_variable(),
            &kinds[..1],
        ),
        partial_any(vec![Some(FileId(1))]),
        "a projection that drops the kind column no longer tells which rows are missing"
    );
    // A join on a column that is no file or node places the missing rows nowhere: the whole repository.
    let join = Operation::Join {
        left: Input::Step(StepId(0)),
        right: Input::Step(StepId(1)),
        left_on: vec![ColumnIndex(1)],
        right_on: vec![ColumnIndex(1)],
        output: vec![JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        }],
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    };
    assert_eq!(
        through_at(1, &join, only_variable(), &kinds[..1]),
        partial_any(vec![None]),
        "a join keyed by anything else still widens, whatever kind is missing"
    );
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
