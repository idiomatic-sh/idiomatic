//! The shapes the fifth step adds, each lowered from a scene and said in lines as the cases above it are: a step only
//! projections read is defined on the columns they keep. Every expected text is what a run printed.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::placement::types::SampleStage;
use std::borrow::Cow;

/// The scenes whose plans a row lowers together, one rule each, and what the load was given of a sample.
type Input = super::super::Input;
/// The pack in lines, then its codes by count, then each refused rule: one text, a line each.
type Value = super::super::Value;
type Failure = super::super::Failure;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_step_only_projections_read_is_defined_on_the_columns_they_keep"),
        word: Word::Succeeds,
        input: || (&["the elements two projections read"], SampleStage::First),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 3 columns, emptiness Unknown\n",
            "  pattern bound [] collected []: Table 0\n",
            "  pattern bound [0] collected [2]: Table 0\n",
            "table 0: Rows, 3 columns, indexes [[0]]\n",
            "  pipeline: Scan:File Read:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Or:Fanout Emit:Constant\n",
            "codes: {\"EachRowMade\": 1, \"Emit\": 1, \"End\": 2, \"Or\": 1, \"ReadMade\": 2, \"StageRead\": 2, \"StageScan\": 1, \"StageWrite\": 1}",
        ))),
    },
    Case {
        name: CaseName("a_step_one_reader_reads_whole_is_defined_on_every_column"),
        word: Word::Succeeds,
        input: || {
            (
                &["the elements a projection and a whole read"],
                SampleStage::First,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness Unknown\n",
            "  pattern bound [] collected []: Table 0\n",
            "  pattern bound [0] collected [4]: Table 0\n",
            "table 0: Rows, 5 columns, indexes [[0]]\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Or:Fanout Emit:Constant\n",
            "codes: {\"EachRowMade\": 1, \"Emit\": 1, \"End\": 2, \"Or\": 1, \"ReadMade\": 2, \"StageRead\": 3, \"StageScan\": 1, \"StageWrite\": 1}",
        ))),
    },
    Case {
        name: CaseName("a_walk_only_a_projection_reads_is_defined_on_every_column"),
        word: Word::Succeeds,
        input: || {
            (
                &["the nearest elements above, projected"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness Unknown\n",
            "  pattern bound [] collected []: Table 1\n",
            "  pattern bound [0] collected []: Call: EachMatch:Fanout Yield:Constant\n",
            "definition 1: 6 columns, emptiness All([All([Definition(DefinitionId(0)), Unknown, Definition(DefinitionId(0))]), Any([All([Definition(DefinitionId(0))])])])\n",
            "  pattern bound [] collected []: Table 0\n",
            "  pattern bound [0] collected [5]: Call: Nearest:Fanout Exists:Fanout Let:Constant Yield:Constant\n",
            "table 0: Rows, 6 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant Read:Constant Use:Fanout\n",
            "table 1: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 1, \"EachRowMade\": 1, \"Emit\": 1, \"End\": 4, \"Exists\": 1, \"Let\": 1, \"Nearest\": 1, \"ReadMade\": 1, \"StageRead\": 7, \"StageScan\": 2, \"StageUse\": 1, \"StageWrite\": 2, \"Yield\": 2}",
        ))),
    },
    Case {
        name: CaseName("an_antijoin_against_a_relation_is_an_exists_over_its_rows_read_in_place"),
        word: Word::Succeeds,
        input: || (&["the words no link starts at"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant EachMatch:Fanout Exists:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchFilled\": 1, \"EachMatchNode\": 1, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 3, \"Exists\": 1, \"ReadNode\": 3, \"Yield\": 1}",
        ))),
    },
    Case {
        name: CaseName("an_antijoin_against_a_relation_is_in_a_pipeline_a_probe_asked_absent"),
        word: Word::Succeeds,
        input: || {
            (
                &["the words no link starts at", "the words no link starts at"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 0\n",
            "table 0: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 2, \"Emit\": 2, \"End\": 3, \"ReadMade\": 8, \"StageProbe\": 2, \"StageRead\": 3, \"StageScan\": 1, \"StageWrite\": 1}",
        ))),
    },
    Case {
        name: CaseName("a_nearest_walk_whose_domain_is_a_relation_with_a_filter_reads_it_in_place"),
        word: Word::Succeeds,
        input: || {
            (
                &["the nearest sentence above each word"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 1\n",
            "  pattern bound [1] collected []: Call: EachMatch:Fanout EachMatch:Fanout Yield:Constant\n",
            "definition 1: 6 columns, emptiness All([All([Definition(DefinitionId(0)), Unknown, Unknown]), Any([All([Definition(DefinitionId(0))])])])\n",
            "  pattern bound [] collected []: Table 0\n",
            "  pattern bound [1] collected [5]: Call: Nearest:Fanout Exists:Fanout Let:Constant Yield:Constant\n",
            "table 0: Rows, 6 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant Read:Constant Use:Fanout\n",
            "table 1: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 3, \"EachRowMade\": 1, \"Emit\": 1, \"End\": 5, \"Exists\": 1, \"Let\": 1, \"Nearest\": 1, \"ReadMade\": 5, \"ReadNode\": 1, \"StageProbe\": 1, \"StageRead\": 7, \"StageScan\": 2, \"StageUse\": 1, \"StageWrite\": 2, \"Test\": 1, \"Yield\": 3}",
        ))),
    },
    Case {
        name: CaseName("a_test_of_the_file_waits_in_a_pipeline_for_a_row_that_holds_the_file"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the words counted by text, in stub files",
                    "the words counted by text, in stub files",
                ],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 2\n",
            "definition 1: 4 columns, emptiness All([All([Definition(DefinitionId(0)), Definition(DefinitionId(0))]), Any([All([Definition(DefinitionId(0))])])])\n",
            "  pattern bound [] collected []: Table 1\n",
            "table 0: Group(Count), 2 columns, indexes [[0]]\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant Read:Constant\n",
            "table 1: Rows, 4 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Keep:Constant Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "table 2: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 2, \"Emit\": 2, \"End\": 3, \"ReadMade\": 6, \"StageKeep\": 1, \"StageProbe\": 2, \"StageRead\": 11, \"StageScan\": 3, \"StageWrite\": 3}",
        ))),
    },
    Case {
        name: CaseName(
            "a_pipeline_takes_the_order_keyed_throughout_before_a_cheaper_one_with_a_product",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the elements whose replaced text an import names",
                    "the elements whose replaced text an import names",
                ],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 3 columns, emptiness All([All([Unknown, Row(StringLiteral, None)]), Any([All([Row(StringLiteral, None)])])])\n",
            "  pattern bound [] collected []: Table 0\n",
            "table 0: Rows, 3 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Call:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 2, \"Emit\": 2, \"End\": 3, \"ReadMade\": 4, \"StageCall\": 1, \"StageProbe\": 1, \"StageRead\": 2, \"StageScan\": 1, \"StageWrite\": 1}",
        ))),
    },
    Case {
        name: CaseName("a_next_sibling_axis_is_a_walk_to_the_sibling_after"),
        word: Word::Succeeds,
        input: || (&["the word after each word"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Axis:Fanout EachMatch:Fanout Read:Constant Read:Constant EachMatch:Fanout Read:Constant Emit:Constant\n",
            "codes: {\"Axis\": 1, \"EachMatchNode\": 2, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 4}",
        ))),
    },
    Case {
        name: CaseName("an_ancestor_axis_over_edges_that_are_no_tree_is_a_closure_from_the_start"),
        word: Word::Succeeds,
        input: || (&["the scopes above each scope"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 4 columns, emptiness All([Row(Scope, None), Any([All([Row(Scope, None)])])])\n",
            "  pattern bound [] collected []: Table 1\n",
            "  pattern bound [1] collected []: Call: EachMatch:Fanout Yield:Constant\n",
            "definition 1: 5 columns, emptiness All([All([Definition(DefinitionId(0)), Unknown, Definition(DefinitionId(0))]), Any([All([Definition(DefinitionId(0)), Definition(DefinitionId(0))])])])\n",
            "  pattern bound [] collected []: Table 0\n",
            "  pattern bound [1] collected [4]: Call: Exists:Fanout Closure:Fanout Exists:Fanout Let:Constant Yield:Constant\n",
            "table 0: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant Use:Fanout Use:Fanout\n",
            "table 1: Rows, 4 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"Closure\": 1, \"EachMatchFilled\": 2, \"EachRowMade\": 1, \"Emit\": 1, \"End\": 5, \"Exists\": 2, \"Let\": 1, \"ReadFilled\": 1, \"ReadMade\": 4, \"StageRead\": 6, \"StageScan\": 2, \"StageUse\": 2, \"StageWrite\": 2, \"Yield\": 3}",
        ))),
    },
    Case {
        name: CaseName("a_group_count_over_a_relation_scans_the_relations_own_rows"),
        word: Word::Succeeds,
        input: || (&["the links counted by kind"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "table 0: Group(Count), 3 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 1, \"Emit\": 1, \"End\": 2, \"ReadMade\": 2, \"StageRead\": 4, \"StageScan\": 1, \"StageWrite\": 1}",
        ))),
    },
    Case {
        name: CaseName(
            "a_join_keyed_on_both_sides_by_a_calls_cell_reads_its_right_sides_table_by_the_key",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &["the elements that start where a bracket ends"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 7 columns, emptiness Unknown\n",
            "  pattern bound [6] collected [0]: Table 0\n",
            "table 0: Rows, 7 columns, indexes [[6]]\n",
            "  pipeline: Scan:File Read:Constant Call:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Call:Constant Or:Fanout Emit:Constant\n",
            "codes: {\"Call\": 1, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"Or\": 1, \"ReadNode\": 1, \"StageCall\": 1, \"StageRead\": 2, \"StageScan\": 1, \"StageWrite\": 1}",
        ))),
    },
];

/// The scenes lowered and said in lines, as the cases of the service itself are.
fn subject(input: Input) -> Result<Value, Failure> {
    super::super::subject(input)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
