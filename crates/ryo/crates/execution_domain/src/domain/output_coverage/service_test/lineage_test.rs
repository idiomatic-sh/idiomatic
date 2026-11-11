//! The `output_coverage` service's cases of where a join or an anti-join places what one file lacks: through the key of a relation
//! it reads, never through a step's key alone, and through the operands' lineage.
use crate::domain::completion::service::partial_any;
use crate::domain::completion::types::CoverageState;
use crate::domain::operating::errors::OperatingError;
use crate::domain::output_coverage::service::output_coverage;
use crate::domain::output_coverage::types::{InputCoverage, OperandOrdinal};
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::lineage::types::Lineage;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, JoinColumn, Side, StepId},
    plans::types::Operation,
};

/// What an operand of the join reads.
#[derive(Clone, Copy, Debug)]
enum Reads {
    Relation,
    Step,
}

/// The lineage of a step's node, file and number columns.
#[derive(Clone, Copy, Debug)]
enum Lines {
    /// Each row comes from one file, as a relation's or a per-file step's.
    OneFile,
    /// The rows mix files, as a cross-file step's.
    Mixed,
}

/// What a row asks: where the rows missing from the rejected file 1 are placed.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// By a join on the node column of two operands that read this, the right missing file 1.
    NodeKey(Reads),
    /// By a join of two steps of these lineages on these keys, answering the first two columns of this side, the right missing
    /// file 1 (and the left too when the last is true).
    Join(Lines, Lines, &'static [u32], Side, bool),
    /// By an anti-join on these keys of a cross-file left step and a per-file right step missing file 1.
    Antijoin(&'static [u32]),
}

/// Where the rows a join places missing from one file lie.
#[derive(Debug, PartialEq, Eq)]
enum Placement {
    /// In the one file that was missing them, any row of it.
    MissingFile(u32),
    /// In the whole repository.
    Anywhere,
    /// Nowhere the cases name: a complete or inapplicable answer, or a partial of other files.
    Elsewhere,
}

type Input = Ask;
type Value = Placement;
type Failure = OperatingError;

/// Where a coverage places the missing rows.
fn placement(coverage: CoverageState) -> Placement {
    match coverage {
        CoverageState::Partial { files, .. } if files == [Some(FileId(1))] => {
            Placement::MissingFile(1)
        }
        // A partial that names the repository is partial anywhere, whatever file it names besides (`affected_ordinals`).
        CoverageState::Partial { files, .. } if files.contains(&None) => Placement::Anywhere,
        CoverageState::Complete
        | CoverageState::Partial { .. }
        | CoverageState::Inapplicable { .. } => Placement::Elsewhere,
    }
}

fn lineage(lines: Lines) -> Lineage {
    match lines {
        Lines::OneFile => one_file(),
        Lines::Mixed => Lineage::Mixed,
    }
}

/// A step may join the rows of file f with the nodes of file g, keep (f's file, g's node) and so carry a node column that does not
/// lie in the file its row is partitioned by; with no per-column lineage a step's file or node key places nothing, so the missing
/// file is never narrowed to its own partition, while the same key of a relation the operand reads places the rows in the missing file.
fn the_placement_of_a_node_key(operand: Reads) -> Result<Placement, OperatingError> {
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
    output_coverage(
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
                coverage: partial_any(vec![Some(FileId(1))]),
                lineage: Default::default(),
            },
        ],
    )
    .map(placement)
}

/// Where a join of two steps keyed on `key` places what the rejected file 1 lacks from the right (and the left when `both`).
fn the_placement_through_lineage(
    left: Lineage,
    right: Lineage,
    key: &[u32],
    from: Side,
    both: bool,
) -> Result<Placement, OperatingError> {
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
    output_coverage(&join, &[CellKind::Node, CellKind::File], &inputs).map(placement)
}

/// Where an anti-join of a step (left, lineage `left`) and a per-file step missing the rejected file 1 (right) keyed on `key`
/// places what the right lacks: the left rows a missing right row would keep out.
fn the_placement_of_an_antijoin(left: Lineage, key: &[u32]) -> Result<Placement, OperatingError> {
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
    output_coverage(&antijoin, &kinds, &inputs).map(placement)
}

/// The lineage of node, file and number rows that come from one file each: a relation's, or a per-file step's.
fn one_file() -> Lineage {
    ryo_values::domain::lineage::service::one_file(&[
        CellKind::Node,
        CellKind::File,
        CellKind::Number,
    ])
}

fn subject(ask: Input) -> Result<Value, Failure> {
    match ask {
        Ask::NodeKey(operand) => the_placement_of_a_node_key(operand),
        Ask::Join(left, right, key, from, both) => {
            the_placement_through_lineage(lineage(left), lineage(right), key, from, both)
        }
        Ask::Antijoin(key) => the_placement_of_an_antijoin(Lineage::Mixed, key),
    }
}

/// A join of two per-file steps on node and file, answering the columns of this side, with the left missing file 1 too if true.
const fn per_file(from: Side, both: bool) -> Ask {
    Ask::Join(Lines::OneFile, Lines::OneFile, &[0, 1], from, both)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_key_of_a_relation_places_the_missing_rows_in_the_missing_file"),
        word: Word::Succeeds,
        input: || Ask::NodeKey(Reads::Relation),
        expect: Expect::Value(Placement::MissingFile(1)),
    },
    Case {
        name: CaseName("a_node_key_of_a_step_cannot_place_the_missing_rows"),
        word: Word::Succeeds,
        input: || Ask::NodeKey(Reads::Step),
        expect: Expect::Value(Placement::Anywhere),
    },
    Case {
        name: CaseName(
            "a_cross_file_step_joined_to_a_per_file_step_by_its_file_withholds_only_the_rejected_file",
        ),
        word: Word::Succeeds,
        input: || per_file(Side::Left, true),
        expect: Expect::Value(Placement::MissingFile(1)),
    },
    Case {
        name: CaseName(
            "without_the_cross_file_steps_lineage_its_missing_rows_place_nothing_in_the_other_operands_columns",
        ),
        word: Word::Succeeds,
        input: || Ask::Join(Lines::Mixed, Lines::OneFile, &[0, 1], Side::Right, true),
        expect: Expect::Value(Placement::Anywhere),
    },
    Case {
        name: CaseName(
            "the_per_file_steps_file_and_node_keys_carry_its_missing_file_into_the_cross_file_steps_columns",
        ),
        word: Word::Succeeds,
        input: || Ask::Join(Lines::Mixed, Lines::OneFile, &[0, 1], Side::Left, true),
        expect: Expect::Value(Placement::MissingFile(1)),
    },
    Case {
        name: CaseName(
            "a_cross_file_step_joined_to_a_handed_over_relation_withholds_only_the_rejected_file",
        ),
        word: Word::Succeeds,
        input: || per_file(Side::Left, true),
        expect: Expect::Value(Placement::MissingFile(1)),
    },
    Case {
        name: CaseName("a_handed_over_relation_alone_missing_withholds_only_the_rejected_file"),
        word: Word::Succeeds,
        input: || per_file(Side::Left, false),
        expect: Expect::Value(Placement::MissingFile(1)),
    },
    Case {
        name: CaseName("the_handed_over_relations_columns_withhold_only_the_rejected_file"),
        word: Word::Succeeds,
        input: || per_file(Side::Right, true),
        expect: Expect::Value(Placement::MissingFile(1)),
    },
    Case {
        name: CaseName(
            "the_handed_over_relations_columns_alone_missing_withhold_only_the_rejected_file",
        ),
        word: Word::Succeeds,
        input: || per_file(Side::Right, false),
        expect: Expect::Value(Placement::MissingFile(1)),
    },
    Case {
        name: CaseName("a_lineage_not_keyed_by_file_cannot_place_the_missing_rows"),
        word: Word::Succeeds,
        input: || Ask::Join(Lines::OneFile, Lines::OneFile, &[2], Side::Left, false),
        expect: Expect::Value(Placement::Anywhere),
    },
    Case {
        name: CaseName(
            "an_antijoin_keyed_by_the_missing_steps_file_withholds_only_the_rejected_file",
        ),
        word: Word::Succeeds,
        input: || Ask::Antijoin(&[0, 1]),
        expect: Expect::Value(Placement::MissingFile(1)),
    },
    Case {
        name: CaseName("an_antijoin_keyed_by_the_file_alone_cannot_place_the_left_node_column"),
        word: Word::Succeeds,
        input: || Ask::Antijoin(&[1]),
        expect: Expect::Value(Placement::Anywhere),
    },
    Case {
        name: CaseName("an_antijoin_keyed_by_a_number_cannot_place_the_missing_rows"),
        word: Word::Succeeds,
        input: || Ask::Antijoin(&[2]),
        expect: Expect::Value(Placement::Anywhere),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
