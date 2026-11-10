//! The `output_coverage` service's cases: how far a join, an ordered match and an ordered reduction carry what an operand lacks.
use super::types::OperandOrdinal;
use crate::domain::completion::service::partial_any;
use crate::domain::completion::types::CoverageState;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
#[cfg(test)]
mod lineage_test;

/// What a row of the cases proves.
#[derive(Clone, Copy)]
enum Scenario {
    /// Ordered reduction requires a file partition before preserving file scoped uncertainty.
    OrderedReductionRequiresAFilePartitionBeforePreservingFileScopedUncertainty,
    /// A rejected file withholds only its own partition from a join anchored by file.
    ARejectedFileWithholdsOnlyItsOwnPartitionFromAJoinAnchoredByFile,
    /// A rejected file withholds only its own partition from an ordered match keyed by file.
    ARejectedFileWithholdsOnlyItsOwnPartitionFromAnOrderedMatchKeyedByFile,
}

/// What a row of the cases asks.
type Input = Scenario;
type Value = ();
type Failure = ();

fn ordered_reduction_requires_a_file_partition_before_preserving_file_scoped_uncertainty() {
    use super::service::ordered_output_coverage;
    use crate::domain::output_coverage::types::InputCoverage;
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::plan_parts::types::ColumnIndex;
    let source = InputCoverage {
        ordinal: OperandOrdinal(0),
        kinds: &[CellKind::File, CellKind::Number],
        coverage: CoverageState::Partial {
            files: vec![Some(FileId(1))],
            lacks: vec![],
        },
        lineage: Default::default(),
    };
    assert_eq!(
        ordered_output_coverage(&[ColumnIndex(0)], &[CellKind::File], &source),
        source.coverage
    );
    assert_eq!(
        ordered_output_coverage(&[ColumnIndex(1)], &[CellKind::File], &source),
        partial_any(vec![None])
    );
}

/// Two files, one syntax-rejected (file 1): a join whose rows are anchored to the file it joins on withholds only the rejected
/// file's partition, so the other file's answer is complete; a join on a key that is no file leaves the missing rows anywhere.
fn a_rejected_file_withholds_only_its_own_partition_from_a_join_anchored_by_file() {
    use crate::domain::output_coverage::service::output_coverage;
    use crate::domain::output_coverage::types::InputCoverage;
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
                    ordinal: OperandOrdinal(0),
                    kinds: &left,
                    coverage: CoverageState::Complete,
                    lineage: Default::default(),
                },
                InputCoverage {
                    ordinal: OperandOrdinal(1),
                    kinds: &right,
                    coverage: missing,
                    lineage: Default::default(),
                },
            ],
        )
        .unwrap()
    };
    let rejected = || CoverageState::Partial {
        files: vec![Some(FileId(1))],
        lacks: vec![],
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
                files: vec![Some(FileId(0))],
                lacks: vec![],
            }
        ),
        CoverageState::Partial {
            files: vec![Some(FileId(0))],
            lacks: vec![],
        },
        "a genuinely partial valid file is still reported partial"
    );
    assert_eq!(
        coverage(&join(2, 1), rejected()),
        partial_any(vec![None]),
        "a key that is no file cannot place the missing rows"
    );
}

/// An ordered match pairs rows that agree on every key: keyed by file it withholds only the rejected file's partition, and keyed by
/// a symbol it cannot place the missing rows.
fn a_rejected_file_withholds_only_its_own_partition_from_an_ordered_match_keyed_by_file() {
    use crate::domain::output_coverage::service::output_coverage;
    use crate::domain::output_coverage::types::InputCoverage;
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
                ordinal: OperandOrdinal(0),
                kinds: &kinds,
                coverage: CoverageState::Partial {
                    files: vec![Some(FileId(1))],
                    lacks: vec![],
                },
                lineage: Default::default(),
            }],
        )
        .unwrap()
    };
    assert_eq!(
        coverage(&matched(1)),
        CoverageState::Partial {
            files: vec![Some(FileId(1))],
            lacks: vec![],
        },
        "the rejected file's partition alone is withheld"
    );
    assert_eq!(
        coverage(&matched(3)),
        partial_any(vec![None]),
        "a symbol key cannot place the missing rows"
    );
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "ordered_reduction_requires_a_file_partition_before_preserving_file_scoped_uncertainty",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::OrderedReductionRequiresAFilePartitionBeforePreservingFileScopedUncertainty
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "a_rejected_file_withholds_only_its_own_partition_from_a_join_anchored_by_file",
        ),
        word: Word::Succeeds,
        input: || Scenario::ARejectedFileWithholdsOnlyItsOwnPartitionFromAJoinAnchoredByFile,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "a_rejected_file_withholds_only_its_own_partition_from_an_ordered_match_keyed_by_file",
        ),
        word: Word::Succeeds,
        input: || Scenario::ARejectedFileWithholdsOnlyItsOwnPartitionFromAnOrderedMatchKeyedByFile,
        expect: Expect::Value(()),
    },
];

fn subject(input: Input) {
    match input {
        Scenario::OrderedReductionRequiresAFilePartitionBeforePreservingFileScopedUncertainty => {
            ordered_reduction_requires_a_file_partition_before_preserving_file_scoped_uncertainty()
        }
        Scenario::ARejectedFileWithholdsOnlyItsOwnPartitionFromAJoinAnchoredByFile => {
            a_rejected_file_withholds_only_its_own_partition_from_a_join_anchored_by_file()
        }
        Scenario::ARejectedFileWithholdsOnlyItsOwnPartitionFromAnOrderedMatchKeyedByFile => {
            a_rejected_file_withholds_only_its_own_partition_from_an_ordered_match_keyed_by_file()
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
