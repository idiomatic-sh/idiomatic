//! The `activation` service: each node's requirement from its operator's contract, and every node's state in one file,
//! composed in node order from its inputs'. No rows, IO or ports.
use crate::domain::executable_program::activation::types::{
    ActivationInput, ActivationPlan, Activities, FactSlot, KindSlot, NodeActivity, NodeRequirement,
    NodeState, TriggerPresence, Triggers,
};
use crate::domain::executable_program::types::{Draft, SchemaTiming};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::presence::types::{InputMask, PresenceMask, RowFailure};
use crate::domain::operator::contract::service::{failure, semantics};
use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_operators::domain::tree_cells::service::native_form;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::plan_parts::types::{LiteralText, StepScope};
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::plans::types::Operation;

/// The root kinds a file is asked about, by kind slot: the order a file's kind lists are made in.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::service::trigger_kinds;
/// use ryo_execution_domain::domain::executable_program::activation::types::ActivationPlan;
/// assert!(trigger_kinds(&ActivationPlan::default()).is_empty());
/// ```
#[must_use]
pub fn trigger_kinds(plan: &ActivationPlan) -> &[LiteralText] {
    &plan.kinds
}

/// The fact relations a file is asked about, by fact slot.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::service::trigger_facts;
/// use ryo_execution_domain::domain::executable_program::activation::types::ActivationPlan;
/// assert!(trigger_facts(&ActivationPlan::default()).is_empty());
/// ```
#[must_use]
pub fn trigger_facts(plan: &ActivationPlan) -> &[Relation] {
    &plan.facts
}

/// Writes into `into` every node's state in a file holding `triggers`, in node order: a node's inputs come first.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::service::activities;
/// use ryo_execution_domain::domain::executable_program::activation::types::{ActivationPlan, Activities, Triggers};
/// let mut into = Activities::default();
/// activities(&ActivationPlan::default(), &Triggers::default(), &mut into).unwrap();
/// assert!(into.0.is_empty());
/// ```
///
/// # Errors
///
/// `ShapeMismatch` when `triggers` lacks a slot of the plan or a node reads one after it: the triggers were made for another
/// plan.
pub fn activities(
    plan: &ActivationPlan,
    triggers: &Triggers,
    into: &mut Activities,
) -> Result<(), OperatingError> {
    into.0.clear();
    for requirement in &*plan.nodes {
        let state = match requirement {
            NodeRequirement::Live => NodeState::default(),
            NodeRequirement::Kinds { slots, fallible } => {
                let rows = any_kind(triggers, slots)?;
                // The prefix cannot fail; a comparison after it fails only on a row.
                let faults = if rows == TriggerPresence::Present && fallible.0 != 0 {
                    TriggerPresence::Present
                } else {
                    TriggerPresence::Absent
                };
                NodeState { rows, faults }
            }
            NodeRequirement::Inputs {
                rows,
                fallible,
                from,
            } => composed(*rows, *fallible, from, triggers, &into.0)?,
        };
        into.0.push(state);
    }
    Ok(())
}

/// Whether `node` runs in the file `activities` were written for: dormant only when it can have neither a row nor a row
/// error. A node the activities do not hold runs.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::service::activity;
/// use ryo_execution_domain::domain::executable_program::activation::types::{Activities, NodeActivity};
/// use ryo_operators::domain::physical_plan::types::PlanNodeId;
/// assert_eq!(activity(&Activities::default(), PlanNodeId(0)), NodeActivity::Active);
/// ```
#[must_use]
pub fn activity(activities: &Activities, node: PlanNodeId) -> NodeActivity {
    match activities
        .0
        .get(usize::try_from(node.0).unwrap_or(usize::MAX))
    {
        Some(NodeState {
            rows: TriggerPresence::Absent,
            faults: TriggerPresence::Absent,
        }) => NodeActivity::Dormant,
        Some(_) | None => NodeActivity::Active,
    }
}

/// A root select's rows: present where the file holds any of its kinds.
fn any_kind(triggers: &Triggers, slots: &[KindSlot]) -> Result<TriggerPresence, OperatingError> {
    for slot in slots {
        let held = triggers
            .kinds
            .get(slot.0)
            .ok_or(OperatingError::ShapeMismatch)?;
        if *held == TriggerPresence::Present {
            return Ok(TriggerPresence::Present);
        }
    }
    Ok(TriggerPresence::Absent)
}

/// One input's state: an earlier node's, a fact relation's rows (reading one cannot fail), or the tree's.
fn input_state(
    source: ActivationInput,
    triggers: &Triggers,
    earlier: &[NodeState],
) -> Result<NodeState, OperatingError> {
    Ok(match source {
        ActivationInput::Node(node) => *earlier
            .get(usize::try_from(node.0).map_err(|_| OperatingError::ShapeMismatch)?)
            .ok_or(OperatingError::ShapeMismatch)?,
        ActivationInput::Fact(FactSlot(slot)) => NodeState {
            rows: *triggers
                .facts
                .get(slot)
                .ok_or(OperatingError::ShapeMismatch)?,
            faults: TriggerPresence::Absent,
        },
        ActivationInput::Tree => NodeState {
            rows: TriggerPresence::Present,
            faults: TriggerPresence::Absent,
        },
    })
}

/// Whether the inputs `mask` names can have rows, each in operand order: an input the mask names past the node's inputs is
/// read as having them.
fn masked_rows<'s>(
    states: &'s [NodeState],
    mask: InputMask,
) -> impl Iterator<Item = TriggerPresence> + 's {
    (0..8_usize)
        .filter(move |place| mask.0 & (1 << place) != 0)
        .map(|place| match states.get(place) {
            Some(state) => state.rows,
            // An input the operation names and the node does not read cannot be shown empty.
            None => TriggerPresence::Present,
        })
}

/// An operation's state from its inputs': its rows as its presence mask declares, and a row error when an input carries one
/// or a fallible input can have rows.
fn composed(
    rows: PresenceMask,
    fallible: InputMask,
    from: &[ActivationInput],
    triggers: &Triggers,
    earlier: &[NodeState],
) -> Result<NodeState, OperatingError> {
    let states = from
        .iter()
        .map(|source| input_state(*source, triggers, earlier))
        .collect::<Result<Vec<NodeState>, OperatingError>>()?;
    let present = |held: TriggerPresence| held == TriggerPresence::Present;
    let has_rows = match rows {
        PresenceMask::AllOf(mask) => masked_rows(&states, mask).all(present),
        PresenceMask::AnyOf(mask) => masked_rows(&states, mask).any(present),
        PresenceMask::Always => true,
    };
    let carried = states
        .iter()
        .any(|state| state.faults == TriggerPresence::Present);
    let raised = masked_rows(&states, fallible).any(present);
    Ok(NodeState {
        rows: if has_rows {
            TriggerPresence::Present
        } else {
            TriggerPresence::Absent
        },
        faults: if carried || raised {
            TriggerPresence::Present
        } else {
            TriggerPresence::Absent
        },
    })
}

/// The activation plan of the drafted nodes of a logical program, by node id: each node's requirement from its operator's
/// contract (`semantics(kind).presence`, `failure`), a tree select `roots` names read as its kinds, and every other node
/// `Live` when it runs after the files, its schema is refused or not as stated, it is hosted, or it reads a frontier or a
/// bound observation.
pub(in crate::domain::executable_program) fn plan_of(
    drafts: &[Draft<'_>],
    lists: &[Vec<PlanInput>],
    roots: &hashbrown::HashMap<LiteralText, Vec<PlanNodeId>>,
) -> ActivationPlan {
    let mut kinds: Vec<LiteralText> = Vec::with_capacity(roots.len());
    let mut root_slots: Vec<Vec<KindSlot>> = vec![Vec::new(); drafts.len()];
    for (kind, nodes) in roots {
        let slot = KindSlot(kinds.len());
        kinds.push(kind.clone());
        for node in nodes {
            if let Some(slots) = root_slots.get_mut(usize::try_from(node.0).unwrap_or(usize::MAX)) {
                slots.push(slot);
            }
        }
    }
    let mut facts: Vec<Relation> = Vec::new();
    let mut nodes = Vec::with_capacity(drafts.len());
    for ((draft, list), slots) in drafts.iter().zip(lists).zip(root_slots) {
        // A node after the files, of a refused schema or one answered later, or reading a host's relation, whose withholding
        // is its fault, may have rows and row errors in any file.
        let read = if draft.scope != StepScope::PerFile
            || draft.schema.is_err()
            || draft.timing != SchemaTiming::Stated
            || matches!(draft.operation, Operation::Hosted { .. })
        {
            None
        } else {
            read_inputs(list, &mut facts)
        };
        let requirement = match read {
            Some(from) => {
                let fallible = fallible(draft, list, drafts);
                if slots.is_empty() {
                    NodeRequirement::Inputs {
                        rows: semantics(kind_of(draft.operation)).presence,
                        fallible,
                        from,
                    }
                } else {
                    NodeRequirement::Kinds {
                        slots: slots.into_boxed_slice(),
                        fallible,
                    }
                }
            }
            None => NodeRequirement::Live,
        };
        nodes.push(requirement);
    }
    ActivationPlan {
        nodes: nodes.into_boxed_slice(),
        kinds: kinds.into_boxed_slice(),
        facts: facts.into_boxed_slice(),
    }
}

/// A node's inputs as activation reads them, each fact relation given its slot among `facts`; none when it reads a frontier
/// or a bound observation, whose rows no file trigger tells.
fn read_inputs(list: &[PlanInput], facts: &mut Vec<Relation>) -> Option<Box<[ActivationInput]>> {
    list.iter()
        .map(|input| match input {
            PlanInput::Node(node) => Some(ActivationInput::Node(*node)),
            PlanInput::Relation(relation) if native_form(*relation).is_some() => {
                Some(ActivationInput::Tree)
            }
            PlanInput::Relation(relation) => {
                let at = match facts.iter().position(|each| each == relation) {
                    Some(at) => at,
                    None => {
                        facts.push(*relation);
                        facts.len() - 1
                    }
                };
                Some(ActivationInput::Fact(FactSlot(at)))
            }
            PlanInput::Frontier(_) | PlanInput::Observation(_) => None,
        })
        .collect()
}

/// The inputs whose rows can make a drafted node fail, as its contract declares: a search that fails unless its span is
/// located is read as failing on its first input.
fn fallible(draft: &Draft<'_>, list: &[PlanInput], drafts: &[Draft<'_>]) -> InputMask {
    let first = list
        .first()
        .map_or(&[][..], |input| input_kinds(drafts, input));
    let shapes = [InputShape {
        kinds: first,
        tree: None,
    }];
    match failure(draft.operation, &shapes, draft.scope, None) {
        RowFailure::Never => InputMask(0),
        RowFailure::OnRows(mask) => mask,
        RowFailure::UnlessLocated(_) => InputMask(1),
    }
}

/// The column kinds an input of a drafted node reads: none for an input whose kinds the drafts do not hold.
fn input_kinds<'a>(drafts: &'a [Draft<'_>], input: &PlanInput) -> &'a [CellKind] {
    match input {
        PlanInput::Node(node) => match drafts
            .get(usize::try_from(node.0).unwrap_or(usize::MAX))
            .map(|draft| &draft.schema)
        {
            Some(Ok(kinds)) => kinds.as_slice(),
            // A refused input makes its reader `Live` before its kinds are read.
            Some(Err(_)) | None => &[],
        },
        PlanInput::Relation(relation) => {
            ryo_relations::domain::relations::service::kinds(*relation)
        }
        PlanInput::Frontier(_) | PlanInput::Observation(_) => &[],
    }
}
