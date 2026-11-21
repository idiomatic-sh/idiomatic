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
            "definition 1: 6 columns, emptiness All([Definition(DefinitionId(0)), Unknown, Definition(DefinitionId(0))])\n",
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
            "definition 1: 6 columns, emptiness All([Definition(DefinitionId(0)), Unknown, Unknown])\n",
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
];

/// The scenes lowered and said in lines, as the cases of the service itself are.
fn subject(input: Input) -> Result<Value, Failure> {
    super::super::subject(input)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
