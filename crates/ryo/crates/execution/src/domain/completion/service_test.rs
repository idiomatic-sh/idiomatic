//! Physical completeness survives file-frontier conversion.
use super::{service::file_frontier_coverage, types::CoverageState};
use crate::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::selects;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
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
            selects!(
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
            selects!(
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

/// A count keyed by a later column of its operand answers where its key sits in the answer, whichever schema the caller probes with.
#[test]
fn a_grouped_answer_withholds_only_the_partitions_of_the_affected_files() {
    use crate::domain::coverage::service::drop_uncertain;
    use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, Input, StepId},
        plans::types::Operation,
        rows::types::{RowCount, Table},
    };
    let operation = Operation::GroupCount {
        from: Input::Step(StepId(0)),
        keys: vec![ColumnIndex(1)],
        distinct: None,
    };
    let answer = || Table {
        kinds: vec![CellKind::File, CellKind::Number],
        cells: (0..3)
            .flat_map(|file| [Cell::File(FileId(file)), Cell::Number(Integer(1))])
            .collect(),
        count: RowCount(3),
    };
    let operand = [CellKind::Number, CellKind::File];
    for probed in [&operand[..], &answer().kinds[..]] {
        let mut skipped = answer();
        drop_uncertain(&operation, probed, &mut skipped, &[Some(FileId(1))]);
        assert_eq!(skipped.count, RowCount(2), "the other partitions answer");
        assert!(
            !skipped.cells.contains(&Cell::File(FileId(1))),
            "the partition of the skipped file is withheld, not answered"
        );
        let mut repository = answer();
        drop_uncertain(&operation, probed, &mut repository, &[None]);
        assert_eq!(
            repository.count,
            RowCount(0),
            "an unlocated cause withholds all"
        );
    }
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

/// A file no language reads, or whose syntax a rule skipped, is out of scope for a group not keyed by file and costs a group keyed
/// by file only its own rows; it is never reported complete for the rows keyed to it.
#[test]
fn an_inapplicable_file_withholds_nothing_from_a_symbol_keyed_group_and_only_its_own_rows_from_a_file_keyed_one()
 {
    use super::service::{filter_contributor_rows, output_coverage};
    use super::types::InputCoverage;
    use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
    use ryo_relations::domain::terms::types::Symbol;
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, Input, StepId},
        plans::types::Operation,
        rows::types::{RowCount, Table},
    };
    let group = |key: u32| Operation::GroupCount {
        from: Input::Step(StepId(0)),
        keys: vec![ColumnIndex(key)],
        distinct: None,
    };
    let operand = [CellKind::Symbol, CellKind::File];
    let coverage = || CoverageState::Inapplicable {
        files: vec![FileId(1)],
    };
    let by_symbol = Table {
        kinds: vec![CellKind::Symbol, CellKind::Number],
        cells: (0..3)
            .flat_map(|name| [Cell::Symbol(Symbol(name)), Cell::Number(Integer(1))])
            .collect(),
        count: RowCount(3),
    };
    let mut kept = by_symbol.clone();
    filter_contributor_rows(
        &group(0),
        1,
        &[InputCoverage {
            ordinal: 0,
            kinds: &operand,
            coverage: coverage(),
        }],
        &mut kept,
    );
    assert_eq!(
        kept.count,
        RowCount(3),
        "a symbol-keyed group withholds nothing"
    );
    assert_eq!(
        output_coverage(
            &group(0),
            &kept.kinds,
            &[InputCoverage {
                ordinal: 0,
                kinds: &operand,
                coverage: coverage(),
            }],
        )
        .unwrap(),
        CoverageState::Complete,
        "an answer with no file to be keyed by is not made partial by an inapplicable file"
    );
    let by_file = Table {
        kinds: vec![CellKind::File, CellKind::Number],
        cells: (0..3)
            .flat_map(|file| [Cell::File(FileId(file)), Cell::Number(Integer(1))])
            .collect(),
        count: RowCount(3),
    };
    let mut kept = by_file.clone();
    filter_contributor_rows(
        &group(1),
        1,
        &[InputCoverage {
            ordinal: 0,
            kinds: &operand,
            coverage: coverage(),
        }],
        &mut kept,
    );
    assert_eq!(
        kept.count,
        RowCount(2),
        "a file-keyed group withholds its own rows"
    );
    assert!(!kept.cells.contains(&Cell::File(FileId(1))));
    assert_eq!(
        output_coverage(
            &group(1),
            &kept.kinds,
            &[InputCoverage {
                ordinal: 0,
                kinds: &operand,
                coverage: coverage(),
            }],
        )
        .unwrap(),
        coverage(),
        "the rows keyed to the file stay inapplicable, never complete"
    );
    let absence = Operation::Antijoin {
        left: Input::Step(StepId(0)),
        right: Input::Step(StepId(1)),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
    };
    let mut checked = by_symbol.clone();
    filter_contributor_rows(
        &absence,
        2,
        &[InputCoverage {
            ordinal: 1,
            kinds: &operand,
            coverage: coverage(),
        }],
        &mut checked,
    );
    assert_eq!(
        checked.count,
        RowCount(0),
        "a skipped file is still unavailable for an absence check"
    );
}

/// Two files, one syntax-rejected (file 1): a join whose rows are anchored to the file it joins on withholds only the rejected
/// file's partition, so the other file's answer is complete; a join on a key that is no file leaves the missing rows anywhere.
#[test]
fn a_rejected_file_withholds_only_its_own_partition_from_a_join_anchored_by_file() {
    use super::service::output_coverage;
    use super::types::InputCoverage;
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, Input, JoinColumn, Side, StepId},
        plans::types::Operation,
    };
    let read = |at: u32| {
        Input::Relation(if at == 0 {
            ryo_relations::domain::relations::types::Relation::Function
        } else {
            ryo_relations::domain::relations::types::Relation::Identifier
        })
    };
    let join = |left_on: u32, right_on: u32| Operation::Join {
        left: read(0),
        right: read(1),
        left_on: vec![ColumnIndex(left_on)],
        right_on: vec![ColumnIndex(right_on)],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(1),
            },
        ],
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    };
    let left = [CellKind::Node, CellKind::File, CellKind::Symbol];
    let right = [CellKind::File, CellKind::Symbol];
    let output = [CellKind::Node, CellKind::File];
    let coverage = |operation: &Operation, missing: CoverageState| {
        output_coverage(
            operation,
            &output,
            &[
                InputCoverage {
                    ordinal: 0,
                    kinds: &left,
                    coverage: CoverageState::Complete,
                },
                InputCoverage {
                    ordinal: 1,
                    kinds: &right,
                    coverage: missing,
                },
            ],
        )
        .unwrap()
    };
    let rejected = || CoverageState::Partial {
        files: vec![Some(FileId(1))],
    };
    assert_eq!(
        coverage(&join(1, 0), rejected()),
        rejected(),
        "the rejected file's partition alone is withheld, never the whole repository"
    );
    assert_eq!(
        coverage(
            &join(1, 0),
            CoverageState::Partial {
                files: vec![Some(FileId(0))]
            }
        ),
        CoverageState::Partial {
            files: vec![Some(FileId(0))]
        },
        "a genuinely partial valid file is still reported partial"
    );
    assert_eq!(
        super::service::file_frontier_coverage(CoverageState::Complete, FileId(0)).unwrap(),
        ryo_values::domain::records::types::FrontierCoverage::Complete,
        "a complete read of the valid file stays complete"
    );
    assert_eq!(
        coverage(&join(2, 1), rejected()),
        CoverageState::Partial { files: vec![None] },
        "a key that is no file cannot place the missing rows"
    );
}

/// An ordered match pairs rows that agree on every key: keyed by file it withholds only the rejected file's partition, and keyed by
/// a symbol it cannot place the missing rows.
#[test]
fn a_rejected_file_withholds_only_its_own_partition_from_an_ordered_match_keyed_by_file() {
    use super::service::output_coverage;
    use super::types::InputCoverage;
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, Input, JoinColumn, Side, StepId},
        plans::types::Operation,
    };
    let matched = |key: u32| Operation::OrderedMatch {
        left: Input::Relation(ryo_relations::domain::relations::types::Relation::Function),
        right: Input::Relation(ryo_relations::domain::relations::types::Relation::Function),
        left_on: vec![ColumnIndex(key)],
        right_on: vec![ColumnIndex(key)],
        left_order: ColumnIndex(2),
        right_order: ColumnIndex(2),
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(1),
            },
        ],
    };
    let kinds = [
        CellKind::Node,
        CellKind::File,
        CellKind::Number,
        CellKind::Symbol,
    ];
    let coverage = |operation: &Operation| {
        output_coverage(
            operation,
            &[CellKind::Node, CellKind::File],
            &[InputCoverage {
                ordinal: 0,
                kinds: &kinds,
                coverage: CoverageState::Partial {
                    files: vec![Some(FileId(1))],
                },
            }],
        )
        .unwrap()
    };
    assert_eq!(
        coverage(&matched(1)),
        CoverageState::Partial {
            files: vec![Some(FileId(1))]
        },
        "the rejected file's partition alone is withheld"
    );
    assert_eq!(
        coverage(&matched(3)),
        CoverageState::Partial { files: vec![None] },
        "a symbol key cannot place the missing rows"
    );
}
