//! The `lineage` service's cases: which steps keep their rows in one file, and which of their columns lie in it.
use super::service::{anchor_of, naming_of, of_step, one_file, pairing};
use super::types::{ColumnAnchor, FileNaming, KeyPairing, Lineage};
use ryo_relations::domain::relations::types::CellKind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::plan_parts::types::{
    ColumnIndex, Input as PlanInput, JoinColumn, Side, StepId,
};
use crate::domain::plans::types::Operation;

/// What a row of the cases proves.
#[derive(Clone, Copy)]
enum Scenario {
    /// A relation's file and node columns lie in its rows' file, and a selection keeps the ones it keeps.
    ASelectionKeepsTheColumnsThatLieInTheRowsFile,
    /// A join keyed by a column of each operand that lies in its row's file keeps its rows in one file.
    AJoinKeyedByFileKeepsItsRowsInOneFile,
    /// A join on a byte offset may combine the rows of two files.
    AJoinOnAByteOffsetMayCombineTwoFiles,
    /// A span is a place in its row's file, so a join keyed by spans pairs rows of one file; but it names no file, so a count
    /// keyed by spans, or the distinct rows of spans alone, may merge equal spans of two files.
    AJoinKeyedBySpansPairsRowsOfOneFile,
    /// A join with a hosted relation holding no file, whose answer takes every file and node column from the other operand,
    /// keeps that operand's rows in their file; one taking a file or node column of each operand does not.
    AJoinTakingEveryFileColumnFromOneOperandKeepsItsFile,
    /// A count keyed by file keeps its keys; one keyed by no file may count two files' rows.
    ACountKeepsItsRowsInOneFileOnlyWhenKeyedByFile,
    /// The rows of a union lie in one file each; a column lies in it only when it does in every member.
    AUnionLiesInOneFileWhereEveryMemberDoes,
}

type Input = Scenario;
type Value = ();
type Failure = ();

fn columns(anchors: &[ColumnAnchor]) -> Lineage {
    Lineage::OneFile(anchors.into())
}

fn join(key: u32) -> Operation {
    Operation::Join {
        left: PlanInput::Step(StepId(0)),
        right: PlanInput::Step(StepId(1)),
        left_on: vec![ColumnIndex(key)],
        right_on: vec![ColumnIndex(key)],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(1),
            },
        ],
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    }
}

fn subject(input: Input) {
    use ColumnAnchor::{Elsewhere, InFile};
    let relation = one_file(&[CellKind::Node, CellKind::File, CellKind::Number]);
    match input {
        Scenario::ASelectionKeepsTheColumnsThatLieInTheRowsFile => {
            assert_eq!(relation, columns(&[InFile, InFile, Elsewhere]));
            let select = Operation::Select {
                from: PlanInput::Step(StepId(0)),
                comparisons: Vec::new(),
                kept: vec![ColumnIndex(2), ColumnIndex(1)],
            };
            assert_eq!(
                of_step(
                    &select,
                    std::slice::from_ref(&relation),
                    &[CellKind::Number, CellKind::File]
                ),
                columns(&[Elsewhere, InFile])
            );
        }
        Scenario::AJoinKeyedByFileKeepsItsRowsInOneFile => assert_eq!(
            of_step(
                &join(1),
                &[relation.clone(), relation],
                &[CellKind::Node, CellKind::File]
            ),
            columns(&[InFile, InFile]),
            "both rows come from the file the key names"
        ),
        Scenario::AJoinOnAByteOffsetMayCombineTwoFiles => assert_eq!(
            of_step(
                &join(2),
                &[relation.clone(), relation],
                &[CellKind::Node, CellKind::File]
            ),
            Lineage::Mixed
        ),
        Scenario::AJoinKeyedBySpansPairsRowsOfOneFile => {
            assert_eq!(anchor_of(CellKind::Span), InFile);
            assert_eq!(naming_of(CellKind::Span), FileNaming::Unnamed);
            let spans = one_file(&[CellKind::Span, CellKind::Number]);
            assert_eq!(
                pairing(&spans, &[ColumnIndex(0)], &spans, &[ColumnIndex(0)], &[]),
                KeyPairing::SameFile,
                "equal spans of two files do not meet"
            );
            assert_eq!(
                pairing(&spans, &[ColumnIndex(1)], &spans, &[ColumnIndex(1)], &[]),
                KeyPairing::Unproven,
                "equal numbers of two files meet"
            );
            assert_eq!(
                of_step(
                    &Operation::GroupCount {
                        from: PlanInput::Step(StepId(0)),
                        keys: vec![ColumnIndex(0)],
                        distinct: None,
                    },
                    std::slice::from_ref(&spans),
                    &[CellKind::Span, CellKind::Number]
                ),
                Lineage::Mixed,
                "a count merges equal spans of two files"
            );
            let distinct = |columns: Vec<ColumnIndex>| Operation::Project {
                from: PlanInput::Step(StepId(0)),
                columns,
                distinct: crate::domain::plan_parts::types::RepeatedRows::Dropped,
            };
            assert_eq!(
                of_step(
                    &distinct(vec![ColumnIndex(0)]),
                    std::slice::from_ref(&spans),
                    &[CellKind::Span]
                ),
                columns(&[Elsewhere]),
                "the distinct spans of two files merge"
            );
            let located = one_file(&[CellKind::Span, CellKind::File]);
            assert_eq!(
                of_step(
                    &distinct(vec![ColumnIndex(0), ColumnIndex(1)]),
                    std::slice::from_ref(&located),
                    &[CellKind::Span, CellKind::File]
                ),
                columns(&[InFile, InFile]),
                "a file kept beside the span keeps the files apart"
            );
        }
        Scenario::AJoinTakingEveryFileColumnFromOneOperandKeepsItsFile => {
            let configuration = of_step(
                &Operation::Hosted {
                    relation: crate::domain::plan_parts::types::HostedNumber(0),
                },
                &[],
                &[CellKind::Symbol],
            );
            assert_eq!(configuration, Lineage::Mixed);
            let on_code = |output: Vec<JoinColumn>| Operation::Join {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Step(StepId(1)),
                left_on: vec![ColumnIndex(2)],
                right_on: vec![ColumnIndex(0)],
                output,
                filter: Vec::new(),
                view: Vec::new(),
                reach: None,
            };
            let column = |side, at| JoinColumn {
                side,
                column: ColumnIndex(at),
            };
            assert_eq!(
                of_step(
                    &on_code(vec![
                        column(Side::Left, 0),
                        column(Side::Left, 1),
                        column(Side::Right, 0),
                    ]),
                    &[relation.clone(), configuration.clone()],
                    &[CellKind::Node, CellKind::File, CellKind::Symbol]
                ),
                columns(&[InFile, InFile, Elsewhere]),
                "every file and node column comes from the left row"
            );
            assert_eq!(
                of_step(
                    &on_code(vec![column(Side::Left, 0), column(Side::Right, 1)]),
                    &[relation.clone(), relation.clone()],
                    &[CellKind::Node, CellKind::File]
                ),
                Lineage::Mixed,
                "a node of the left row beside a file of the right row may name two files"
            );
            assert_eq!(
                of_step(
                    &on_code(vec![column(Side::Left, 0)]),
                    &[Lineage::Mixed, configuration],
                    &[CellKind::Node]
                ),
                Lineage::Mixed,
                "an operand of mixed rows keeps no column in one file"
            );
        }
        Scenario::ACountKeepsItsRowsInOneFileOnlyWhenKeyedByFile => {
            let count = |key: u32| Operation::GroupCount {
                from: PlanInput::Step(StepId(0)),
                keys: vec![ColumnIndex(key)],
                distinct: None,
            };
            assert_eq!(
                of_step(
                    &count(1),
                    std::slice::from_ref(&relation),
                    &[CellKind::File, CellKind::Number]
                ),
                columns(&[InFile, Elsewhere])
            );
            assert_eq!(
                of_step(
                    &count(2),
                    std::slice::from_ref(&relation),
                    &[CellKind::Number, CellKind::Number]
                ),
                Lineage::Mixed
            );
        }
        Scenario::AUnionLiesInOneFileWhereEveryMemberDoes => {
            let union = Operation::Union {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Step(StepId(1)),
            };
            let kinds = [CellKind::Node, CellKind::File, CellKind::Number];
            assert_eq!(
                of_step(
                    &union,
                    &[relation.clone(), columns(&[InFile, Elsewhere, Elsewhere])],
                    &kinds
                ),
                columns(&[InFile, Elsewhere, Elsewhere])
            );
            assert_eq!(
                of_step(&union, &[relation, Lineage::Mixed], &kinds),
                Lineage::Mixed
            );
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_selection_keeps_the_columns_that_lie_in_the_rows_file"),
        word: Word::Succeeds,
        input: || Scenario::ASelectionKeepsTheColumnsThatLieInTheRowsFile,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_join_keyed_by_file_keeps_its_rows_in_one_file"),
        word: Word::Succeeds,
        input: || Scenario::AJoinKeyedByFileKeepsItsRowsInOneFile,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_join_on_a_byte_offset_may_combine_two_files"),
        word: Word::Succeeds,
        input: || Scenario::AJoinOnAByteOffsetMayCombineTwoFiles,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_join_keyed_by_spans_pairs_rows_of_one_file"),
        word: Word::Succeeds,
        input: || Scenario::AJoinKeyedBySpansPairsRowsOfOneFile,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_join_taking_every_file_column_from_one_operand_keeps_its_file"),
        word: Word::Succeeds,
        input: || Scenario::AJoinTakingEveryFileColumnFromOneOperandKeepsItsFile,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_count_keeps_its_rows_in_one_file_only_when_keyed_by_file"),
        word: Word::Succeeds,
        input: || Scenario::ACountKeepsItsRowsInOneFileOnlyWhenKeyedByFile,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_union_lies_in_one_file_where_every_member_does"),
        word: Word::Succeeds,
        input: || Scenario::AUnionLiesInOneFileWhereEveryMemberDoes,
        expect: Expect::Value(()),
    },
];

/// The lineage of a hosted step whose output columns are of the kinds a row names: a hosted row comes from the file its one file
/// column names; with two file columns, or none, no column says which file it comes from.
fn hosted(kinds: &[CellKind]) -> Lineage {
    let hosted = Operation::Hosted {
        relation: crate::domain::plan_parts::types::HostedNumber(0),
    };
    of_step(&hosted, &[Lineage::Mixed], kinds)
}

/// The hosted rows answer a lineage, which is not a constant, so their table is made when the test runs.
fn hosted_cases() -> [Case<&'static [CellKind], Lineage, Failure>; 3] {
    use ColumnAnchor::{Elsewhere, InFile};
    [
        Case {
            name: CaseName("a_hosted_row_lies_in_the_file_its_one_file_column_names"),
            word: Word::Succeeds,
            input: || &[CellKind::File, CellKind::Node, CellKind::Number],
            expect: Expect::Value(columns(&[InFile, Elsewhere, Elsewhere])),
        },
        Case {
            name: CaseName("a_hosted_row_with_two_file_columns_lies_in_no_one_file"),
            word: Word::Succeeds,
            input: || &[CellKind::File, CellKind::File],
            expect: Expect::Value(Lineage::Mixed),
        },
        Case {
            name: CaseName("a_hosted_row_with_no_file_column_lies_in_no_one_file"),
            word: Word::Succeeds,
            input: || &[CellKind::Number],
            expect: Expect::Value(Lineage::Mixed),
        },
    ]
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
    run(&hosted_cases(), Word::Succeeds, |kinds| Ok(hosted(kinds)));
}
