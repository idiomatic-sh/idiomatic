//! The `output_coverage` service's cases of where a join or an anti-join places what one file lacks: through the key of a relation
//! it reads, never through a step's key alone, and through the operands' lineage.
use crate::domain::completion::service::partial_any;
use crate::domain::completion::types::CoverageState;
use crate::domain::operating::errors::OperatingError;
use crate::domain::output_coverage::types::OperandOrdinal;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::lineage::types::Lineage;
use ryo_values::domain::plan_parts::types::Side;

/// What a row of the cases proves.
#[derive(Clone, Copy)]
enum Scenario {
    /// A node key of a relation places the missing rows in the missing file.
    ANodeKeyOfARelationPlacesTheMissingRows,
    /// A node key of a step cannot place the missing rows.
    ANodeKeyOfAStepCannotPlaceTheMissingRows,
    /// Effect 1 of the six-rule unknowns: a cross-file step joined to a per-file step by node and file withholds only the rejected file.
    ACrossFileStepJoinedToAPerFileStepByItsFileWithholdsOnlyTheRejectedFile,
    /// Effect 2: a cross-file step joined on id and file to a relation handed over as a per-file step withholds only the rejected file.
    ACrossFileStepJoinedToAHandedOverRelationWithholdsOnlyTheRejectedFile,
    /// The kept rule: lineage that pairs no key lying in each row's file places nothing.
    ALineageNotKeyedByFileCannotPlaceTheMissingRows,
}

/// What a row of the cases asks.
type Input = Scenario;
type Value = ();
type Failure = ();

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
    use crate::domain::output_coverage::service::output_coverage;
    use crate::domain::output_coverage::types::InputCoverage;
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
                lineage: Default::default(),
            },
            InputCoverage {
                ordinal: OperandOrdinal(1),
                kinds: &kinds,
                coverage: crate::domain::completion::service::partial_any(vec![Some(FileId(1))]),
                lineage: Default::default(),
            },
        ],
    )?;
    match coverage {
        CoverageState::Partial { files, .. } if files == [Some(FileId(1))] => {
            Ok(Placement::MissingFile(1))
        }
        // A partial that names the repository is partial anywhere, whatever file it names besides (`affected_ordinals`).
        CoverageState::Partial { files, .. } if files.contains(&None) => Ok(Placement::Anywhere),
        CoverageState::Complete
        | CoverageState::Partial { .. }
        | CoverageState::Inapplicable { .. } => Err(OperatingError::WrongOperation),
    }
}

/// Where a join of two steps keyed on `key` places what the rejected file 1 lacks from the right (and the left when `both`).
fn the_placement_through_lineage(
    left: Lineage,
    right: Lineage,
    key: &[u32],
    from: ryo_values::domain::plan_parts::types::Side,
    both: bool,
) -> Result<Placement, OperatingError> {
    use crate::domain::output_coverage::service::output_coverage;
    use crate::domain::output_coverage::types::InputCoverage;
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, Input as PlanInput, JoinColumn, StepId},
        plans::types::Operation,
    };
    let keys: Vec<ColumnIndex> = key.iter().copied().map(ColumnIndex).collect();
    let output = (0..2)
        .map(|column| JoinColumn {
            side: from,
            column: ColumnIndex(column),
        })
        .collect();
    let join = Operation::Join {
        left: PlanInput::Step(StepId(0)),
        right: PlanInput::Step(StepId(1)),
        left_on: keys.clone(),
        right_on: keys,
        output,
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    };
    let kinds = [CellKind::Node, CellKind::File, CellKind::Number];
    let rejected = || partial_any(vec![Some(FileId(1))]);
    let inputs = [
        InputCoverage {
            ordinal: OperandOrdinal(0),
            kinds: &kinds,
            coverage: if both {
                rejected()
            } else {
                CoverageState::Complete
            },
            lineage: left,
        },
        InputCoverage {
            ordinal: OperandOrdinal(1),
            kinds: &kinds,
            coverage: rejected(),
            lineage: right,
        },
    ];
    match output_coverage(&join, &[CellKind::Node, CellKind::File], &inputs)? {
        CoverageState::Partial { files, .. } if files == [Some(FileId(1))] => {
            Ok(Placement::MissingFile(1))
        }
        // A partial that names the repository is partial anywhere, whatever file it names besides (`affected_ordinals`).
        CoverageState::Partial { files, .. } if files.contains(&None) => Ok(Placement::Anywhere),
        CoverageState::Complete
        | CoverageState::Partial { .. }
        | CoverageState::Inapplicable { .. } => Err(OperatingError::WrongOperation),
    }
}

/// Where an anti-join of a step (left, lineage `left`) and a per-file step missing the rejected file 1 (right) keyed on `key`
/// places what the right lacks: the left rows a missing right row would keep out.
fn the_placement_of_an_antijoin(left: Lineage, key: &[u32]) -> Result<Placement, OperatingError> {
    use crate::domain::output_coverage::service::output_coverage;
    use crate::domain::output_coverage::types::InputCoverage;
    use ryo_relations::domain::relations::types::CellKind;
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, Input as PlanInput, StepId},
        plans::types::Operation,
    };
    let keys: Vec<ColumnIndex> = key.iter().copied().map(ColumnIndex).collect();
    let antijoin = Operation::Antijoin {
        left: PlanInput::Step(StepId(0)),
        right: PlanInput::Step(StepId(1)),
        left_on: keys.clone(),
        right_on: keys,
    };
    let kinds = [CellKind::Node, CellKind::File, CellKind::Number];
    let inputs = [
        InputCoverage {
            ordinal: OperandOrdinal(0),
            kinds: &kinds,
            coverage: CoverageState::Complete,
            lineage: left,
        },
        InputCoverage {
            ordinal: OperandOrdinal(1),
            kinds: &kinds,
            coverage: partial_any(vec![Some(FileId(1))]),
            lineage: one_file(),
        },
    ];
    match output_coverage(&antijoin, &kinds, &inputs)? {
        CoverageState::Partial { files, .. } if files == [Some(FileId(1))] => {
            Ok(Placement::MissingFile(1))
        }
        CoverageState::Partial { files, .. } if files.contains(&None) => Ok(Placement::Anywhere),
        CoverageState::Complete
        | CoverageState::Partial { .. }
        | CoverageState::Inapplicable { .. } => Err(OperatingError::WrongOperation),
    }
}

/// The lineage of node, file and number rows that come from one file each: a relation's, or a per-file step's.
fn one_file() -> Lineage {
    ryo_values::domain::lineage::service::one_file(&[
        ryo_relations::domain::relations::types::CellKind::Node,
        ryo_relations::domain::relations::types::CellKind::File,
        ryo_relations::domain::relations::types::CellKind::Number,
    ])
}

const CASES: &[Case<Input, Value, Failure>] = &[
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
    Case {
        name: CaseName(
            "a_cross_file_step_joined_to_a_per_file_step_by_its_file_withholds_only_the_rejected_file",
        ),
        word: Word::Succeeds,
        input: || Scenario::ACrossFileStepJoinedToAPerFileStepByItsFileWithholdsOnlyTheRejectedFile,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "a_cross_file_step_joined_to_a_handed_over_relation_withholds_only_the_rejected_file",
        ),
        word: Word::Succeeds,
        input: || Scenario::ACrossFileStepJoinedToAHandedOverRelationWithholdsOnlyTheRejectedFile,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_lineage_not_keyed_by_file_cannot_place_the_missing_rows"),
        word: Word::Succeeds,
        input: || Scenario::ALineageNotKeyedByFileCannotPlaceTheMissingRows,
        expect: Expect::Value(()),
    },
];

fn subject(input: Input) {
    match input {
        Scenario::ANodeKeyOfARelationPlacesTheMissingRows => assert_eq!(
            the_placement_of_a_node_key(Reads::Relation),
            Ok(Placement::MissingFile(1))
        ),
        Scenario::ANodeKeyOfAStepCannotPlaceTheMissingRows => assert_eq!(
            the_placement_of_a_node_key(Reads::Step),
            Ok(Placement::Anywhere)
        ),
        Scenario::ACrossFileStepJoinedToAPerFileStepByItsFileWithholdsOnlyTheRejectedFile => {
            assert_eq!(
                the_placement_through_lineage(one_file(), one_file(), &[0, 1], Side::Left, true),
                Ok(Placement::MissingFile(1))
            );
            assert_eq!(
                the_placement_through_lineage(
                    Lineage::Mixed,
                    one_file(),
                    &[0, 1],
                    Side::Right,
                    true
                ),
                Ok(Placement::Anywhere),
                "without the cross-file step's lineage its missing rows place nothing in the other operand's columns"
            );
            assert_eq!(
                the_placement_through_lineage(
                    Lineage::Mixed,
                    one_file(),
                    &[0, 1],
                    Side::Left,
                    true
                ),
                Ok(Placement::MissingFile(1)),
                "the per-file step's file and node keys carry its missing file into the cross-file step's columns"
            );
        }
        Scenario::ACrossFileStepJoinedToAHandedOverRelationWithholdsOnlyTheRejectedFile => {
            for (from, both) in [
                (Side::Left, true),
                (Side::Left, false),
                (Side::Right, true),
                (Side::Right, false),
            ] {
                assert_eq!(
                    the_placement_through_lineage(one_file(), one_file(), &[0, 1], from, both),
                    Ok(Placement::MissingFile(1))
                );
            }
        }
        Scenario::ALineageNotKeyedByFileCannotPlaceTheMissingRows => assert_eq!(
            the_placement_through_lineage(one_file(), one_file(), &[2], Side::Left, false),
            Ok(Placement::Anywhere),
            "a number key pairs two files' rows"
        ),
    }
}

/// The anti-join rows: the key of a row, and where the anti-join of a cross-file left step and a per-file right step missing the
/// rejected file 1 places what the right lacks, whatever the left operand's lineage.
const ANTIJOIN: &[Case<&'static [u32], Placement, OperatingError>] = &[
    Case {
        name: CaseName(
            "an_antijoin_keyed_by_the_missing_steps_file_withholds_only_the_rejected_file",
        ),
        word: Word::Succeeds,
        input: || &[0, 1],
        expect: Expect::Value(Placement::MissingFile(1)),
    },
    Case {
        name: CaseName("an_antijoin_keyed_by_the_file_alone_cannot_place_the_left_node_column"),
        word: Word::Succeeds,
        input: || &[1],
        expect: Expect::Value(Placement::Anywhere),
    },
    Case {
        name: CaseName("an_antijoin_keyed_by_a_number_cannot_place_the_missing_rows"),
        word: Word::Succeeds,
        input: || &[2],
        expect: Expect::Value(Placement::Anywhere),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
    run(ANTIJOIN, Word::Succeeds, |key| {
        the_placement_of_an_antijoin(Lineage::Mixed, key)
    });
}
