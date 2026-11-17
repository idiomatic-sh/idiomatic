//! The `activation` service: each node's requirement from its operator's contract, and every node's state in one file,
//! composed in node order from its inputs'. No rows, IO or ports.
use crate::domain::executable_program::activation::types::{
    ActivationInput, ActivationPlan, Activities, FactAnswer, FactSlot, KindSlot, NodeActivity,
    NodeRequirement, NodeState, TriggerPresence, Triggers,
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

/// Whether `node` runs in the file `triggers` and `facts` describe: dormant only when it can have neither a row nor a row
/// error. Its state is composed when it is first asked for, and kept in `into` for the rest of the file: from its inputs'
/// states, its node inputs first and then its fact relations, each in operand order, as the runner readies and reads them.
/// `facts` answers a relation's read: each relation is read once in the file, so a read here is the read its readers get.
/// A refused read stops the composition there: the node and every node waiting on it run, and meet the refusal. A node the
/// plan does not hold runs.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::service::activity;
/// use ryo_execution_domain::domain::executable_program::activation::types::{
///     ActivationPlan, Activities, FactAnswer, NodeActivity, Triggers,
/// };
/// use ryo_operators::domain::physical_plan::types::PlanNodeId;
/// let mut into = Activities::default();
/// let answer = activity(&ActivationPlan::default(), &Triggers::default(), &mut |_| FactAnswer::Rows, &mut into, PlanNodeId(0));
/// assert_eq!(answer.unwrap(), NodeActivity::Active);
/// ```
///
/// # Errors
///
/// `ShapeMismatch` when `triggers` lacks a kind slot of the plan, or a node reads a node after it or a fact slot the plan does
/// not hold: the triggers or the plan were made for another program.
pub fn activity(
    plan: &ActivationPlan,
    triggers: &Triggers,
    facts: &mut dyn FnMut(Relation) -> FactAnswer,
    into: &mut Activities,
    node: PlanNodeId,
) -> Result<NodeActivity, OperatingError> {
    if into.0.len() < plan.nodes.len() {
        into.0.resize(plan.nodes.len(), None);
    }
    let mut composing = Composing {
        plan,
        triggers,
        facts,
        into,
        refusal: Refusal::NotMet,
    };
    Ok(match composing.state(node)? {
        NodeState {
            rows: TriggerPresence::Absent,
            faults: TriggerPresence::Absent,
        } => NodeActivity::Dormant,
        NodeState { .. } => NodeActivity::Active,
    })
}

/// Whether a composition met a refused read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Refusal {
    /// It did not.
    NotMet,
    /// It did: nothing more is read, and every node waiting runs.
    Met,
}

/// One composition of node states in a file.
struct Composing<'c> {
    plan: &'c ActivationPlan,
    triggers: &'c Triggers,
    facts: &'c mut dyn FnMut(Relation) -> FactAnswer,
    into: &'c mut Activities,
    refusal: Refusal,
}

impl Composing<'_> {
    /// `node`'s state, composed once and kept.
    fn state(&mut self, node: PlanNodeId) -> Result<NodeState, OperatingError> {
        let at = usize::try_from(node.0).map_err(|_| OperatingError::ShapeMismatch)?;
        if let Some(Some(known)) = self.into.0.get(at) {
            return Ok(*known);
        }
        let plan = self.plan;
        let state = match plan.nodes.get(at) {
            None | Some(NodeRequirement::Live) => NodeState::default(),
            Some(NodeRequirement::Kinds { slots, fallible }) => {
                let rows = any_kind(self.triggers, slots)?;
                // The prefix cannot fail; a comparison after it fails only on a row.
                let faults = if rows == TriggerPresence::Present && fallible.0 != 0 {
                    TriggerPresence::Present
                } else {
                    TriggerPresence::Absent
                };
                NodeState { rows, faults }
            }
            Some(NodeRequirement::Inputs {
                rows,
                fallible,
                from,
            }) => self.composed(node, *rows, *fallible, from)?,
        };
        if let Some(slot) = self.into.0.get_mut(at) {
            *slot = Some(state);
        }
        Ok(state)
    }

    /// An operation's state from its inputs', in the order the runner meets them: its node inputs, then its fact relations.
    /// Its rows are as its presence mask declares, and it can fault when an input carries a fault or a fallible input can have
    /// rows. An input a mask names past the node's inputs cannot be shown empty, so it is read as having rows.
    fn composed(
        &mut self,
        node: PlanNodeId,
        rows: PresenceMask,
        fallible: InputMask,
        from: &[ActivationInput],
    ) -> Result<NodeState, OperatingError> {
        let mut with_rows = u8::MAX
            .checked_shl(u32::try_from(from.len()).unwrap_or(u32::MAX))
            .unwrap_or(0);
        let mut carried = TriggerPresence::Absent;
        let bit = |place: usize| {
            1_u8.checked_shl(u32::try_from(place).unwrap_or(u32::MAX))
                .unwrap_or(0)
        };
        for (place, input) in from.iter().enumerate() {
            let ActivationInput::Node(read) = input else {
                continue;
            };
            if *read >= node {
                return Err(OperatingError::ShapeMismatch);
            }
            let state = self.state(*read)?;
            if self.refusal == Refusal::Met {
                return Ok(NodeState::default());
            }
            if state.rows == TriggerPresence::Present {
                with_rows |= bit(place);
            }
            if state.faults == TriggerPresence::Present {
                carried = TriggerPresence::Present;
            }
        }
        for (place, input) in from.iter().enumerate() {
            match input {
                ActivationInput::Node(_) => {}
                ActivationInput::Tree => with_rows |= bit(place),
                ActivationInput::Fact(FactSlot(slot)) => {
                    let relation = *self
                        .plan
                        .facts
                        .get(*slot)
                        .ok_or(OperatingError::ShapeMismatch)?;
                    match (self.facts)(relation) {
                        FactAnswer::Rows => with_rows |= bit(place),
                        FactAnswer::NoRow => {}
                        FactAnswer::Refused => {
                            self.refusal = Refusal::Met;
                            return Ok(NodeState::default());
                        }
                    }
                }
            }
        }
        let has_rows = match rows {
            PresenceMask::AllOf(mask) => mask.0 & !with_rows == 0,
            PresenceMask::AnyOf(mask) => mask.0 & with_rows != 0,
            PresenceMask::Always => true,
        };
        Ok(NodeState {
            rows: if has_rows {
                TriggerPresence::Present
            } else {
                TriggerPresence::Absent
            },
            faults: if fallible.0 & with_rows != 0 {
                TriggerPresence::Present
            } else {
                carried
            },
        })
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
