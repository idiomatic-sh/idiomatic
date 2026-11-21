//! The cases of a step that is one read of a relation with its pins: each reader that takes its branches reads the
//! relation in place (a scan, a pin's bucket, or its own key), and no table copies the relation. A step with a test that
//! is no pin, a union of reads and a fill's rows stay definitions. Each scene is lowered and said in lines as the cases
//! above it are. Every expected text is what a run printed.

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
        name: CaseName(
            "two_rules_that_emit_a_relations_rows_each_read_the_relation_and_no_table_is_made",
        ),
        word: Word::Succeeds,
        input: || (&["one tree rule", "one tree rule"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowNode\": 2, \"Emit\": 2, \"End\": 3, \"ReadNode\": 6}",
        ))),
    },
    Case {
        name: CaseName(
            "a_relations_rows_held_to_a_constant_are_read_in_place_as_the_constants_bucket",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the edges of the field label",
                    "the edges of the field label",
                ],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachMatch:File Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachMatch:File Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 2, \"Emit\": 2, \"End\": 3, \"ReadNode\": 4}",
        ))),
    },
    Case {
        name: CaseName("a_relations_rows_two_steps_read_are_read_in_place_by_each"),
        word: Word::Succeeds,
        input: || (&["the elements two projections read"], SampleStage::First),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant EachMatch:Fanout Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 1, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 2}",
        ))),
    },
    Case {
        name: CaseName("a_reader_in_a_pipeline_probes_the_relation_by_its_own_key"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the elements two projections read",
                    "the elements two projections read",
                ],
                SampleStage::First,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 3 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 0\n",
            "table 0: Rows, 3 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 2, \"Emit\": 2, \"End\": 3, \"ReadMade\": 6, \"StageProbe\": 1, \"StageRead\": 2, \"StageScan\": 1, \"StageWrite\": 1}",
        ))),
    },
    Case {
        name: CaseName("a_pin_of_a_view_read_by_a_readers_own_key_is_a_test_of_the_row_found"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the words, beside their edges and their spans",
                    "the words, beside their edges and their spans",
                ],
                SampleStage::First,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 3 columns, emptiness All([All([All([Unknown, Unknown]), Unknown]), Any([All([Row(SyntaxTree, Some(RowValue { column: SyntaxTree(Kind), value: CellWord(1) })), Row(SyntaxTree, Some(RowValue { column: SyntaxTree(Kind), value: CellWord(1) }))])])])\n",
            "  pattern bound [] collected []: Table 0\n",
            "table 0: Rows, 3 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Probe:Fanout Read:Constant Probe:Fanout Read:Constant Keep:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 2, \"Emit\": 2, \"End\": 3, \"ReadMade\": 6, \"StageKeep\": 1, \"StageProbe\": 2, \"StageRead\": 4, \"StageScan\": 1, \"StageWrite\": 1}",
        ))),
    },
    Case {
        name: CaseName("a_union_of_two_reads_of_a_relation_stays_a_definition"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the elements of two kinds, by a union",
                    "the elements of two kinds, by a union",
                ],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Any([Unknown, Unknown]), Any([All([Row(SyntaxTree, Some(RowValue { column: SyntaxTree(Kind), value: CellWord(1) }))]), All([Row(SyntaxTree, Some(RowValue { column: SyntaxTree(Kind), value: CellWord(2) }))])])])\n",
            "  pattern bound [] collected []: Table 0\n",
            "table 0: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant\n",
            "  pipeline: Scan:File Read:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 2, \"Emit\": 2, \"End\": 3, \"ReadMade\": 8, \"StageRead\": 4, \"StageScan\": 2, \"StageWrite\": 2}",
        ))),
    },
    Case {
        name: CaseName("a_fill_reads_a_views_definition_and_every_other_reader_the_relation"),
        word: Word::Succeeds,
        input: || {
            (
                &["the elements beside the count of their kind"],
                SampleStage::First,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 3 columns, emptiness Unknown\n",
            "  pattern bound [] collected []: Table 1\n",
            "table 0: Group(Count), 3 columns, indexes [[0]]\n",
            "  pipeline: Scan:File Read:Constant Read:Constant\n",
            "table 1: Rows, 3 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant EachMatch:Fanout Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchMade\": 1, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadMade\": 1, \"ReadNode\": 1, \"StageRead\": 3, \"StageScan\": 2, \"StageWrite\": 2}",
        ))),
    },
    Case {
        name: CaseName("a_kind_set_over_a_view_reads_the_relations_buckets"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the sentences and words of a projection, and its comments",
                    "the sentences and words of a projection, and its comments",
                ],
                SampleStage::First,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 3 columns, emptiness All([Any([Any([Unknown, Unknown]), Unknown]), Any([All([Row(SyntaxTree, Some(RowValue { column: SyntaxTree(Kind), value: CellWord(1) }))]), All([Row(SyntaxTree, Some(RowValue { column: SyntaxTree(Kind), value: CellWord(2) }))]), All([Row(SyntaxTree, Some(RowValue { column: SyntaxTree(Kind), value: CellWord(3) }))])])])\n",
            "  pattern bound [] collected []: Table 0\n",
            "table 0: Rows, 3 columns, indexes []\n",
            "  pipeline: Scan:File\n",
            "  pipeline: Scan:File\n",
            "  pipeline: Scan:File\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 2, \"Emit\": 2, \"End\": 3, \"ReadMade\": 4, \"StageScan\": 3, \"StageWrite\": 3}",
        ))),
    },
    Case {
        name: CaseName("a_selection_of_a_view_has_the_emptiness_of_the_relations_own_column"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the edges of a projection in two fields",
                    "the edges of a projection in two fields",
                ],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 4 columns, emptiness All([Any([Row(SyntaxChild, Some(RowValue { column: SyntaxChild(Field), value: CellWord(1) })), Row(SyntaxChild, Some(RowValue { column: SyntaxChild(Field), value: CellWord(2) }))]), Any([All([Row(SyntaxChild, Some(RowValue { column: SyntaxChild(Field), value: CellWord(1) }))]), All([Row(SyntaxChild, Some(RowValue { column: SyntaxChild(Field), value: CellWord(2) }))])])])\n",
            "  pattern bound [] collected []: Table 0\n",
            "table 0: Rows, 4 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant\n",
            "  pipeline: Scan:File Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 2, \"Emit\": 2, \"End\": 3, \"ReadMade\": 6, \"StageRead\": 2, \"StageScan\": 2, \"StageWrite\": 2}",
        ))),
    },
    Case {
        name: CaseName(
            "two_columns_of_one_row_held_equal_stay_equal_when_the_step_is_one_read_of_a_definition",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &["the words that are their own parent, beside their spans"],
                SampleStage::First,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 0\n",
            "  pattern bound [1] collected []: Table 0\n",
            "table 0: Rows, 5 columns, indexes [[1]]\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Test:Constant Exists:Fanout Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 1, \"Emit\": 1, \"End\": 2, \"Exists\": 1, \"ReadMade\": 3, \"StageProbe\": 1, \"StageRead\": 3, \"StageScan\": 1, \"StageWrite\": 1, \"Test\": 1}",
        ))),
    },
    Case {
        name: CaseName(
            "an_absence_asked_of_a_selection_that_holds_two_columns_equal_asks_the_selection",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &["the words no word that is its own parent shares a text with"],
                SampleStage::First,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 1\n",
            "definition 1: 5 columns, emptiness All([Definition(DefinitionId(0)), Any([All([Definition(DefinitionId(0))])])])\n",
            "  pattern bound [3] collected []: Table 0\n",
            "table 0: Rows, 5 columns, indexes [[3]]\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Keep:Constant Read:Constant Read:Constant\n",
            "table 1: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Exists:Fanout Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 1, \"Emit\": 1, \"End\": 2, \"Exists\": 1, \"ReadMade\": 4, \"StageKeep\": 1, \"StageProbe\": 1, \"StageRead\": 7, \"StageScan\": 2, \"StageWrite\": 2}",
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
