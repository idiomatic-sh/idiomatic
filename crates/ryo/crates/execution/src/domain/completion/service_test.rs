//! Physical completeness survives file-frontier conversion.
use super::{service::file_frontier_coverage, types::CoverageState};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run, selects},
    domain::records::types::{FrontierAffected, FrontierCoverage},
};
struct FrontierCase {
    coverage: CoverageState,
    expected: FrontierCoverage,
}
type Input = FrontierCase;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("complete"),
        word: Word::Succeeds,
        input: || FrontierCase {
            coverage: CoverageState::Complete,
            expected: FrontierCoverage::Complete,
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("current_file"),
        word: Word::Succeeds,
        input: || FrontierCase {
            coverage: CoverageState::Partial {
                files: vec![Some(FileId(0))],
            },
            expected: FrontierCoverage::Partial(vec![FrontierAffected::CurrentFile]),
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("repository"),
        word: Word::Succeeds,
        input: || FrontierCase {
            coverage: CoverageState::Partial { files: vec![None] },
            expected: FrontierCoverage::Partial(vec![FrontierAffected::Repository]),
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("duplicate_causes"),
        word: Word::Succeeds,
        input: || {
            let mut causes = vec![FrontierAffected::CurrentFile, FrontierAffected::Repository];
            causes.sort_unstable();
            FrontierCase {
                coverage: CoverageState::Partial {
                    files: vec![Some(FileId(0)), Some(FileId(0)), None],
                },
                expected: FrontierCoverage::Partial(causes),
            }
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("foreign_file"),
        word: Word::Refuses,
        input: || FrontierCase {
            coverage: CoverageState::Partial {
                files: vec![Some(FileId(1))],
            },
            expected: FrontierCoverage::Complete,
        },
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, OperatingError::ObservationIncomplete { reason } if reason.as_str() == "file frontier carries a foreign physical partition"),
            )
        }),
    },
    Case {
        name: CaseName("empty_partial"),
        word: Word::Refuses,
        input: || FrontierCase {
            coverage: CoverageState::Partial { files: vec![] },
            expected: FrontierCoverage::Complete,
        },
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, OperatingError::ObservationIncomplete { reason } if reason.as_str() == "partial frontier has no affected physical partition"),
            )
        }),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let actual = file_frontier_coverage(input.coverage, FileId(0))?;
    assert_eq!(actual, input.expected);
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn contributor_partiality_filters_absence_answers_before_keys_can_be_projected_away() {
    use super::service::filter_contributor_rows;
    use crate::domain::operator::contract::service::semantics;
    use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, Input, StepId},
        plans::{service::kind_of, types::Operation},
        rows::types::{RowCount, Table},
    };
    let left = Input::Step(StepId(0));
    let right = Input::Step(StepId(1));
    let candidates = [
        Cell::File(FileId(1)),
        Cell::Number(Integer(10)),
        Cell::File(FileId(0)),
        Cell::Number(Integer(20)),
        Cell::File(FileId(0)),
        Cell::Number(Integer(20)),
        Cell::File(FileId(2)),
        Cell::Number(Integer(30)),
    ];
    let operations = [
        Operation::Antijoin {
            left,
            right,
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        },
        Operation::Difference { left, right },
        Operation::GroupCount {
            from: left,
            keys: vec![ColumnIndex(0)],
            distinct: None,
        },
        Operation::GroupDigest {
            from: left,
            keys: vec![ColumnIndex(0)],
            over: ColumnIndex(1),
        },
    ];
    for operation in operations {
        let (_, _, _, absence) = semantics(kind_of(&operation));
        let input_kinds = vec![vec![CellKind::File, CellKind::Number]; 2];
        let digest = matches!(operation, Operation::GroupDigest { .. });
        let extend_digest = |cells: &[Cell]| {
            cells
                .chunks(2)
                .flat_map(|row| {
                    let mut row = row.to_vec();
                    if digest {
                        row.push(Cell::Number(Integer(1)));
                    }
                    row
                })
                .collect::<Vec<_>>()
        };
        for affected in [vec![Some(FileId(2)), Some(FileId(1))], vec![None]] {
            let mut kinds = vec![CellKind::File, CellKind::Number];
            if digest {
                kinds.push(CellKind::Number);
            }
            let mut rows = Table {
                kinds,
                cells: extend_digest(&candidates),
                count: RowCount(4),
            };
            let contributions = input_kinds
                .iter()
                .enumerate()
                .map(|(ordinal, kinds)| super::types::InputCoverage {
                    ordinal,
                    kinds,
                    coverage: CoverageState::Partial {
                        files: affected.clone(),
                    },
                })
                .collect::<Vec<_>>();
            filter_contributor_rows(&operation, absence, &contributions, &mut rows);
            assert_eq!(
                rows.cells,
                if affected.contains(&None) {
                    vec![]
                } else {
                    extend_digest(&[
                        Cell::File(FileId(0)),
                        Cell::Number(Integer(20)),
                        Cell::File(FileId(0)),
                        Cell::Number(Integer(20)),
                    ])
                }
            );
        }
    }
    let global_count = Operation::GroupCount {
        from: left,
        keys: vec![],
        distinct: None,
    };
    let mut count = Table {
        kinds: vec![CellKind::Number],
        cells: vec![Cell::Number(Integer(4))],
        count: RowCount(1),
    };
    filter_contributor_rows(
        &global_count,
        1,
        &[super::types::InputCoverage {
            ordinal: 0,
            kinds: &[CellKind::File],
            coverage: CoverageState::Partial {
                files: vec![Some(FileId(1))],
            },
        }],
        &mut count,
    );
    assert_eq!(
        count.count,
        RowCount(0),
        "an unkeyed aggregate cannot localize uncertainty"
    );
}

#[test]
fn ordered_reduction_requires_a_file_partition_before_preserving_file_scoped_uncertainty() {
    use super::service::ordered_output_coverage;
    use super::types::InputCoverage;
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::plan_parts::types::ColumnIndex;
    let source = InputCoverage {
        ordinal: 0,
        kinds: &[CellKind::File, CellKind::Number],
        coverage: CoverageState::Partial {
            files: vec![Some(FileId(1))],
        },
    };
    assert_eq!(
        ordered_output_coverage(&[ColumnIndex(0)], &[CellKind::File], &source),
        source.coverage
    );
    assert_eq!(
        ordered_output_coverage(&[ColumnIndex(1)], &[CellKind::File], &source),
        CoverageState::Partial { files: vec![None] }
    );
}
