//! Seam D: a request's rules merged into one logical program, and the planned program each rule is served from.

use super::fixtures::{Request, rules_of};
use super::service::scopes::service::observation_reach;
use super::service::{assemble, consumers, logical, roots, rule};
use super::types::program_inputs::value::ProgramInputs;
use super::types::{RuleInput, RuleObservations};
use crate::domain::coverage::types::RuleIndex;
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_operators::inbound::use_cases::plan::interface::PlanUseCase as _;
use ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::query_bindings::types::RuleBindings;
use std::borrow::Cow;

type Input = Request;
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("source_view_keeps_order_bindings_refusals_roots_and_original_steps"),
        word: Word::Succeeds,
        input: || Request::SourceViews,
        expect: Expect::Value(Cow::Borrowed(
            "same [true, true, false, false, false, false, false, false]; steps [5, 6, 6, 6, 5, 6, 5, 6]; shared frontier true; distinct risky roots true; invalid projection Err(Table(NoSuchColumn { index: 99 }))",
        )),
    },
    Case {
        name: CaseName("equal_steps_over_equal_inputs_are_one_node"),
        word: Word::Succeeds,
        input: || Request::EqualSteps,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 1; rule 0: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)); rule 1: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)); consumers [2]; inputs [[Relation(Function)]]; reach [Phase]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("same_step_in_file_and_project_scope_stays_apart"),
        word: Word::Succeeds,
        input: || Request::FileAndProjectScope,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 2; rule 0: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)); rule 1: steps [Ok(PlanNodeId(1))] emit Ok(PlanNodeId(1)); consumers [1, 1]; inputs [[Relation(Function)], [Relation(Function)]]; reach [Phase, Phase]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("equal_steps_with_different_parameter_values_stay_apart"),
        word: Word::Succeeds,
        input: || Request::DifferentParameterValues,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 2; rule 0: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)); rule 1: steps [Ok(PlanNodeId(1))] emit Ok(PlanNodeId(1)); consumers [1, 1]; inputs [[Relation(Function)], [Relation(Function)]]; reach [Phase, Phase]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("a_bound_input_reads_its_rule_s_place"),
        word: Word::Succeeds,
        input: || Request::BoundInput,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 1; rule 0: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)); consumers [1]; inputs [[Observation(BoundObservationPlace { rule: RuleOrdinal(0), input: ObservationBindingId(0) })]]; reach [Occurrence]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("a_schema_fault_is_the_emit_before_any_row"),
        word: Word::Succeeds,
        input: || Request::SchemaFault,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 1; rule 0: steps [Err(Table(NoSuchColumn { index: 99 }))] emit Err(Table(NoSuchColumn { index: 99 })); consumers [0]; inputs [[Relation(Function)]]; reach [Phase]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("root_kinds_stop_at_the_infallible_prefix"),
        word: Word::Succeeds,
        input: || Request::RootKinds,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 2; rule 0: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)); rule 1: steps [Ok(PlanNodeId(1))] emit Ok(PlanNodeId(1)); consumers [1, 1]; inputs [[Relation(SyntaxTree)], [Relation(SyntaxTree)]]; reach [Phase, Phase]; roots word [PlanNodeId(0)]; roots other [];",
        )),
    },
    Case {
        name: CaseName("two_rules_frontier_entries_stay_apart"),
        word: Word::Succeeds,
        input: || Request::TwoRulesFrontiers,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 3; rule 0: steps [Ok(PlanNodeId(0)), Ok(PlanNodeId(1))] emit Ok(PlanNodeId(1)); rule 1: steps [Ok(PlanNodeId(0)), Ok(PlanNodeId(2))] emit Ok(PlanNodeId(2)); consumers [2, 1, 1]; inputs [[Relation(Function)], [Frontier(FrontierPlace { rule: RuleOrdinal(0), input: InputNumber(0) })], [Frontier(FrontierPlace { rule: RuleOrdinal(1), input: InputNumber(0) })]]; reach [Phase, Phase, Phase]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("observation_inputs_of_two_rules_stay_apart"),
        word: Word::Succeeds,
        input: || Request::TwoRulesObservations,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 2; rule 0: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)); rule 1: steps [Ok(PlanNodeId(1))] emit Ok(PlanNodeId(1)); consumers [1, 1]; inputs [[Observation(BoundObservationPlace { rule: RuleOrdinal(0), input: ObservationBindingId(0) })], [Observation(BoundObservationPlace { rule: RuleOrdinal(1), input: ObservationBindingId(0) })]]; reach [Occurrence, Occurrence]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("a_node_reading_an_observation_is_occurrence_bound"),
        word: Word::Succeeds,
        input: || Request::ObservationReach,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 4; rule 0: steps [Ok(PlanNodeId(0)), Ok(PlanNodeId(1)), Ok(PlanNodeId(2)), Ok(PlanNodeId(3))] emit Ok(PlanNodeId(3)); consumers [1, 1, 0, 1]; inputs [[Relation(Function)], [Observation(BoundObservationPlace { rule: RuleOrdinal(0), input: ObservationBindingId(0) })], [Node(PlanNodeId(1))], [Frontier(FrontierPlace { rule: RuleOrdinal(0), input: InputNumber(1) })]]; reach [Phase, Occurrence, Occurrence, Phase]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("consumers_count_readers_emits_and_holds"),
        word: Word::Succeeds,
        input: || Request::SharedReads,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 2; rule 0: steps [Ok(PlanNodeId(0)), Ok(PlanNodeId(1))] emit Ok(PlanNodeId(1)); consumers [2, 1]; inputs [[Relation(Function)], [Node(PlanNodeId(0))]]; reach [Phase, Phase]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("a_fragment_two_rules_read_at_different_steps_is_one_node_each"),
        word: Word::Succeeds,
        input: || Request::SharedFragment,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 8; rule 0: steps [Ok(PlanNodeId(0)), Ok(PlanNodeId(1)), Ok(PlanNodeId(2)), Ok(PlanNodeId(3)), Ok(PlanNodeId(4))] emit Ok(PlanNodeId(4)); rule 1: steps [Ok(PlanNodeId(2)), Ok(PlanNodeId(3)), Ok(PlanNodeId(5)), Ok(PlanNodeId(6)), Ok(PlanNodeId(7))] emit Ok(PlanNodeId(7)); consumers [1, 1, 1, 2, 1, 1, 1, 1]; inputs [[Relation(Function)], [Node(PlanNodeId(0))], [Relation(Function)], [Node(PlanNodeId(2))], [Node(PlanNodeId(1)), Node(PlanNodeId(3))], [Relation(Function)], [Node(PlanNodeId(5))], [Node(PlanNodeId(6)), Node(PlanNodeId(3))]]; reach [Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("six_rules_reading_one_fragment_after_their_own_steps_share_its_nodes"),
        word: Word::Succeeds,
        input: || Request::SharedFragmentAfterOwnSteps,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 37; rule 0: steps [Ok(PlanNodeId(0)), Ok(PlanNodeId(1)), Ok(PlanNodeId(2)), Ok(PlanNodeId(3)), Ok(PlanNodeId(4)), Ok(PlanNodeId(5)), Ok(PlanNodeId(6))] emit Ok(PlanNodeId(6)); rule 1: steps [Ok(PlanNodeId(7)), Ok(PlanNodeId(8)), Ok(PlanNodeId(9)), Ok(PlanNodeId(2)), Ok(PlanNodeId(3)), Ok(PlanNodeId(4)), Ok(PlanNodeId(5)), Ok(PlanNodeId(10))] emit Ok(PlanNodeId(10)); rule 2: steps [Ok(PlanNodeId(11)), Ok(PlanNodeId(12)), Ok(PlanNodeId(13)), Ok(PlanNodeId(14)), Ok(PlanNodeId(2)), Ok(PlanNodeId(3)), Ok(PlanNodeId(4)), Ok(PlanNodeId(5)), Ok(PlanNodeId(15))] emit Ok(PlanNodeId(15)); rule 3: steps [Ok(PlanNodeId(16)), Ok(PlanNodeId(17)), Ok(PlanNodeId(18)), Ok(PlanNodeId(19)), Ok(PlanNodeId(20)), Ok(PlanNodeId(2)), Ok(PlanNodeId(3)), Ok(PlanNodeId(4)), Ok(PlanNodeId(5)), Ok(PlanNodeId(21))] emit Ok(PlanNodeId(21)); rule 4: steps [Ok(PlanNodeId(22)), Ok(PlanNodeId(23)), Ok(PlanNodeId(24)), Ok(PlanNodeId(25)), Ok(PlanNodeId(26)), Ok(PlanNodeId(27)), Ok(PlanNodeId(2)), Ok(PlanNodeId(3)), Ok(PlanNodeId(4)), Ok(PlanNodeId(5)), Ok(PlanNodeId(28))] emit Ok(PlanNodeId(28)); rule 5: steps [Ok(PlanNodeId(29)), Ok(PlanNodeId(30)), Ok(PlanNodeId(31)), Ok(PlanNodeId(32)), Ok(PlanNodeId(33)), Ok(PlanNodeId(34)), Ok(PlanNodeId(35)), Ok(PlanNodeId(2)), Ok(PlanNodeId(3)), Ok(PlanNodeId(4)), Ok(PlanNodeId(5)), Ok(PlanNodeId(36))] emit Ok(PlanNodeId(36)); consumers [1, 1, 1, 2, 1, 6, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]; inputs [[Relation(Function)], [Node(PlanNodeId(0))], [Relation(Function)], [Node(PlanNodeId(2))], [Node(PlanNodeId(3))], [Node(PlanNodeId(3)), Node(PlanNodeId(4))], [Node(PlanNodeId(1)), Node(PlanNodeId(5))], [Relation(Function)], [Node(PlanNodeId(7))], [Node(PlanNodeId(8))], [Node(PlanNodeId(9)), Node(PlanNodeId(5))], [Relation(Function)], [Node(PlanNodeId(11))], [Node(PlanNodeId(12))], [Node(PlanNodeId(13))], [Node(PlanNodeId(14)), Node(PlanNodeId(5))], [Relation(Function)], [Node(PlanNodeId(16))], [Node(PlanNodeId(17))], [Node(PlanNodeId(18))], [Node(PlanNodeId(19))], [Node(PlanNodeId(20)), Node(PlanNodeId(5))], [Relation(Function)], [Node(PlanNodeId(22))], [Node(PlanNodeId(23))], [Node(PlanNodeId(24))], [Node(PlanNodeId(25))], [Node(PlanNodeId(26))], [Node(PlanNodeId(27)), Node(PlanNodeId(5))], [Relation(Function)], [Node(PlanNodeId(29))], [Node(PlanNodeId(30))], [Node(PlanNodeId(31))], [Node(PlanNodeId(32))], [Node(PlanNodeId(33))], [Node(PlanNodeId(34))], [Node(PlanNodeId(35)), Node(PlanNodeId(5))]]; reach [Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase, Phase]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("steps_reading_the_same_nodes_in_another_order_or_count_stay_apart"),
        word: Word::Succeeds,
        input: || Request::InputOrder,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 6; rule 0: steps [Ok(PlanNodeId(0)), Ok(PlanNodeId(1)), Ok(PlanNodeId(2))] emit Ok(PlanNodeId(2)); rule 1: steps [Ok(PlanNodeId(0)), Ok(PlanNodeId(1)), Ok(PlanNodeId(3))] emit Ok(PlanNodeId(3)); rule 2: steps [Ok(PlanNodeId(0)), Ok(PlanNodeId(1)), Ok(PlanNodeId(4))] emit Ok(PlanNodeId(4)); rule 3: steps [Ok(PlanNodeId(0)), Ok(PlanNodeId(1)), Ok(PlanNodeId(5))] emit Ok(PlanNodeId(5)); consumers [5, 3, 1, 1, 1, 1]; inputs [[Relation(Function)], [Relation(Function)], [Node(PlanNodeId(0)), Node(PlanNodeId(1))], [Node(PlanNodeId(1)), Node(PlanNodeId(0))], [Node(PlanNodeId(0)), Node(PlanNodeId(1))], [Node(PlanNodeId(0)), Node(PlanNodeId(0))]]; reach [Phase, Phase, Phase, Phase, Phase, Phase]; roots word []; roots other [];",
        )),
    },
];

/// What the planned program says of each rule and node: the nodes, each rule's step nodes and emit, whether it is served, the
/// consumers and the root kinds of the two kind texts the cases write.
fn subject(request: Input) -> Result<Value, Failure> {
    let written = rules_of(request);
    let rules: Vec<RuleInput<'_>> = written
        .iter()
        .map(|(plan, bindings)| RuleInput {
            plan,
            bindings: RuleBindings(bindings),
            observations: RuleObservations(&plan.observation_inputs),
        })
        .collect();
    let mut inputs = ProgramInputs::default();
    let build = logical(&rules, &mut inputs).map_err(|_| ())?;
    if matches!(request, Request::SourceViews) {
        let nodes = &build.rules;
        let same: Vec<_> = nodes.iter().map(|r| r.emit == nodes[0].emit).collect();
        let steps: Vec<_> = nodes.iter().map(|r| r.steps.len()).collect();
        let frontier = nodes[0].steps[3] == nodes[1].steps[4];
        let risky = nodes[4].emit != nodes[5].emit;
        let invalid = &nodes[7].steps[0];
        return Ok(Cow::Owned(format!(
            "same {same:?}; steps {steps:?}; shared frontier {frontier}; distinct risky roots {risky}; invalid projection {invalid:?}"
        )));
    }
    let planned = PlanUseCaseImpl.plan(build.program()).map_err(|_| ())?;
    let program = assemble(build, planned);
    let nodes = inputs.lists.len();
    let mut said = format!("nodes {nodes};");
    for at in 0..rules.len() {
        let nodes = rule(&program, RuleIndex(at)).map_err(|_| ())?;
        said.push_str(&format!(
            " rule {at}: steps {:?} emit {:?};",
            nodes.steps, nodes.emit
        ));
    }
    let counts: Vec<u32> = (0..nodes)
        .map(|node| {
            consumers(&program, PlanNodeId(u32::try_from(node).unwrap_or(0))).map(|count| count.0)
        })
        .collect::<Result<_, _>>()
        .map_err(|_| ())?;
    said.push_str(&format!(" consumers {counts:?};"));
    said.push_str(&format!(" inputs {:?};", inputs.lists));
    let reach: Vec<_> = (0..nodes)
        .map(|node| observation_reach(&program, PlanNodeId(u32::try_from(node).unwrap_or(0))))
        .collect::<Result<_, _>>()
        .map_err(|_| ())?;
    said.push_str(&format!(" reach {reach:?};"));
    for kind in ["word", "other"] {
        said.push_str(&format!(
            " roots {kind} {:?};",
            roots(&program, &LiteralText(kind.into()))
        ));
    }
    Ok(Cow::Owned(said))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
