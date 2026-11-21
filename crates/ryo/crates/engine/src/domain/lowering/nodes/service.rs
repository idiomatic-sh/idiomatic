//! The services of the `nodes` part: the steps a rule's emit reaches, each visited once; the shape of an input; and the pack's
//! store of merged plan nodes with every node's readers. A node is named by its digest alone (`digests::service::node_digests`).

use super::types::{
    AcceptedPlace, AdmittedRule, NodeHome, NodeReader, NodeStore, StepReach, StepShape, StoredNode,
};
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::{Input, InputNumber, StepId};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::query_plans::types::{NodeDigest, Plan};
use std::collections::HashSet;

/// The steps of `plan` its emit reaches, by step: each step is visited once, whatever the number of paths to it. None when a
/// step reads a step that is not before it: such a plan cannot be evaluated, and the step is named.
pub(crate) fn reached(plan: &Plan) -> Result<Vec<StepReach>, StepId> {
    let mut reached = vec![StepReach::Unreached; plan.steps.len()];
    let mut waiting = vec![plan.emit.step];
    while let Some(step) = waiting.pop() {
        let place = usize::from(step);
        match reached.get_mut(place) {
            Some(StepReach::Reached) => continue,
            Some(seen) => *seen = StepReach::Reached,
            None => return Err(step),
        }
        let Some(each) = plan.steps.get(place) else {
            return Err(step);
        };
        for input in named(&inputs_of(&each.operation)) {
            match input {
                Input::Step(read) if read.0 < step.0 => waiting.push(*read),
                Input::Step(_) => return Err(step),
                Input::Relation(_) | Input::BoundObservation(_) => {}
            }
        }
    }
    Ok(reached)
}

/// The shape of an input: a relation's columns as the catalog has them, a step's as computed; none for a step with no shape
/// yet and for the host's observation rows, which no step names the columns of.
#[must_use]
pub(crate) fn input_shape(shapes: &[Option<StepShape>], input: Input) -> Option<StepShape> {
    match input {
        Input::Relation(relation) => Some(StepShape(kinds(relation).to_vec())),
        Input::Step(step) => shapes.get(usize::from(step)).cloned().flatten(),
        Input::BoundObservation(_) => None,
    }
}

/// The digest of a step of a rule.
#[must_use]
pub(crate) fn digest_of(rule: &AdmittedRule<'_>, step: StepId) -> NodeDigest {
    rule.digests
        .0
        .get(usize::from(step))
        .copied()
        .unwrap_or_default()
}

/// The pack's store: every step some accepted rule's emit reaches, once per digest, homed in the first rule that holds it,
/// with its distinct readers across the pack. Two rules that hold one sub-plan give its top step two readers; the steps
/// below it have the one reader they have in either rule.
#[must_use]
pub(crate) fn stored(rules: &[AdmittedRule<'_>]) -> NodeStore {
    let mut store = NodeStore::default();
    for (place, rule) in rules.iter().enumerate() {
        let mut read = |digest: NodeDigest, step: StepId, reader: NodeReader| {
            let node = store.0.entry(digest).or_insert_with(|| StoredNode {
                home: NodeHome {
                    rule: AcceptedPlace(place),
                    step,
                },
                readers: HashSet::new(),
            });
            let _new = node.readers.insert(reader);
        };
        let emitted = rule.rule.plan.emit.step;
        read(
            digest_of(rule, emitted),
            emitted,
            NodeReader::Emit(rule.rule.rule),
        );
        for (at, each) in rule.rule.plan.steps.iter().enumerate() {
            if rule.shapes.get(at).is_none_or(Option::is_none) {
                continue;
            }
            let reader = rule.digests.0.get(at).copied().unwrap_or_default();
            for (number, input) in named(&inputs_of(&each.operation)).iter().enumerate() {
                if let Input::Step(step) = input {
                    let by = NodeReader::Step(reader, InputNumber(number));
                    read(digest_of(rule, *step), *step, by);
                }
            }
        }
    }
    store
}

/// How many distinct readers a node has across the pack; none for a node the store does not hold.
#[must_use]
pub(crate) fn readers(store: &NodeStore, digest: NodeDigest) -> Count {
    Count(store.0.get(&digest).map_or(0, |node| node.readers.len()))
}

/// The digest of every step of a rule, in step order: the one place the lowering takes digests from.
#[must_use]
pub(crate) fn digested(
    rule: &crate::domain::lowering::types::SelectedRule<'_>,
) -> ryo_values::domain::query_plans::types::NodeDigests {
    crate::domain::digests::service::node_digests(rule.plan, rule.bindings)
}
