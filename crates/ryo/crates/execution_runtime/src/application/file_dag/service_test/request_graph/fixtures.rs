//! The support of the `request_graph` cases.
use ryo_execution_domain::domain::file_program::service::compile_prepared;
use ryo_execution_domain::domain::file_program::types::{NodeInput, RequestGraph, SchemaValidity};
use ryo_execution_domain::domain::observation_schedule::fixtures::plan;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::types::DemandOrder;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::Input as InputSource;
use ryo_values::domain::plan_parts::types::*;
use ryo_values::domain::plans::types::*;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_bindings::types::{ParameterType, PlanParameter};
use std::sync::Arc;

/// How the request varies the base plan.
#[derive(Clone, Copy, Debug)]
pub(super) enum Requested {
    /// Two rules whose select differs only in its comparison's recorded origin.
    Origins,
    /// Two rules of one bound select, bound to different texts.
    Bindings,
    /// Two rules whose first step is duplicated as their second.
    Duplicates,
    /// Two rules replacing the same text in the same rows.
    DerivedText,
    /// Two rules whose one step is the same request source.
    RequestSource,
    /// A union of two selects, a join of a select to a relation, and a group count of a select.
    Dependencies,
    /// A projection of a column the select does not have.
    InvalidContractShape,
}

/// What the request graph answered.
#[derive(Debug, PartialEq)]
pub(super) enum Graphed {
    /// Whether the two rules' compared steps are one physical producer.
    Shared(bool),
    /// Whether rule 0's second step, and rule 1's first and second, are rule 0's first producer.
    Duplicated([bool; 3]),
    /// Whether the union's two selects are distinct producers; and for the union, the join and the
    /// group count, the demand order of each and whether its inputs are its authored edges in
    /// their exact positions.
    Dependencies {
        distinct_union: bool,
        orders: [DemandOrder; 3],
        authored: [bool; 3],
    },
    /// The invalid projection's schema validity.
    Validity(SchemaValidity),
}

/// The request graph of `plans` over the operators the application prepares for them.
pub(super) fn graph(
    plans: &[Plan],
    bindings: &[Vec<ParameterBinding>],
) -> Result<RequestGraph, OperatingError> {
    let operators = crate::application::file_dag::service::prepare_with_schemas(
        plans,
        bindings,
        &[],
        &ryo_relations::domain::relations::service::kinds,
        Arc::new(ryo_execution_store::outbound::derived::factory::semantics()),
    )?;
    Ok(compile_prepared(plans, &operators))
}

/// Compiles the varied plans into one request graph and reads the producers it shares.
pub(super) fn exercise(requested: Requested) -> Result<Graphed, OperatingError> {
    let first = plan();
    match requested {
        Requested::Origins => {
            let mut second = first.clone();
            if let Operation::Select { comparisons, .. } = &mut second.steps[0].operation {
                comparisons[0].origin = ComparisonOrigin::Recorded(ComparisonOriginId(7));
            }
            let graph = graph(&[first, second], &[vec![], vec![]])?;
            Ok(Graphed::Shared(
                graph.program_nodes[0][0].as_ref().ok() == graph.program_nodes[1][0].as_ref().ok(),
            ))
        }
        Requested::Bindings => {
            let mut bound = first;
            bound.parameters.push(PlanParameter {
                name: ParameterName("name".into()),
                kind: ParameterType::Text,
            });
            if let Operation::Select { comparisons, .. } = &mut bound.steps[0].operation {
                comparisons[0].right = Operand::Parameter(ParameterId(0));
            }
            let graph = graph(
                &[bound.clone(), bound],
                &[
                    vec![ParameterBinding::Text(LiteralText("main".into()))],
                    vec![ParameterBinding::Text(LiteralText("other".into()))],
                ],
            )?;
            Ok(Graphed::Shared(
                graph.program_nodes[0][0].as_ref().ok() == graph.program_nodes[1][0].as_ref().ok(),
            ))
        }
        Requested::Duplicates => {
            let mut duplicate = first;
            duplicate.steps.insert(1, duplicate.steps[0].clone());
            duplicate.emit.step = StepId(2);
            duplicate.frontier = vec![InputSource::Step(StepId(1))];
            let graph = graph(&[duplicate.clone(), duplicate], &[vec![], vec![]])?;
            let node = graph.program_nodes[0][0].as_ref().ok();
            let same =
                |rule: usize, step: usize| graph.program_nodes[rule][step].as_ref().ok() == node;
            Ok(Graphed::Duplicated([same(0, 1), same(1, 0), same(1, 1)]))
        }
        Requested::DerivedText => {
            // Derived text belongs to the file being read, not to a rule: the same replacement over the
            // same rows is computed once for every rule that writes it.
            let mut replaced = first;
            replaced.steps[1].operation = Operation::Replace {
                from: InputSource::Step(StepId(0)),
                text: ColumnIndex(1),
                pattern: LiteralText("a".into()),
                replacement: LiteralText("b".into()),
            };
            let graph = graph(&[replaced.clone(), replaced], &[vec![], vec![]])?;
            Ok(Graphed::Shared(
                graph.program_nodes[0][1].as_ref().ok() == graph.program_nodes[1][1].as_ref().ok(),
            ))
        }
        Requested::RequestSource => {
            // A request source is what the request hands one rule: equal steps of two rules that bind nothing still
            // answer each rule's own rows, so neither reads the other's producer.
            let mut hosted = first;
            hosted.steps = vec![PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Hosted {
                    relation: HostedNumber(0),
                },
            }];
            hosted.emit.step = StepId(0);
            let graph = graph(&[hosted.clone(), hosted], &[vec![], vec![]])?;
            Ok(Graphed::Shared(
                graph.program_nodes[0][0].as_ref().ok() == graph.program_nodes[1][0].as_ref().ok(),
            ))
        }
        Requested::Dependencies => {
            let mut union = first.clone();
            let mut other = union.steps[0].clone();
            if let Operation::Select { comparisons, .. } = &mut other.operation {
                comparisons[0].right =
                    Operand::Literal(QueryLiteral::Text(LiteralText("other".into())));
            }
            union.steps = vec![
                union.steps[0].clone(),
                other,
                PlanStep {
                    scope: StepScope::PerFile,
                    operation: Operation::Union {
                        left: InputSource::Step(StepId(0)),
                        right: InputSource::Step(StepId(1)),
                    },
                },
            ];
            union.emit.step = StepId(2);
            let mut joined = first.clone();
            joined.steps[1].operation = Operation::Join {
                left: InputSource::Step(StepId(0)),
                right: InputSource::Relation(Relation::Function),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(1),
                }],
                filter: vec![],
                view: vec![],
                reach: None,
            };
            let mut grouped = first;
            grouped.steps[1].operation = Operation::GroupCount {
                from: InputSource::Step(StepId(0)),
                keys: vec![ColumnIndex(0)],
                distinct: None,
            };
            let graph = graph(&[union, joined, grouped], &[vec![], vec![], vec![]])?;
            let producer = |rule: usize, step: usize| {
                graph.program_nodes[rule][step]
                    .as_ref()
                    .map(|node| *node)
                    .map_err(Clone::clone)
            };
            let (union_left, union_right) = (producer(0, 0)?, producer(0, 1)?);
            let (join_left, group_input) = (producer(1, 0)?, producer(2, 0)?);
            let mut orders = [DemandOrder::Unary; 3];
            let mut authored = [false; 3];
            for (at, (rule, step, inputs)) in [
                (
                    0,
                    2,
                    vec![NodeInput::Node(union_left), NodeInput::Node(union_right)],
                ),
                (
                    1,
                    1,
                    vec![
                        NodeInput::Node(join_left),
                        NodeInput::Relation(Relation::Function),
                    ],
                ),
                (2, 1, vec![NodeInput::Node(group_input)]),
            ]
            .into_iter()
            .enumerate()
            {
                let node = &graph.nodes[producer(rule, step)?.0];
                orders[at] = node.contract.semantics.demand;
                authored[at] = node.inputs == inputs;
            }
            Ok(Graphed::Dependencies {
                distinct_union: union_left != union_right,
                orders,
                authored,
            })
        }
        Requested::InvalidContractShape => {
            let mut invalid = first;
            if let Operation::Project { columns, .. } = &mut invalid.steps[1].operation {
                columns[0] = ColumnIndex(999);
            }
            let graph = graph(&[invalid], &[vec![]])?;
            let id = graph.program_nodes[0][1].as_ref().map_err(Clone::clone)?;
            Ok(Graphed::Validity(graph.nodes[id.0].schema_valid))
        }
    }
}
