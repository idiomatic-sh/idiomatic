//! The `query_plans` service: free functions named by verb over a whole compiled query, with no IO and no ports: the
//! relations it reads, the order its steps are read in, their digests, and the plan the new engine receives of it, each
//! join that holds a reach a `Closure` step and a plain `Join` again.

use super::digesting::service::step_digest;
use super::types::{NodeBytes, NodeDigest, NodeDigests, Plan, RuleReads, StepMet};
use crate::domain::plan_parts::types::{
    ColumnIndex, Input, ReachLookup, RepeatedRows, StepId, StepScope,
};
use crate::domain::plans::service::{inputs_of, named};
use crate::domain::plans::types::{Operation, PlanStep};
use crate::domain::query_bindings::types::RuleBindings;
use crate::domain::written_sites::types::StepWritten;
use ryo_relations::domain::relations::types::Count;
use std::borrow::Cow;

/// The relations `plan`'s steps read, each once, in the order a step first reads it: the one walk of a rule's read set.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::query_plans::service::reads_of;
/// use ryo_values::domain::query_plans::types::Plan;
/// fn reads(plan: &Plan) -> usize {
///     reads_of(plan).0.len()
/// }
/// ```
#[must_use]
pub fn reads_of(plan: &Plan) -> RuleReads {
    let mut reads = Vec::new();
    for step in &plan.steps {
        for input in named(&inputs_of(&step.operation)) {
            if let Input::Relation(relation) = *input
                && !reads.contains(&relation)
            {
                reads.push(relation);
            }
        }
    }
    RuleReads(reads.into_boxed_slice())
}

/// The step a place of a plan's steps is.
fn step_at(place: Count) -> StepId {
    StepId(u32::try_from(place.0).unwrap_or(u32::MAX))
}

/// The steps of `plan`, each once, every step after the steps it reads: the order a walk that needs a step's inputs first
/// takes them in. A compiled plan is in that order already ("each step reads only the steps before it") and is answered as
/// it stands; the plan `reach_unfolded` answers is not, because a join of it reads a step appended after the plan's last.
/// Each step is met once and its inputs are looked at once more for each of them that stands after it, whatever the plan's
/// depth. Steps that read each other in a ring have no such order and stand in the order met.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::query_plans::service::read_order;
/// use ryo_values::domain::query_plans::types::Plan;
///
/// fn first(plan: &Plan) -> Option<u32> {
///     read_order(plan).first().map(|step| step.0)
/// }
/// ```
#[must_use]
pub fn read_order(plan: &Plan) -> Vec<StepId> {
    let mut met = vec![StepMet::Unmet; plan.steps.len()];
    let mut order = Vec::with_capacity(plan.steps.len());
    let mut waiting: Vec<usize> = Vec::new();
    for first in 0..plan.steps.len() {
        if met.get(first) != Some(&StepMet::Unmet) {
            continue;
        }
        waiting.push(first);
        while let Some(at) = waiting.last().copied() {
            if let Some(mark) = met.get_mut(at) {
                *mark = StepMet::Waiting;
            }
            // The first step it reads that no walk has met yet: that one is placed before it.
            let unread = plan.steps.get(at).and_then(|step| {
                named(&inputs_of(&step.operation))
                    .iter()
                    .find_map(|input| match input {
                        Input::Step(read) => Some(usize::from(*read))
                            .filter(|read| met.get(*read) == Some(&StepMet::Unmet)),
                        Input::Relation(_) | Input::BoundObservation(_) => None,
                    })
            });
            match unread {
                Some(read) => waiting.push(read),
                None => {
                    if let Some(mark) = met.get_mut(at) {
                        *mark = StepMet::Placed;
                    }
                    order.push(step_at(Count(at)));
                    let _placed = waiting.pop();
                }
            }
        }
    }
    order
}

/// The digest of every step of `plan`, by step, each step visited once: the steps are digested in `read_order`, so each is
/// written after the steps it reads, once, and `digest` is asked once a step, whatever the plan's depth. A step's digest
/// covers its operation and constants, where it runs, each input's digest (a relation by its catalog name) and, for each
/// parameter it names, the value `bindings` binds it to: two rules whose plans differ only in a bound value never share a
/// digest, and two equal sub-plans of any two rules share one whatever their places in their plans. That holds of a plan
/// `reach_unfolded` answers too: a join that reads a step appended after it holds that step's digest.
///
/// `digest` is the caller's: this crate hashes nothing. It is given a step's bytes, numbers at fixed widths with the least
/// byte first, so with one `digest` a plan's digests are the same on every machine; the host that owns the hash pins one.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::query_bindings::types::RuleBindings;
/// use ryo_values::domain::query_plans::service::node_digests;
/// use ryo_values::domain::query_plans::types::{NodeBytes, NodeDigest, Plan};
///
/// fn digests(plan: &Plan, digest: &dyn Fn(NodeBytes<'_>) -> NodeDigest) -> usize {
///     node_digests(plan, RuleBindings(&[]), digest).0.len()
/// }
/// ```
#[must_use]
pub fn node_digests(
    plan: &Plan,
    bindings: RuleBindings<'_>,
    digest: &dyn Fn(NodeBytes<'_>) -> NodeDigest,
) -> NodeDigests {
    let mut digests: Vec<Option<NodeDigest>> = vec![None; plan.steps.len()];
    for at in read_order(plan) {
        let Some(step) = plan.steps.get(usize::from(at)) else {
            continue;
        };
        let node = step_digest(plan, step, bindings, &digests, digest);
        if let Some(place) = digests.get_mut(usize::from(at)) {
            *place = Some(node);
        }
    }
    NodeDigests(digests.into_iter().map(Option::unwrap_or_default).collect())
}

/// The columns a reach projects its closure by, when they are not the closure's own: a closure's row is its partition
/// keys, then the source, then the destination, and a reach whose `columns` name exactly those in that order projects
/// nothing.
fn reach_projection(reach: &ReachLookup) -> Option<&[ColumnIndex]> {
    let own = reach.keys.len().saturating_add(2);
    let places = (0..own).map(|place| u32::try_from(place).unwrap_or(u32::MAX));
    let projects = !reach.columns.iter().map(|column| column.0).eq(places);
    projects.then_some(&reach.columns)
}

/// Appends the steps a reach join's right side is: the closure of the edges the join reads, and after it the projection of
/// its rows by the reach's columns when there is one. Answers the step the join then reads.
fn reach_appended(
    steps: &mut Vec<PlanStep>,
    edges: Input,
    reach: &ReachLookup,
    scope: StepScope,
    written: &StepWritten,
) -> StepId {
    let closure = step_at(Count(steps.len()));
    steps.push(PlanStep {
        operation: Operation::Closure {
            edges,
            keys: reach.keys.clone(),
            source: reach.source,
            destination: reach.destination,
        },
        scope,
        written: written.clone(),
    });
    let Some(columns) = reach_projection(reach) else {
        return closure;
    };
    let projected = step_at(Count(steps.len()));
    steps.push(PlanStep {
        operation: Operation::Project {
            from: Input::Step(closure),
            columns: columns.to_vec(),
            distinct: RepeatedRows::Kept,
        },
        scope,
        written: written.clone(),
    });
    projected
}

/// The plan the new engine receives of a compiled one: every join that holds a `reach` (a transitive closure the compiler
/// folded into the join as a lookup, for today's engine) is a `Closure` step and a plain `Join` again. The closure is a step
/// of the plan, so it has a node digest and the lowering has something to make a definition of.
///
/// Every step of `plan` keeps its `StepId`, so whatever names a step of the rule's own plan (what it emits, its frontier, a
/// work notice, a cost site) still names it. For each reach join, in step order, a `Closure` over the edges the join read
/// is appended after the plan's last step, with the reach's keys, source and destination; where the reach's `columns` are
/// not the closure's own, a `Project` of the closure by them is appended after it, its rows not made distinct (each
/// entity reached stands for one right row, as in today's join). The join's right input is re-pointed at the last step
/// appended for it and its `reach` is none; nothing else of it changes. So the answered plan is not in step order: a join
/// reads a step after it. Nothing else does: an appended step reads the edges, which stood before the join, or the step
/// appended just before it. A walk that needs a step's inputs first takes the steps in `read_order`.
///
/// Each step is visited once. A plan with no reach join is answered as it is, borrowed.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::query_plans::service::reach_unfolded;
/// use ryo_values::domain::query_plans::types::Plan;
/// use std::borrow::Cow;
///
/// fn unchanged(plan: &Plan) -> bool {
///     matches!(reach_unfolded(plan), Cow::Borrowed(_))
/// }
/// ```
#[must_use]
pub fn reach_unfolded(plan: &Plan) -> Cow<'_, Plan> {
    let mut unfolded: Option<Plan> = None;
    for (at, step) in plan.steps.iter().enumerate() {
        let Operation::Join {
            right: edges,
            reach: Some(lookup),
            ..
        } = &step.operation
        else {
            continue;
        };
        let whole = unfolded.get_or_insert_with(|| plan.clone());
        let logical = reach_appended(&mut whole.steps, *edges, lookup, step.scope, &step.written);
        if let Some(PlanStep {
            operation: Operation::Join { right, reach, .. },
            ..
        }) = whole.steps.get_mut(at)
        {
            *right = Input::Step(logical);
            *reach = None;
        }
    }
    unfolded.map_or(Cow::Borrowed(plan), Cow::Owned)
}

/// The step of a compiled plan that a step of its unfolded plan (`reach_unfolded`) belongs to: the step itself, or for an
/// appended step the join it was unfolded from. A notice or a cost site that names a step of the unfolded plan names a
/// statement of the rule through this. A step neither plan holds is answered as it is.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::plan_parts::types::StepId;
/// use ryo_values::domain::query_plans::service::compiled_step;
/// use ryo_values::domain::query_plans::types::Plan;
///
/// fn first(compiled: &Plan) -> StepId {
///     compiled_step(compiled, StepId(0))
/// }
/// ```
#[must_use]
pub fn compiled_step(compiled: &Plan, step: StepId) -> StepId {
    if usize::from(step) < compiled.steps.len() {
        return step;
    }
    let mut appended = compiled.steps.len();
    for (at, each) in compiled.steps.iter().enumerate() {
        let Operation::Join {
            reach: Some(lookup),
            ..
        } = &each.operation
        else {
            continue;
        };
        let steps = reach_projection(lookup).map_or(1_usize, |_| 2);
        appended = appended.saturating_add(steps);
        if usize::from(step) < appended {
            return step_at(Count(at));
        }
    }
    step
}
