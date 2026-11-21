//! The operations gate (b) adds, each lowered from a scene and said in lines as the cases above it are: a closure over a
//! step's edges and over a relation's own rows, the per-row calls, a group ordinal. Every expected text is what a run printed.

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
            "a_closure_is_a_definition_whose_table_uses_its_own_walk_and_a_program_walks_in_place",
        ),
        word: Word::Succeeds,
        input: || (&["closure of the sentence's edges"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 2 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 1\n",
            "  pattern bound [0] collected []: Call: EachMatch:Fanout EachMatch:Fanout Yield:Constant\n",
            "  pattern bound [0] collected [1]: Call: EachMatch:Fanout EachMatch:Fanout Let:Constant Yield:Constant\n",
            "definition 1: 2 columns, emptiness Definition(DefinitionId(0))\n",
            "  pattern bound [] collected []: Table 0\n",
            "  pattern bound [0] collected [1]: Call: Exists:Fanout Closure:Fanout Let:Constant Yield:Constant\n",
            "table 0: Rows, 2 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Use:Fanout\n",
            "table 1: Rows, 2 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Probe:Fanout\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"Closure\": 1, \"EachMatchNode\": 4, \"EachRowMade\": 1, \"Emit\": 1, \"End\": 5, \"Exists\": 1, \"Let\": 2, \"ReadMade\": 2, \"StageProbe\": 1, \"StageRead\": 2, \"StageScan\": 2, \"StageUse\": 1, \"StageWrite\": 2, \"Yield\": 3}",
        ))),
    },
    Case {
        name: CaseName("a_replace_is_one_call_after_the_cells_it_reads_are_given"),
        word: Word::Succeeds,
        input: || (&["the words' texts replaced"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant EachMatch:Fanout Read:Constant Call:Constant Emit:Constant\n",
            "codes: {\"Call\": 1, \"EachMatchNode\": 1, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 3}",
        ))),
    },
    Case {
        name: CaseName("span_bounds_is_one_call_that_gives_two_cells"),
        word: Word::Succeeds,
        input: || (&["the elements' span bounds"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Call:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"Call\": 1, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 3}",
        ))),
    },
    Case {
        name: CaseName(
            "a_group_ordinal_is_a_group_table_of_its_inputs_distinct_rows_with_their_places",
        ),
        word: Word::Succeeds,
        input: || (&["group-ordinal from zero"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 1\n",
            "table 0: Group(Places), 6 columns, indexes [[5]]\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant Read:Constant\n",
            "table 1: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachMatch:File Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchMade\": 1, \"Emit\": 1, \"End\": 2, \"ReadMade\": 4, \"StageProbe\": 1, \"StageRead\": 7, \"StageScan\": 2, \"StageWrite\": 2}",
        ))),
    },
    Case {
        name: CaseName("a_closure_of_a_relations_own_rows_reads_them_in_place"),
        word: Word::Succeeds,
        input: || (&["closure of the scopes"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 3 columns, emptiness Row(Scope, None)\n",
            "  pattern bound [] collected []: Table 0\n",
            "  pattern bound [1] collected [2]: Call: EachMatch:Fanout Closure:Fanout Let:Constant Yield:Constant\n",
            "table 0: Rows, 3 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Use:Fanout\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"Closure\": 1, \"EachMatchFilled\": 2, \"EachRowMade\": 1, \"Emit\": 1, \"End\": 4, \"Let\": 1, \"ReadFilled\": 1, \"ReadMade\": 2, \"StageRead\": 1, \"StageScan\": 1, \"StageUse\": 1, \"StageWrite\": 1, \"Yield\": 2}",
        ))),
    },
    Case {
        name: CaseName("span_between_is_one_call_of_two_cells_that_gives_one"),
        word: Word::Succeeds,
        input: || (&["the elements' spans between"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Call:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"Call\": 1, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 3}",
        ))),
    },
    Case {
        name: CaseName("a_function_in_a_comparison_is_a_call_whose_cell_is_tested"),
        word: Word::Succeeds,
        input: || {
            (
                &["the words whose prefixed text is x.a"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Call:Constant Test:Constant EachMatch:Fanout Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"Call\": 1, \"EachMatchNode\": 1, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 3, \"Test\": 1}",
        ))),
    },
    Case {
        name: CaseName(
            "a_nearest_walk_over_the_tree_is_a_definition_whose_table_uses_its_own_walk",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &["the nearest element above, over the tree"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness Unknown\n",
            "  pattern bound [] collected []: Table 1\n",
            "  pattern bound [0] collected []: Call: EachMatch:Fanout Yield:Constant\n",
            "definition 1: 6 columns, emptiness All([Definition(DefinitionId(0)), Unknown, Definition(DefinitionId(0))])\n",
            "  pattern bound [] collected []: Table 0\n",
            "  pattern bound [0] collected [5]: Call: Nearest:Fanout Exists:Fanout Let:Constant Yield:Constant\n",
            "table 0: Rows, 6 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant Read:Constant Use:Fanout\n",
            "table 1: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 1, \"EachRowMade\": 1, \"Emit\": 1, \"End\": 4, \"Exists\": 1, \"Let\": 1, \"Nearest\": 1, \"ReadMade\": 5, \"StageRead\": 7, \"StageScan\": 2, \"StageUse\": 1, \"StageWrite\": 2, \"Yield\": 2}",
        ))),
    },
    Case {
        name: CaseName("a_nearest_walk_over_a_steps_edges_asks_their_definition_for_the_parent"),
        word: Word::Succeeds,
        input: || {
            (
                &["the nearest element above, over a step's edges"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness Unknown\n",
            "  pattern bound [] collected []: Table 1\n",
            "  pattern bound [0] collected []: Call: EachMatch:Fanout Yield:Constant\n",
            "definition 1: 5 columns, emptiness Unknown\n",
            "  pattern bound [1] collected [0]: Call: EachMatch:Fanout Read:Constant Let:Constant Yield:Constant\n",
            "definition 2: 6 columns, emptiness All([Definition(DefinitionId(0)), Unknown, Definition(DefinitionId(0))])\n",
            "  pattern bound [] collected []: Table 0\n",
            "  pattern bound [0] collected [5]: Call: Nearest:Fanout Exists:Fanout Let:Constant Yield:Constant\n",
            "table 0: Rows, 6 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant Read:Constant Use:Fanout\n",
            "table 1: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 2, \"EachRowMade\": 1, \"Emit\": 1, \"End\": 5, \"Exists\": 1, \"Let\": 2, \"NearestDerived\": 1, \"ReadMade\": 5, \"ReadNode\": 1, \"StageRead\": 7, \"StageScan\": 2, \"StageUse\": 1, \"StageWrite\": 2, \"Yield\": 3}",
        ))),
    },
    Case {
        name: CaseName("a_join_names_its_right_columns_through_the_view"),
        word: Word::Succeeds,
        input: || (&["the words' spans through a view"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant EachMatch:Fanout EachMatch:Fanout Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 2, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 2}",
        ))),
    },
    Case {
        name: CaseName("an_axis_names_its_entity_through_the_domains_view"),
        word: Word::Succeeds,
        input: || (&["the parents through a view"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Axis:Constant EachMatch:Fanout Emit:Constant\n",
            "codes: {\"Axis\": 1, \"EachMatchNode\": 1, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 3}",
        ))),
    },
    Case {
        name: CaseName("a_reach_join_unfolded_is_a_join_that_reads_the_closure_appended_after_it"),
        word: Word::Succeeds,
        input: || {
            (
                &["the scopes above, a reach join unfolded"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 3 columns, emptiness Row(Scope, None)\n",
            "  pattern bound [1] collected [2]: Call: EachMatch:Fanout Closure:Fanout Let:Constant Yield:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Or:Fanout Emit:Constant\n",
            "codes: {\"Closure\": 1, \"EachMatchFilled\": 2, \"EachRowFilled\": 1, \"Emit\": 1, \"End\": 4, \"Let\": 1, \"Or\": 1, \"ReadFilled\": 2, \"Yield\": 2}",
        ))),
    },
    Case {
        name: CaseName("a_source_search_is_a_call_that_gives_a_row_for_each_occurrence"),
        word: Word::Succeeds,
        input: || {
            (
                &["the occurrences in the elements' source"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Call:Fanout Emit:Constant\n",
            "codes: {\"Call\": 1, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 3}",
        ))),
    },
    Case {
        name: CaseName("a_capture_is_a_call_that_gives_one_text_for_each_slot"),
        word: Word::Succeeds,
        input: || (&["the words' texts captured"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant EachMatch:Fanout Read:Constant Call:Fanout Emit:Constant\n",
            "codes: {\"Call\": 1, \"EachMatchNode\": 1, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 3}",
        ))),
    },
    Case {
        name: CaseName("an_ordered_match_is_a_table_of_pairs_read_by_place"),
        word: Word::Succeeds,
        input: || (&["the words matched in order"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 1\n",
            "table 0: Ordered(Match), 10 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant Read:Constant\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant Read:Constant\n",
            "table 1: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 1, \"Emit\": 1, \"End\": 2, \"ReadMade\": 2, \"StageProbe\": 1, \"StageRead\": 11, \"StageScan\": 3, \"StageWrite\": 3}",
        ))),
    },
    Case {
        name: CaseName("a_selection_by_a_constant_field_reads_the_fields_bucket_of_the_node_table"),
        word: Word::Succeeds,
        input: || (&["the edges of the field label"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachMatch:File Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 2}",
        ))),
    },
    Case {
        name: CaseName(
            "a_selection_by_a_constant_field_is_empty_in_a_file_with_no_child_in_that_field",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the edges of the field label, past the first",
                    "the edges of the field label, past the first",
                ],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness Row(SyntaxChild, Some(RowValue { column: SyntaxChild(Field), value: CellWord(1) }))\n",
            "  pattern bound [] collected []: Table 0\n",
            "table 0: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Keep:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 2, \"Emit\": 2, \"End\": 3, \"ReadMade\": 8, \"StageKeep\": 1, \"StageRead\": 2, \"StageScan\": 1, \"StageWrite\": 1}",
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
