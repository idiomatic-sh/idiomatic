//! The cases of a selection that holds a node's kind or field to a set of literals: each way a plan writes the set gives
//! the union of the literals' buckets, and a pattern with a wildcard, or a column that is no kind and no field, stays a
//! test. Each scene is lowered and said in lines as the cases above it are. Every expected text is what a run printed.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::placement::types::SampleStage;
use crate::domain::lowering::types::{LoadRefusal, PlanNode};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// The scenes whose plans a row lowers together, one rule each, and what the load was given of a sample.
type Input = super::super::Input;
/// The pack in lines, then its codes by count, then each refused rule: one text, a line each.
type Value = super::super::Value;
type Failure = super::super::Failure;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_kind_held_to_a_glob_of_literals_reads_each_literals_bucket"),
        word: Word::Succeeds,
        input: || {
            (
                &["the elements of two kinds, by a glob"],
                SampleStage::Second,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 2 branches\n",
            "  branch: EachMatch:File Read:Constant Read:Constant Emit:Constant\n",
            "  branch: EachMatch:File Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 2, \"Emit\": 2, \"End\": 3, \"ReadNode\": 4}",
        ))),
    },
    Case {
        name: CaseName("a_kind_held_to_a_set_is_in_a_table_one_pipeline_for_each_bucket"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the elements of two kinds, by a glob",
                    "the elements of two kinds, by a glob",
                ],
                SampleStage::Second,
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
        name: CaseName("a_union_of_equalities_reads_the_buckets_the_glob_reads"),
        word: Word::Succeeds,
        input: || {
            (
                &["the elements of two kinds, by a union"],
                SampleStage::Second,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 2 branches\n",
            "  branch: EachMatch:File Read:Constant Read:Constant Emit:Constant\n",
            "  branch: EachMatch:File Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 2, \"Emit\": 2, \"End\": 3, \"ReadNode\": 4}",
        ))),
    },
    Case {
        name: CaseName("an_expression_of_literal_words_reads_the_buckets_the_glob_reads"),
        word: Word::Succeeds,
        input: || {
            (
                &["the elements of two kinds, by an expression"],
                SampleStage::Second,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 2 branches\n",
            "  branch: EachMatch:File Read:Constant Read:Constant Emit:Constant\n",
            "  branch: EachMatch:File Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 2, \"Emit\": 2, \"End\": 3, \"ReadNode\": 4}",
        ))),
    },
    Case {
        name: CaseName("an_in_list_reads_the_buckets_the_glob_reads"),
        word: Word::Succeeds,
        input: || {
            (
                &["the elements of two kinds, by a list"],
                SampleStage::Second,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 2 branches\n",
            "  branch: EachMatch:File Read:Constant Read:Constant Emit:Constant\n",
            "  branch: EachMatch:File Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 2, \"Emit\": 2, \"End\": 3, \"ReadNode\": 4}",
        ))),
    },
    Case {
        name: CaseName(
            "a_field_held_to_a_set_reads_each_fields_bucket_and_needs_an_edge_in_one_of_them",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the edges in two fields, by a glob",
                    "the edges in two fields, by a glob",
                ],
                SampleStage::Second,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Any([Row(SyntaxChild, Some(RowValue { column: SyntaxChild(Field), value: CellWord(1) })), Row(SyntaxChild, Some(RowValue { column: SyntaxChild(Field), value: CellWord(2) }))]), Any([All([Row(SyntaxChild, Some(RowValue { column: SyntaxChild(Field), value: CellWord(1) }))]), All([Row(SyntaxChild, Some(RowValue { column: SyntaxChild(Field), value: CellWord(2) }))])])])\n",
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
        name: CaseName("a_glob_with_a_star_stays_a_test_of_the_kinds_text"),
        word: Word::Succeeds,
        input: || {
            (
                &["the elements of a kind by a glob with a star"],
                SampleStage::Second,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Test:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 3, \"Test\": 1}",
        ))),
    },
    Case {
        name: CaseName("a_text_held_to_a_set_stays_a_test_of_the_text"),
        word: Word::Succeeds,
        input: || {
            (
                &["the elements of two texts, by a glob"],
                SampleStage::Second,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Test:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 3, \"Test\": 1}",
        ))),
    },
    Case {
        name: CaseName(
            "a_set_in_the_filter_of_a_nearest_walks_domain_stays_a_test_of_its_one_read",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &["the nearest sentence or clause above each word"],
                SampleStage::Second,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 1\n",
            "  pattern bound [1] collected []: Call: EachMatch:Fanout Exists:Constant Yield:Constant\n",
            "definition 1: 6 columns, emptiness All([All([Definition(DefinitionId(0)), Unknown, Unknown]), Any([All([Definition(DefinitionId(0))])])])\n",
            "  pattern bound [] collected []: Table 0\n",
            "  pattern bound [1] collected [5]: Call: Nearest:Fanout Exists:Fanout Let:Constant Yield:Constant\n",
            "table 0: Rows, 6 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant Read:Constant Use:Fanout\n",
            "table 1: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 3, \"EachRowMade\": 1, \"Emit\": 1, \"End\": 6, \"Exists\": 2, \"Let\": 1, \"Nearest\": 1, \"ReadMade\": 5, \"ReadNode\": 1, \"StageProbe\": 1, \"StageRead\": 7, \"StageScan\": 2, \"StageUse\": 1, \"StageWrite\": 2, \"Test\": 1, \"Yield\": 4}",
        ))),
    },
    Case {
        name: CaseName("an_in_list_held_against_a_text_that_is_no_kind_and_no_field_is_refused"),
        word: Word::Refuses,
        input: || {
            (
                &["the elements of two texts, by a list"],
                SampleStage::Second,
            )
        },
        expect: Expect::Error(LoweringError::Refused {
            rule: SelectedRuleOrdinal(0),
            refusal: LoadRefusal::Unlowerable(PlanNode(0)),
        }),
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

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
