//! The parts of `logical`: the nodes and shapes a step's inputs read, and the nodes that start from the tree nodes of a kind.
//! it and the operators' physical plan.

use crate::domain::executable_program::types::{
    Consumers, Draft, DraftInputs, ObservationReach, RuleInput, RuleNodes, StepAt,
};
use crate::domain::kind_sets::service::names_a_kind;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::KindNaming;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::service::semantics;
use ryo_operators::domain::comparing::service::fixed_prefix;
use ryo_operators::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use ryo_operators::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_operators::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_operators::domain::tree_cells::service::native_form;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{
    Input, InputNumber, LiteralText, Operand, QueryLiteral, StepScope, Test,
};
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::plans::types::Operation;

/// The inputs `named` reads, given the nodes of the rule's earlier `steps`; none when one reads a step that has no node. A step
/// after the files reads each entry of its rule's frontier (a relation, a step in each file) at the entry's place, its rows
/// handed over to the rule, and a bound observation at its rule's place.
pub(in crate::domain::executable_program) fn read_inputs<'s>(
    named: &[Input],
    steps: &[Result<PlanNodeId, OperatingError>],
    drafts: &'s [Draft<'_>],
    observed: &'s [Vec<CellKind>],
    at: StepAt<'_>,
) -> Option<DraftInputs<'s>> {
    let mut read = DraftInputs {
        list: Vec::with_capacity(named.len()),
        shapes: Vec::with_capacity(named.len()),
        fault: None,
    };
    for each in named {
        let handed = (at.scope == StepScope::CrossFile)
            .then(|| at.plan.frontier.iter().position(|entry| entry == each))
            .flatten()
            .map(|place| PlanInput::Frontier(FrontierPlace::of(at.rule, InputNumber(place))));
        let (input, shape) = match each {
            Input::Relation(relation) => (
                handed.unwrap_or(PlanInput::Relation(*relation)),
                InputShape {
                    kinds: kinds(*relation),
                    tree: native_form(*relation),
                },
            ),
            Input::Step(id) => {
                let Some(Ok(node)) = steps
                    .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                    .cloned()
                else {
                    return None;
                };
                let draft = drafts.get(usize::try_from(node.0).unwrap_or(usize::MAX))?;
                if let (Err(error), None) = (&draft.schema, &read.fault) {
                    read.fault = Some(error.clone());
                }
                (
                    match handed {
                        Some(place) if draft.scope == StepScope::PerFile => place,
                        Some(_) | None => PlanInput::Node(node),
                    },
                    InputShape {
                        kinds: draft.schema.as_deref().unwrap_or_default(),
                        tree: None,
                    },
                )
            }
            Input::BoundObservation(id) => {
                // A binding fed in each file is handed over with the rule's frontier; one fed after the files is bound to it.
                let in_each_file = at
                    .plan
                    .observation_inputs
                    .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                    .is_some_and(|schema| schema.scope == StepScope::PerFile);
                (
                    match handed {
                        Some(place) if in_each_file => place,
                        Some(_) | None => {
                            PlanInput::Observation(BoundObservationPlace::of(at.rule, *id))
                        }
                    },
                    InputShape {
                        kinds: observed
                            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                            .map_or(&[], Vec::as_slice),
                        tree: None,
                    },
                )
            }
        };
        read.list.push(input);
        read.shapes.push(shape);
    }
    Some(read)
}

/// The nodes that start from the tree nodes of a kind: each select of a tree relation whose infallible prefix tests the kind
/// column equal to a written text, by that text. A kind test after a comparison that may fail is never a root kind, so its
/// fault is never skipped.
pub(in crate::domain::executable_program) fn derived_roots(
    drafts: &[Draft<'_>],
    lists: &[Vec<PlanInput>],
) -> hashbrown::HashMap<LiteralText, Vec<PlanNodeId>> {
    let mut roots = hashbrown::HashMap::<LiteralText, Vec<PlanNodeId>>::new();
    for (at, (draft, list)) in drafts.iter().zip(lists).enumerate() {
        let (Operation::Select { comparisons, .. }, [PlanInput::Relation(relation)]) =
            (draft.operation, list.as_slice())
        else {
            continue;
        };
        if native_form(*relation).is_none() || draft.schema.is_err() {
            continue;
        }
        let kinds: &[CellKind] = kinds(*relation);
        for comparison in fixed_prefix(comparisons, kinds).0 {
            if comparison.test == Test::Equal
                && names_a_kind(comparison) == KindNaming::Names
                && let Operand::Literal(QueryLiteral::Text(text)) = &comparison.right
                && let Ok(node) = u32::try_from(at)
            {
                roots
                    .entry(text.clone())
                    .or_default()
                    .push(PlanNodeId(node));
            }
        }
    }
    roots
}

/// How many nodes and rules read each node, its emitted, frontier and named-root nodes held in `program` for its rules.
///
/// # Errors
///
/// `ShapeMismatch` when the request has more rules than a plan can number.
pub(in crate::domain::executable_program) fn held(
    program: &mut LogicalProgram<'_>,
    rules: &[RuleInput<'_>],
    nodes: &[RuleNodes],
    lists: &[Vec<PlanInput>],
) -> Result<Vec<Consumers>, OperatingError> {
    let mut consumers = vec![Consumers(0); lists.len()];
    let count = |consumers: &mut Vec<Consumers>, node: PlanNodeId| {
        if let Some(each) = consumers.get_mut(usize::try_from(node.0).unwrap_or(usize::MAX)) {
            each.0 = each.0.saturating_add(1);
        }
    };
    for input in lists.iter().flatten() {
        if let PlanInput::Node(read) = input {
            count(&mut consumers, *read);
        }
    }
    for (rule, (input, rule_nodes)) in rules.iter().zip(nodes).enumerate() {
        let ordinal = RuleOrdinal(u32::try_from(rule).map_err(|_| OperatingError::ShapeMismatch)?);
        if let Some(Ok(node)) = rule_nodes
            .steps
            .get(usize::try_from(input.plan.emit.step.0).unwrap_or(usize::MAX))
        {
            program.emit(ordinal, *node);
            count(&mut consumers, *node);
        }
        let kept = input.plan.frontier.iter().chain(
            input
                .plan
                .observations
                .iter()
                .map(|observation| &observation.source),
        );
        for each in kept {
            if let Input::Step(id) = each
                && let Some(Ok(node)) = rule_nodes
                    .steps
                    .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            {
                program.hold(*node);
                count(&mut consumers, *node);
            }
        }
    }
    Ok(consumers)
}

/// Whether each node's rows answer the whole phase or one occurrence: a node that reads a bound observation, an antijoin or a
/// difference after the files (whose rows its occurrence's coverage narrows where it stands), or a node that reads one of
/// those, answers one occurrence only.
pub(in crate::domain::executable_program) fn observation_reaches(
    drafts: &[Draft<'_>],
    lists: &[Vec<PlanInput>],
) -> Vec<ObservationReach> {
    let mut reach: Vec<ObservationReach> = Vec::with_capacity(lists.len());
    for (draft, list) in drafts.iter().zip(lists) {
        let narrowed = draft.scope == StepScope::CrossFile
            && semantics(kind_of(draft.operation)).absence.0 != 0;
        let bound = narrowed
            || list.iter().any(|input| match input {
                PlanInput::Observation(_) => true,
                PlanInput::Node(read) => {
                    reach.get(usize::try_from(read.0).unwrap_or(usize::MAX))
                        == Some(&ObservationReach::Occurrence)
                }
                PlanInput::Relation(_) | PlanInput::Frontier(_) => false,
            });
        reach.push(if bound {
            ObservationReach::Occurrence
        } else {
            ObservationReach::Phase
        });
    }
    reach
}
