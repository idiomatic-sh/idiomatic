//! The parts of `logical`: the nodes and shapes a step's inputs read, and the nodes that start from the tree nodes of a kind.
//! it and the operators' physical plan.

use crate::domain::executable_program::constants::BOUND_STAGE;
use crate::domain::executable_program::types::{Draft, DraftInputs};
use crate::domain::kind_sets::service::names_a_kind;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::KindNaming;
use crate::domain::operation_schema::types::InputShape;
use ryo_operators::domain::comparing::service::fixed_prefix;
use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_operators::domain::tree_cells::service::native_form;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{Input, LiteralText, Operand, QueryLiteral, Test};
use ryo_values::domain::plans::types::Operation;

/// The inputs `named` reads, given the nodes of the rule's earlier `steps`; none when one reads a step that has no node.
pub(in crate::domain::executable_program) fn read_inputs<'s>(
    named: &[Input],
    steps: &[Result<PlanNodeId, OperatingError>],
    drafts: &'s [Draft<'_>],
    observed: &'s [Vec<CellKind>],
) -> Option<DraftInputs<'s>> {
    let mut read = DraftInputs {
        list: Vec::with_capacity(named.len()),
        shapes: Vec::with_capacity(named.len()),
        fault: None,
        stage: None,
    };
    for each in named {
        let (input, shape) = match each {
            Input::Relation(relation) => (
                PlanInput::Relation(*relation),
                InputShape {
                    kinds: kinds(*relation),
                    tree: native_form(*relation),
                },
            ),
            Input::Step(id) => {
                let Some(Ok(node)) = steps.get(id.0 as usize).cloned() else {
                    return None;
                };
                let draft = drafts.get(node.0 as usize)?;
                if let (Err(error), None) = (&draft.schema, &read.fault) {
                    read.fault = Some(error.clone());
                }
                (
                    PlanInput::Node(node),
                    InputShape {
                        kinds: draft.schema.as_deref().unwrap_or_default(),
                        tree: None,
                    },
                )
            }
            Input::BoundObservation(id) => {
                read.stage = Some(BOUND_STAGE);
                (
                    PlanInput::Observation(*id),
                    InputShape {
                        kinds: observed.get(id.0 as usize).map_or(&[], Vec::as_slice),
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
