//! The cases of a flag column held to the word a flag is shown as, `true` or `false`: the word is the flag's own cell (one
//! raised, nought lowered), in the key of the read, in a test and in the selection's emptiness; never the word of a text.
//! Each scene is lowered and said in lines, then the constants its rows are read and kept by. Every expected text is
//! what a run printed.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::fixtures::described::constants_said;
use crate::domain::lowering::placement::types::SampleStage;
use std::borrow::Cow;

/// The scenes whose plans a row lowers together, one rule each, and what the load was given of a sample.
type Input = super::super::Input;
/// The pack in lines, the constants its rows are read and kept by, its codes by count: one text, a line each.
type Value = super::super::Value;
type Failure = super::super::Failure;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_flag_held_to_the_word_true_is_held_to_the_raised_flags_cell"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the integer literals that are zero",
                    "the integer literals that are zero",
                ],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 7 columns, emptiness All([Row(LiteralProperties, Some(RowValue { column: LiteralProperties(Zero), value: CellWord(1) })), Any([All([Row(LiteralProperties, None)])])])\n",
            "  pattern bound [] collected []: Table 0\n",
            "table 0: Rows, 7 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Keep:Constant Read:Constant Read:Constant Read:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "table 0: Scan MatchColumns([Filled(LiteralProperties(Zero))]) StepOperands([Word(CellWord(1))])\n",
            "table 0: Keep NotEqual Register(Register(23)) Word(CellWord(1))\n",
            "codes: {\"EachRowMade\": 2, \"Emit\": 2, \"End\": 3, \"ReadMade\": 12, \"StageKeep\": 1, \"StageRead\": 5, \"StageScan\": 1, \"StageWrite\": 1}",
        ))),
    },
    Case {
        name: CaseName("a_flag_held_to_the_word_false_is_held_to_the_lowered_flags_cell"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the literals that are not empty",
                    "the literals that are not empty",
                ],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 7 columns, emptiness All([Row(LiteralProperties, Some(RowValue { column: LiteralProperties(Empty), value: CellWord(0) })), Any([All([Row(LiteralProperties, None)])])])\n",
            "  pattern bound [] collected []: Table 0\n",
            "table 0: Rows, 7 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Keep:Constant Read:Constant Read:Constant Read:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "table 0: Scan MatchColumns([Filled(LiteralProperties(Empty))]) StepOperands([Word(CellWord(0))])\n",
            "table 0: Keep NotEqual Register(Register(23)) Word(CellWord(1))\n",
            "codes: {\"EachRowMade\": 2, \"Emit\": 2, \"End\": 3, \"ReadMade\": 12, \"StageKeep\": 1, \"StageRead\": 5, \"StageScan\": 1, \"StageWrite\": 1}",
        ))),
    },
    Case {
        name: CaseName("a_flag_that_is_not_the_word_true_is_kept_by_a_test_of_the_flags_cell"),
        word: Word::Succeeds,
        input: || {
            (
                &["the literals that are not a boolean"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Test:Constant Read:Constant Test:Constant Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 0: Test NotEqual Register(Register(2)) Word(CellWord(1))\n",
            "rule 0: Test NotEqual Register(Register(5)) Word(CellWord(1))\n",
            "codes: {\"EachRowFilled\": 1, \"Emit\": 1, \"End\": 2, \"ReadFilled\": 6, \"Test\": 2}",
        ))),
    },
    Case {
        name: CaseName("a_word_on_the_left_of_a_flag_is_the_flags_cell_too"),
        word: Word::Succeeds,
        input: || {
            (
                &["the literals that are zero, the word first"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachMatch:File Read:Constant Test:Constant Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 0: EachMatch MatchColumns([Filled(LiteralProperties(Zero))]) StepOperands([Word(CellWord(1))])\n",
            "rule 0: Test NotEqual Register(Register(2)) Word(CellWord(1))\n",
            "codes: {\"EachMatchFilled\": 1, \"Emit\": 1, \"End\": 2, \"ReadFilled\": 5, \"Test\": 1}",
        ))),
    },
    Case {
        name: CaseName("a_closed_sets_column_held_to_one_of_its_words_is_held_to_the_words_symbol"),
        word: Word::Succeeds,
        input: || (&["the definitions of a variable"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachMatch:File Read:Constant Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 0: EachMatch MatchColumns([Filled(Defines(Kind))]) StepOperands([Word(CellWord(4294901774))])\n",
            "codes: {\"EachMatchFilled\": 1, \"Emit\": 1, \"End\": 2, \"ReadFilled\": 5}",
        ))),
    },
    Case {
        name: CaseName(
            "a_closed_sets_column_through_a_projection_is_tested_against_the_words_symbol",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &["the kinds of the definitions that are no variable"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Test:Constant Read:Constant Emit:Constant\n",
            "rule 0: Test NotEqual Register(Register(2)) Word(CellWord(4294901774))\n",
            "codes: {\"EachRowFilled\": 1, \"Emit\": 1, \"End\": 2, \"ReadFilled\": 2, \"Test\": 1}",
        ))),
    },
    Case {
        name: CaseName("a_text_that_is_no_word_of_a_closed_set_equals_no_cell_of_its_column"),
        word: Word::Succeeds,
        input: || {
            (
                &["the definitions of a kind no definition has"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 0 branches\n",
            "codes: {\"End\": 1}",
        ))),
    },
];

/// The scenes lowered and said in lines, with the constants of the pack's reads and tests.
fn subject(input: Input) -> Result<Value, Failure> {
    super::super::subject_with(input, constants_said)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
