//! The services of the `nodes` part: the steps a rule's emit reaches, each visited once; the shape of an input; and the pack's
//! store of merged plan nodes with every node's readers. A node is named by its digest alone (`digests::service::node_digests`).

use super::types::{
    AcceptedPlace, AdmittedRule, NodeHome, NodeReader, NodeStore, ReadColumns, StepReach,
    StepShape, StoredNode,
};
use crate::domain::lowering::types::SelectedRule;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input, InputNumber, StepId};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_plans::types::NodeDigests;
use ryo_values::domain::query_plans::types::{NodeDigest, Plan};
use std::collections::{BTreeSet, HashSet};

/// The steps of `plan` its emit reaches, by step: each step is visited once, whatever the number of paths to it. A step may
/// read a step after it (the closure a reach join is unfolded into is appended after the plan's last step): the order the
/// steps are evaluated in is the values crate's `read_order`, not their ids. The step named is one that is not in the plan.
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
                Input::Step(read) => waiting.push(*read),
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

/// What a step reads of the rows of its inputs: a projection reads the columns it keeps; every other operation is taken to
/// read the whole row. Every operation is named: a new one reads the whole row until it says otherwise here.
fn read_by(operation: &Operation) -> ReadColumns {
    match operation {
        Operation::Project { columns, .. } => ReadColumns::Kept(columns.iter().copied().collect()),
        Operation::Select { .. }
        | Operation::Extend { .. }
        | Operation::Join { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::Replace { .. }
        | Operation::OrderedMatch { .. }
        | Operation::AsOf { .. }
        | Operation::GroupBound { .. }
        | Operation::GroupOrdinal { .. }
        | Operation::Words { .. }
        | Operation::Axis { .. }
        | Operation::Closure { .. }
        | Operation::Follow { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Worklist { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::SourceMatches { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => ReadColumns::Whole,
    }
}

/// The columns two readers read together: the whole row when either reads it.
fn together(read: ReadColumns, more: ReadColumns) -> ReadColumns {
    match (read, more) {
        (ReadColumns::Kept(mut kept), ReadColumns::Kept(more)) => {
            kept.extend(more);
            ReadColumns::Kept(kept)
        }
        (ReadColumns::Whole, ReadColumns::Kept(_) | ReadColumns::Whole)
        | (ReadColumns::Kept(_), ReadColumns::Whole) => ReadColumns::Whole,
    }
}

/// The pack's store: every step some accepted rule's emit reaches, once per digest, homed in the first rule that holds it,
/// with its distinct readers across the pack and the columns they read of it. Two rules that hold one sub-plan give its
/// top step two readers; the steps below it have the one reader they have in either rule.
#[must_use]
pub(crate) fn stored(rules: &[AdmittedRule<'_>]) -> NodeStore {
    let mut store = NodeStore::default();
    for (place, rule) in rules.iter().enumerate() {
        let mut read = |digest: NodeDigest, step: StepId, reader: NodeReader, columns| {
            let node = store.0.entry(digest).or_insert_with(|| StoredNode {
                home: NodeHome {
                    rule: AcceptedPlace(place),
                    step,
                },
                readers: HashSet::new(),
                read: ReadColumns::Kept(BTreeSet::new()),
            });
            let _new = node.readers.insert(reader);
            let before = std::mem::replace(&mut node.read, ReadColumns::Whole);
            node.read = together(before, columns);
        };
        let emitted = rule.rule.plan.emit.step;
        read(
            digest_of(rule, emitted),
            emitted,
            NodeReader::Emit(rule.rule.rule),
            ReadColumns::Whole,
        );
        for (at, each) in rule.rule.plan.steps.iter().enumerate() {
            if rule.shapes.get(at).is_none_or(Option::is_none) {
                continue;
            }
            let reader = rule.digests.0.get(at).copied().unwrap_or_default();
            for (number, input) in named(&inputs_of(&each.operation)).iter().enumerate() {
                if let Input::Step(step) = input {
                    let by = NodeReader::Step(reader, InputNumber(number));
                    read(digest_of(rule, *step), *step, by, read_by(&each.operation));
                }
            }
        }
    }
    store
}

/// The columns of a node's row its readers across the pack read, ascending, when every reader is a projection and they
/// keep fewer columns together than the row's `width`; none when some reader reads the whole row, when they keep every
/// column, and for a node the store does not hold.
#[must_use]
pub(crate) fn kept_columns(
    store: &NodeStore,
    digest: NodeDigest,
    width: Count,
) -> Option<Vec<ColumnIndex>> {
    let ReadColumns::Kept(kept) = &store.0.get(&digest)?.read else {
        return None;
    };
    let within = kept.iter().all(|column| usize::from(*column) < width.0);
    (within && !kept.is_empty() && kept.len() < width.0).then(|| kept.iter().copied().collect())
}

/// How many distinct readers a node has across the pack; none for a node the store does not hold.
#[must_use]
pub(crate) fn readers(store: &NodeStore, digest: NodeDigest) -> Count {
    Count(store.0.get(&digest).map_or(0, |node| node.readers.len()))
}

/// The digest of every step of a rule, in step order: the one place the lowering takes digests from. They are data the
/// host hands to the load with each rule; the engine hashes nothing.
#[must_use]
pub(crate) fn digested(rule: &SelectedRule<'_>) -> NodeDigests {
    rule.digests.clone()
}
