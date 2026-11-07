//! The `output_coverage` service's cases: how far a join, an ordered match and an ordered reduction carry what an operand lacks.
use ryo_relations::domain::relations::types::FileId;
use crate::domain::completion::types::CoverageState;
use crate::domain::completion::service::partial_any;
use super::types::OperandOrdinal;
use crate::domain::operating::errors::OperatingError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases proves.
#[derive(Clone, Copy)]
enum Scenario {
    /// Ordered reduction requires a file partition before preserving file scoped uncertainty.
    OrderedReductionRequiresAFilePartitionBeforePreservingFileScopedUncertainty,
    /// A rejected file withholds only its own partition from a join anchored by file.
    ARejectedFileWithholdsOnlyItsOwnPartitionFromAJoinAnchoredByFile,
    /// A rejected file withholds only its own partition from an ordered match keyed by file.
    ARejectedFileWithholdsOnlyItsOwnPartitionFromAnOrderedMatchKeyedByFile,
    /// A node key of a relation places the missing rows in the missing file.
    ANodeKeyOfARelationPlacesTheMissingRows,
    /// A node key of a step cannot place the missing rows.
    ANodeKeyOfAStepCannotPlaceTheMissingRows,
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
                },
                InputCoverage {
                    ordinal: OperandOrdinal(1),
                    kinds: &right,
                    coverage: missing,
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

/// What an operand of the join reads.
#[derive(Clone, Copy)]
enum Reads {
    Relation,
    Step,
}

/// Where the rows a join places missing from one file lie.
#[derive(Debug, PartialEq, Eq)]
enum Placement {
    /// In the one file that was missing them, any row of it.
    MissingFile(u32),
    /// In the whole repository.
    Anywhere,
}

/// A step may join the rows of file f with the nodes of file g, keep (f's file, g's node) and so carry a node column that does not
/// lie in the file its row is partitioned by; with no per-column lineage a step's file or node key places nothing, so the missing
/// file is never narrowed to its own partition, while the same key of a relation the operand reads places the rows in the missing file.
fn the_placement_of_a_node_key(operand: Reads) -> Result<Placement, OperatingError> {
    use super::service::output_coverage;
    use super::types::InputCoverage;
    use ryo_relations::domain::relations::types::{CellKind, Relation};
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, Input as PlanInput, JoinColumn, Side, StepId},
        plans::types::Operation,
    };
    let from = |at: u32| match operand {
        Reads::Relation => PlanInput::Relation(if at == 0 {
            Relation::Function
        } else {
            Relation::Identifier
        }),
        Reads::Step => PlanInput::Step(StepId(at)),
    };
    let join = Operation::Join {
        left: from(0),
        right: from(1),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
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
    let kinds = [CellKind::Node, CellKind::File];
    let coverage = output_coverage(
        &join,
        &kinds,
        &[
            InputCoverage {
                ordinal: OperandOrdinal(0),
                kinds: &kinds,
                coverage: CoverageState::Complete,
            },
            InputCoverage {
                ordinal: OperandOrdinal(1),
                kinds: &kinds,
                coverage: crate::domain::completion::service::partial_any(vec![Some(FileId(1))]),
            },
        ],
    )?;
    match coverage {
        CoverageState::Partial { files, .. } if files == [Some(FileId(1))] => {
            Ok(Placement::MissingFile(1))
        }
        CoverageState::Partial { files, .. } if files == [None] => Ok(Placement::Anywhere),
        CoverageState::Complete
        | CoverageState::Partial { .. }
        | CoverageState::Inapplicable { .. } => Err(OperatingError::WrongOperation),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("ordered_reduction_requires_a_file_partition_before_preserving_file_scoped_uncertainty"),
        word: Word::Succeeds,
        input: || Scenario::OrderedReductionRequiresAFilePartitionBeforePreservingFileScopedUncertainty,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_rejected_file_withholds_only_its_own_partition_from_a_join_anchored_by_file"),
        word: Word::Succeeds,
        input: || Scenario::ARejectedFileWithholdsOnlyItsOwnPartitionFromAJoinAnchoredByFile,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_rejected_file_withholds_only_its_own_partition_from_an_ordered_match_keyed_by_file"),
        word: Word::Succeeds,
        input: || Scenario::ARejectedFileWithholdsOnlyItsOwnPartitionFromAnOrderedMatchKeyedByFile,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_node_key_of_a_relation_places_the_missing_rows_in_the_missing_file"),
        word: Word::Succeeds,
        input: || Scenario::ANodeKeyOfARelationPlacesTheMissingRows,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_node_key_of_a_step_cannot_place_the_missing_rows"),
        word: Word::Succeeds,
        input: || Scenario::ANodeKeyOfAStepCannotPlaceTheMissingRows,
        expect: Expect::Value(()),
    },
];

fn subject(input: Input) {
    match input {
        Scenario::OrderedReductionRequiresAFilePartitionBeforePreservingFileScopedUncertainty => ordered_reduction_requires_a_file_partition_before_preserving_file_scoped_uncertainty(),
        Scenario::ARejectedFileWithholdsOnlyItsOwnPartitionFromAJoinAnchoredByFile => a_rejected_file_withholds_only_its_own_partition_from_a_join_anchored_by_file(),
        Scenario::ARejectedFileWithholdsOnlyItsOwnPartitionFromAnOrderedMatchKeyedByFile => a_rejected_file_withholds_only_its_own_partition_from_an_ordered_match_keyed_by_file(),
        Scenario::ANodeKeyOfARelationPlacesTheMissingRows => assert_eq!(
            the_placement_of_a_node_key(Reads::Relation),
            Ok(Placement::MissingFile(1))
        ),
        Scenario::ANodeKeyOfAStepCannotPlaceTheMissingRows => assert_eq!(
            the_placement_of_a_node_key(Reads::Step),
            Ok(Placement::Anywhere)
        ),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
