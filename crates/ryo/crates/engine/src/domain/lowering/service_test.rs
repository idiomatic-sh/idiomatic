//! The `lowering` service's cases: each scene's plans lowered, and the pack said in lines (`fixtures::described`): the
//! definitions with their patterns, the tables with their stages, the rules with their steps, each with its cost class;
//! then the codes the straight encoding writes for the pack, by count: every pack here passes the encoder, the holder of
//! the invariant. Every expected text is what a run printed, read line by line and found right.

mod flags;
mod operations;
mod sets;
mod shapes;
mod views;

use super::errors::LoweringError;
use super::fixtures::described::{code_census, described};
use super::fixtures::{bound, digests_of, named};
use super::placement::types::SampleStage;
use super::service::{lowered, lowered_at};
use super::types::{LoadRefusal, PlanNode, SelectedRule, SelectedRules};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::encoding::service::encoded;
use crate::domain::lowered::types::LoweredPack;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::query_plans::types::NodeDigests;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// The scenes whose plans a row lowers together, one rule each, and what the load was given of a sample.
type Input = (&'static [&'static str], SampleStage);
/// The pack in lines, then its codes by count, then each refused rule: one text, a line each.
type Value = Cow<'static, str>;
type Failure = LoweringError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_rule_lowers_to_an_empty_pack"),
        word: Word::Succeeds,
        input: || (&[], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed("codes: {\"End\": 1}")),
    },
    Case {
        name: CaseName("a_rule_over_one_relation_is_one_branch_that_walks_it"),
        word: Word::Succeeds,
        input: || (&["one tree rule"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 3}",
        ))),
    },
    Case {
        name: CaseName("a_plan_two_rules_emit_is_one_definition_and_each_rule_walks_its_table"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    "the elements that are no comment",
                    "the elements that are no comment",
                ],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness Unknown\n",
            "  pattern bound [] collected []: Table 0\n",
            "table 0: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Keep:Constant Read:Constant Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 2, \"Emit\": 2, \"End\": 3, \"ReadMade\": 8, \"StageKeep\": 1, \"StageRead\": 3, \"StageScan\": 1, \"StageWrite\": 1}",
        ))),
    },
    Case {
        name: CaseName("an_antijoin_is_one_decision_of_its_right_sides_definition_by_its_key"),
        word: Word::Succeeds,
        input: || (&["unbound correlated sub-plan"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness Unknown\n",
            "  pattern bound [1] collected []: Call: EachMatch:Fanout Yield:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Exists:Fanout Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 1, \"EachRowNode\": 1, \"Emit\": 1, \"End\": 3, \"Exists\": 1, \"ReadNode\": 3, \"Yield\": 1}",
        ))),
    },
    Case {
        name: CaseName(
            "a_join_on_the_file_alone_is_one_table_with_one_product_stage_and_the_rule_walks_it",
        ),
        word: Word::Succeeds,
        input: || (&["a product within a file"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 1\n",
            "definition 1: 3 columns, emptiness All([Definition(DefinitionId(0)), Definition(DefinitionId(0))])\n",
            "  pattern bound [] collected []: Table 0\n",
            "table 0: Rows, 3 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Product:File Read:Constant Keep:Constant Read:Constant\n",
            "table 1: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 1, \"Emit\": 1, \"End\": 2, \"ReadMade\": 2, \"StageKeep\": 1, \"StageProbe\": 1, \"StageProduct\": 1, \"StageRead\": 6, \"StageScan\": 2, \"StageWrite\": 2}",
        ))),
    },
    Case {
        name: CaseName(
            "the_right_side_two_rules_antijoins_share_is_one_definition_each_decides_by_its_own_key",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &["the elements that are no word", "the edges to no word"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [1] collected []: Call: EachMatch:Fanout EachMatch:Fanout Yield:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Exists:Fanout Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "rule 1: 1 branches\n",
            "  branch: EachRow:File Exists:Fanout Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachMatchNode\": 2, \"EachRowNode\": 2, \"Emit\": 2, \"End\": 4, \"Exists\": 2, \"ReadNode\": 6, \"Yield\": 1}",
        ))),
    },
    Case {
        name: CaseName(
            "a_group_count_is_one_table_whoever_reads_it_and_a_key_pinned_to_a_constant_stays_a_key",
        ),
        word: Word::Succeeds,
        input: || (&["one table, two readers"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 1\n",
            "  pattern bound [3] collected [1]: Table 1\n",
            "table 0: Group(Count), 3 columns, indexes [[0]]\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant Read:Constant\n",
            "table 1: Rows, 5 columns, indexes [[3]]\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 2 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant EachMatch:Fanout Emit:Constant\n",
            "  branch: Or:Fanout Let:Constant EachMatch:Fanout Emit:Constant\n",
            "codes: {\"EachMatchMade\": 2, \"EachRowMade\": 1, \"Emit\": 2, \"End\": 3, \"Let\": 1, \"Or\": 1, \"ReadMade\": 2, \"StageProbe\": 1, \"StageRead\": 7, \"StageScan\": 2, \"StageWrite\": 2}",
        ))),
    },
    Case {
        name: CaseName("an_as_of_is_an_ordered_table_of_its_right_side_read_by_latest"),
        word: Word::Succeeds,
        input: || (&["as-of latest before"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 1\n",
            "table 0: Ordered(Sorted), 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant Read:Constant\n",
            "table 1: Rows, 5 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Latest:Fanout Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 1, \"Emit\": 1, \"End\": 2, \"Latest\": 1, \"ReadMade\": 4, \"StageProbe\": 1, \"StageRead\": 7, \"StageScan\": 2, \"StageWrite\": 2}",
        ))),
    },
    Case {
        name: CaseName("an_as_of_over_a_join_sorts_the_joins_own_table"),
        word: Word::Succeeds,
        input: || (&["as-of tied events"], SampleStage::First),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 1\n",
            "  pattern bound [3] collected [1]: Table 1\n",
            "definition 1: 3 columns, emptiness All([All([Definition(DefinitionId(0)), Definition(DefinitionId(0))]), Any([All([Definition(DefinitionId(0)), Definition(DefinitionId(0))])])])\n",
            "  pattern bound [] collected []: Table 2\n",
            "table 0: Ordered(Sorted), 3 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant\n",
            "table 1: Rows, 5 columns, indexes [[3]]\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "table 2: Rows, 3 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Use:Fanout\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Latest:Fanout Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 1, \"Emit\": 1, \"End\": 2, \"Latest\": 1, \"ReadMade\": 3, \"StageProbe\": 2, \"StageRead\": 8, \"StageScan\": 3, \"StageWrite\": 3}",
        ))),
    },
    Case {
        name: CaseName("a_self_join_on_a_key_is_a_use_by_that_key_and_its_comparison_a_test_after"),
        word: Word::Succeeds,
        input: || (&["pairs within one group"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 0\n",
            "  pattern bound [3] collected [2]: Table 0\n",
            "table 0: Rows, 5 columns, indexes [[3]]\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Or:Fanout Test:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 1, \"Emit\": 1, \"End\": 2, \"Or\": 1, \"ReadMade\": 3, \"StageProbe\": 1, \"StageRead\": 3, \"StageScan\": 1, \"StageWrite\": 1, \"Test\": 1}",
        ))),
    },
    Case {
        name: CaseName("a_join_on_the_file_alone_with_a_relation_is_a_product_over_that_relation"),
        word: Word::Succeeds,
        input: || (&["join with an empty side"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 2 columns, emptiness All([All([All([Unknown, Unknown]), Row(SyntaxError, None)]), Any([All([Row(SyntaxError, None)])])])\n",
            "  pattern bound [] collected []: Table 0\n",
            "table 0: Rows, 2 columns, indexes []\n",
            "  pipeline: Scan:File Probe:Fanout Product:File\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 1, \"Emit\": 1, \"End\": 2, \"ReadMade\": 1, \"StageProbe\": 1, \"StageProduct\": 1, \"StageScan\": 1, \"StageWrite\": 1}",
        ))),
    },
    Case {
        name: CaseName("a_keyed_join_of_two_buckets_is_a_use_and_a_decision_never_a_product"),
        word: Word::Succeeds,
        input: || (&["join empty by its keys"], SampleStage::First),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [3] collected [1]: Table 0\n",
            "  pattern bound [1, 3] collected []: Table 0\n",
            "table 0: Rows, 5 columns, indexes [[1, 3], [3]]\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: Or:Fanout Exists:Fanout Emit:Constant\n",
            "codes: {\"Emit\": 1, \"End\": 2, \"Exists\": 1, \"Or\": 1, \"StageProbe\": 1, \"StageRead\": 3, \"StageScan\": 1, \"StageWrite\": 1}",
        ))),
    },
    Case {
        name: CaseName(
            "a_rule_that_cannot_be_evaluated_is_refused_by_its_step_and_the_others_lower",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &["one tree rule", "over the host's rows"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 3}\n",
            "refused rule 1: step PlanNode(0) cannot be evaluated",
        ))),
    },
    Case {
        name: CaseName("a_group_bound_is_a_group_table_of_its_inputs_rows_and_the_rule_walks_it"),
        word: Word::Succeeds,
        input: || (&["group-max ties"], SampleStage::Absent),
        expect: Expect::Value(Cow::Borrowed(concat!(
            "definition 0: 5 columns, emptiness All([Unknown, Unknown])\n",
            "  pattern bound [] collected []: Table 2\n",
            "  pattern bound [3] collected [2]: Table 2\n",
            "definition 1: 4 columns, emptiness All([All([Definition(DefinitionId(0)), Definition(DefinitionId(0))]), Any([All([Definition(DefinitionId(0)), Definition(DefinitionId(0))])])])\n",
            "  pattern bound [] collected []: Table 1\n",
            "table 0: Group(Ends), 4 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Read:Constant\n",
            "table 1: Rows, 4 columns, indexes []\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Use:Fanout\n",
            "table 2: Rows, 5 columns, indexes [[3]]\n",
            "  pipeline: Scan:File Read:Constant Read:Constant Probe:Fanout Read:Constant\n",
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowMade\": 1, \"Emit\": 1, \"End\": 2, \"ReadMade\": 3, \"StageProbe\": 2, \"StageRead\": 9, \"StageScan\": 3, \"StageWrite\": 3}",
        ))),
    },
    Case {
        name: CaseName(
            "a_rule_another_rule_observes_loads_and_its_pack_holds_no_step_of_the_observed_output",
        ),
        word: Word::Succeeds,
        input: || {
            (
                &["a rule over the tree whose words another rule observes"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Value(Cow::Borrowed(concat!(
            "rule 0: 1 branches\n",
            "  branch: EachRow:File Read:Constant Read:Constant Read:Constant Emit:Constant\n",
            "codes: {\"EachRowNode\": 1, \"Emit\": 1, \"End\": 2, \"ReadNode\": 3}",
        ))),
    },
    Case {
        name: CaseName("a_rule_set_whose_every_rule_is_refused_answers_the_last_refusal"),
        word: Word::Refuses,
        input: || {
            (
                &["over the host's rows", "a rule that emits after the files"],
                SampleStage::Absent,
            )
        },
        expect: Expect::Error(LoweringError::Refused {
            rule: SelectedRuleOrdinal(1),
            refusal: LoadRefusal::Unlowerable(PlanNode(1)),
        }),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    subject_with(input, |_| Vec::new())
}

/// The scenes lowered and said in lines, with the lines `more` says of the pack after the pack's own.
fn subject_with(
    (scenes, stage): Input,
    more: fn(&LoweredPack) -> Vec<String>,
) -> Result<Value, Failure> {
    let plans: Vec<_> = scenes.iter().map(|scene| named(scene)).collect();
    let values: Vec<_> = scenes.iter().map(|scene| bound(scene)).collect();
    let digests: Vec<NodeDigests> = plans
        .iter()
        .zip(&values)
        .map(|(plan, values)| digests_of(plan, RuleBindings(values)))
        .collect();
    let rules: Vec<SelectedRule<'_>> = plans
        .iter()
        .zip(&digests)
        .zip(&values)
        .zip(0..)
        .map(|(((plan, digests), values), rule)| SelectedRule {
            rule: SelectedRuleOrdinal(rule),
            plan,
            bindings: RuleBindings(values),
            digests,
        })
        .collect();
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
        fields: Vec::new(),
        fixed: Vec::new(),
    };
    let lowered = match stage {
        SampleStage::Absent => lowered(SelectedRules(&rules), None, &kinds),
        SampleStage::First | SampleStage::Second => {
            lowered_at(SelectedRules(&rules), stage, None, &kinds)
        }
    };
    // Every rule refused: the row names the last refusal, which is there only when each before it is.
    if let Some(last) = lowered.refused.last()
        && lowered.refused.len() == rules.len()
    {
        return Err(LoweringError::Refused {
            rule: last.rule,
            refusal: last.refusal,
        });
    }
    let mut lines = described(&lowered.pack);
    lines.extend(more(&lowered.pack));
    // The pack is written by the straight encoding, the holder of the invariant: its codes, by count.
    lines.push(format!(
        "codes: {:?}",
        code_census(&encoded(&lowered.pack)?)
    ));
    lines.extend(
        lowered
            .refused
            .iter()
            .map(|refused| format!("refused rule {}: {}", refused.rule.0, refused.refusal)),
    );
    Ok(Cow::Owned(lines.join("\n")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
