//! Seam D: a request's rules merged into one logical program, and the planned program each rule is served from.

use super::service::{assemble, consumers, logical, roots, rule, serving};
use super::types::program_inputs::value::ProgramInputs;
use super::types::{RuleBindings, RuleInput, RuleObservations};
use crate::domain::coverage::types::RuleIndex;
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_operators::inbound::use_cases::plan::interface::PlanUseCase as _;
use ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input as StepInput, LiteralText, ObservationBindingId, Operand, ParameterId,
    PlanComparison, QueryLiteral, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use std::borrow::Cow;
use std::sync::Arc;

/// A request of rules, each written as its steps and what its parameter is bound to.
#[derive(Clone, Copy, Debug)]
enum Request {
    /// Two rules with the same select of a relation, bound alike.
    EqualSteps,
    /// The same select, once on each file and once after the files.
    FileAndProjectScope,
    /// The same select comparing with a parameter, bound to two values.
    DifferentParameterValues,
    /// A rule that reads a bound input.
    BoundInput,
    /// A rule with a kind no lowering serves yet, beside a served rule.
    UnservedKind,
    /// A select of a column its relation does not have.
    SchemaFault,
    /// Two selects of the tree by kind, one behind a comparison that may fail.
    RootKinds,
    /// A select read by a project, both kept by the rule's frontier.
    SharedReads,
}

type Input = Request;
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_steps_over_equal_inputs_are_one_node"),
        word: Word::Succeeds,
        input: || Request::EqualSteps,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 1; rule 0: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)) Served; rule 1: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)) Served; consumers [2]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("same_step_in_file_and_project_scope_stays_apart"),
        word: Word::Succeeds,
        input: || Request::FileAndProjectScope,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 2; rule 0: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)) Served; rule 1: steps [Ok(PlanNodeId(1))] emit Ok(PlanNodeId(1)) NotServedUntil(ServedAt(4)); consumers [1, 1]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("equal_steps_with_different_parameter_values_stay_apart"),
        word: Word::Succeeds,
        input: || Request::DifferentParameterValues,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 2; rule 0: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)) Served; rule 1: steps [Ok(PlanNodeId(1))] emit Ok(PlanNodeId(1)) Served; consumers [1, 1]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("a_bound_input_is_not_served_until_the_fourth_stage"),
        word: Word::Succeeds,
        input: || Request::BoundInput,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 1; rule 0: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)) NotServedUntil(ServedAt(4)); consumers [1]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("an_unlowered_kind_is_not_served_until_its_stage_and_the_other_rule_is"),
        word: Word::Succeeds,
        input: || Request::UnservedKind,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 2; rule 0: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)) NotServedUntil(ServedAt(3)); rule 1: steps [Ok(PlanNodeId(1))] emit Ok(PlanNodeId(1)) Served; consumers [1, 1]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("a_schema_fault_is_the_emit_before_any_row"),
        word: Word::Succeeds,
        input: || Request::SchemaFault,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 1; rule 0: steps [Ok(PlanNodeId(0))] emit Err(Table(NoSuchColumn { index: 99 })) Served; consumers [1]; roots word []; roots other [];",
        )),
    },
    Case {
        name: CaseName("root_kinds_stop_at_the_infallible_prefix"),
        word: Word::Succeeds,
        input: || Request::RootKinds,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 2; rule 0: steps [Ok(PlanNodeId(0))] emit Ok(PlanNodeId(0)) Served; rule 1: steps [Ok(PlanNodeId(1))] emit Ok(PlanNodeId(1)) Served; consumers [1, 1]; roots word [PlanNodeId(0)]; roots other [];",
        )),
    },
    Case {
        name: CaseName("consumers_count_readers_emits_and_holds"),
        word: Word::Succeeds,
        input: || Request::SharedReads,
        expect: Expect::Value(Cow::Borrowed(
            "nodes 2; rule 0: steps [Ok(PlanNodeId(0)), Ok(PlanNodeId(1))] emit Ok(PlanNodeId(1)) Served; consumers [2, 1]; roots word []; roots other [];",
        )),
    },
];

/// A comparison of a column with an operand.
fn compared(test: Test, column: u32, right: Operand) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right,
        origin: ComparisonOrigin::Unrecorded,
    }
}

/// A written text.
fn text(written: &str) -> Operand {
    Operand::Literal(QueryLiteral::Text(LiteralText(written.into())))
}

/// A select of `from` by `comparisons`, keeping every column.
fn select(from: StepInput, comparisons: Vec<PlanComparison>) -> Operation {
    Operation::Select {
        from,
        comparisons,
        kept: Vec::new(),
    }
}

/// A plan of `steps` that emits its last step and hands `frontier` to the steps after the files.
fn plan(steps: Vec<PlanStep>, frontier: Vec<StepInput>) -> Arc<Plan> {
    Arc::new(Plan {
        observation_inputs: Vec::new(),
        parameters: Vec::new(),
        comparison_origins: Vec::new(),
        observations: Vec::new(),
        frontier,
        emit: Emit {
            step: StepId(u32::try_from(steps.len().saturating_sub(1)).unwrap_or(0)),
            subject: ColumnIndex(0),
            key: Vec::new(),
            headings: Vec::new(),
        },
        steps,
    })
}

/// A step on each file.
fn per_file(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}

/// The rules of `request`, each with what its parameter is bound to.
fn rules_of(request: Request) -> Vec<(Arc<Plan>, Vec<ParameterBinding>)> {
    let named = || select(StepInput::Relation(Relation::Function), vec![compared(Test::Equal, 1, text("a"))]);
    let unbound = Vec::new;
    match request {
        Request::EqualSteps => vec![
            (plan(vec![per_file(named())], Vec::new()), unbound()),
            (plan(vec![per_file(named())], Vec::new()), unbound()),
        ],
        Request::FileAndProjectScope => vec![
            (plan(vec![per_file(named())], Vec::new()), unbound()),
            (
                plan(
                    vec![PlanStep {
                        operation: named(),
                        scope: StepScope::CrossFile,
                    }],
                    Vec::new(),
                ),
                unbound(),
            ),
        ],
        Request::DifferentParameterValues => {
            let parameter = || {
                select(
                    StepInput::Relation(Relation::Function),
                    vec![compared(Test::Equal, 1, Operand::Parameter(ParameterId(0)))],
                )
            };
            let bound = |value: &str| vec![ParameterBinding::Text(LiteralText(value.into()))];
            vec![
                (plan(vec![per_file(parameter())], Vec::new()), bound("a")),
                (plan(vec![per_file(parameter())], Vec::new()), bound("c")),
            ]
        }
        Request::BoundInput => vec![(
            plan(
                vec![per_file(select(StepInput::BoundObservation(ObservationBindingId(0)), Vec::new()))],
                Vec::new(),
            ),
            unbound(),
        )],
        Request::UnservedKind => vec![
            (
                plan(
                    vec![per_file(Operation::Axis {
                        from: StepInput::Relation(Relation::SyntaxTree),
                        edges: StepInput::Relation(Relation::SyntaxChild),
                        domain: StepInput::Relation(Relation::SyntaxTree),
                        direction: ryo_values::domain::plan_parts::types::Direction::Parent,
                        id: ColumnIndex(0),
                        parent: ColumnIndex(0),
                        child: ColumnIndex(1),
                        domain_id: ColumnIndex(0),
                        domain_filter: Vec::new(),
                        domain_view: Vec::new(),
                        output: Vec::new(),
                    })],
                    Vec::new(),
                ),
                unbound(),
            ),
            (plan(vec![per_file(named())], Vec::new()), unbound()),
        ],
        Request::SchemaFault => vec![(
            plan(
                vec![per_file(select(
                    StepInput::Relation(Relation::Function),
                    vec![compared(Test::Equal, 99, text("a"))],
                ))],
                Vec::new(),
            ),
            unbound(),
        )],
        Request::RootKinds => vec![
            (
                plan(
                    vec![per_file(select(
                        StepInput::Relation(Relation::SyntaxTree),
                        vec![compared(Test::Equal, 1, text("word"))],
                    ))],
                    Vec::new(),
                ),
                unbound(),
            ),
            (
                plan(
                    vec![per_file(select(
                        StepInput::Relation(Relation::SyntaxTree),
                        vec![
                            compared(Test::Matches, 2, text("[")),
                            compared(Test::Equal, 1, text("other")),
                        ],
                    ))],
                    Vec::new(),
                ),
                unbound(),
            ),
        ],
        Request::SharedReads => vec![(
            plan(
                vec![
                    per_file(named()),
                    per_file(Operation::Project {
                        from: StepInput::Step(StepId(0)),
                        columns: vec![ColumnIndex(0)],
                        distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
                    }),
                ],
                vec![StepInput::Step(StepId(0))],
            ),
            unbound(),
        )],
    }
}

/// What the planned program says of each rule and node: the nodes, each rule's step nodes and emit, whether it is served, the
/// consumers and the root kinds of the two kind texts the cases write.
fn subject(request: Input) -> Result<Value, Failure> {
    let written = rules_of(request);
    let rules: Vec<RuleInput<'_>> = written
        .iter()
        .map(|(plan, bindings)| RuleInput {
            plan,
            bindings: RuleBindings(bindings),
            observations: RuleObservations(&[]),
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
            " rule {at}: steps {:?} emit {:?} {:?};",
            nodes.steps,
            nodes.emit,
            serving(&program, RuleIndex(at)).map_err(|_| ())?
        ));
    }
    let counts: Vec<u32> = (0..nodes)
        .map(|node| consumers(&program, PlanNodeId(u32::try_from(node).unwrap_or(0))).map(|count| count.0))
        .collect::<Result<_, _>>()
        .map_err(|_| ())?;
    said.push_str(&format!(" consumers {counts:?};"));
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
