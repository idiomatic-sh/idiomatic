//! Seam D: a request's rules merged into one logical program, and the planned program each rule is served from.

use super::fixtures::{Request, rules_of};
use super::service::scopes::service::observation_reach;
use super::service::{assemble, consumers, logical, roots, rule};
use super::types::program_inputs::value::ProgramInputs;
use super::types::{RuleBindings, RuleInput, RuleObservations};
use crate::domain::coverage::types::RuleIndex;
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_operators::inbound::use_cases::plan::interface::PlanUseCase as _;
use ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::LiteralText;
use std::borrow::Cow;

type Input = Request;
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
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
            "nodes 1; rule 0: steps [Ok(PlanNodeId(0))] emit Err(Table(NoSuchColumn { index: 99 })); consumers [1]; inputs [[Relation(Function)]]; reach [Phase]; roots word []; roots other [];",
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
