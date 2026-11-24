//! The `numbering` service: a step's number in the plan being made, equal steps being one.

use core::hash::BuildHasher as _;
use crate::domain::plan_lowering::expansion::types::Expansion;
use ryo_values::domain::finish_plans::arguments::literals::types::ArgumentValue as StepArgument;
use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::expressions::types::{ConstantEntry, PlanConstant};
use ryo_values::domain::finish_plans::types::FinishStep;

/// The number of a step: an equal step's when one was made, else the next. A step that holds no map is its own identity
/// and is compared as it is; only a step that holds one is copied into its identity.
pub(crate) fn numbered(expansion: &mut Expansion<'_>, step: FinishStep) -> StepNumber {
    let Expansion { steps, numbers, hasher, .. } = expansion;
    interned(steps, numbers, hasher, step)
}

pub(crate) fn interned(
    steps: &mut Vec<FinishStep>,
    numbers: &mut hashbrown::HashTable<(u64, StepNumber)>,
    hasher: &hashbrown::DefaultHashBuilder,
    step: FinishStep,
) -> StepNumber {
    let mapped = holds_map(&step);
    let identity_form = mapped.then(|| identity(&step));
    let key = identity_form.as_ref().unwrap_or(&step);
    let hash = hasher.hash_one(key);
    let found = numbers.find(hash, |(_, number)| {
        steps.get(number.0).is_some_and(|made| {
            if mapped {
                identity(made) == *key
            } else {
                made == key
            }
        })
    });
    if let Some((_, number)) = found {
        return *number;
    }
    let number = StepNumber(steps.len());
    // Growing the index must not re-hash every earlier plan expression.
    let _slot = numbers.insert_unique(hash, (hash, number), |(hash, _)| *hash);
    steps.push(step);
    number
}

/// Whether a step holds a map, whose entries its identity orders.
fn holds_map(step: &FinishStep) -> bool {
    matches!(step, FinishStep::Extend(_) | FinishStep::AsOf(_) | FinishStep::OrderedMatch(_) | FinishStep::Unlisted(_))
}

/// A step's identity: the step with the entries of every map it holds in key order, so that two steps differing only
/// in the order a map was written are one.
fn identity(step: &FinishStep) -> FinishStep {
    let mut identity = step.clone();
    match &mut identity {
        FinishStep::Extend(extend) => {
            extend.values.sort_by(|a, b| a.name.cmp(&b.name));
            for value in &mut extend.values {
                sorted_constant(&mut value.value);
            }
        }
        FinishStep::AsOf(as_of) => as_of.output.sort_by(|a, b| a.name.cmp(&b.name)),
        FinishStep::OrderedMatch(matched) => matched.output.sort_by(|a, b| a.name.cmp(&b.name)),
        FinishStep::Unlisted(unlisted) => {
            unlisted.arguments.sort_by(|a, b| a.name.cmp(&b.name));
            for argument in &mut unlisted.arguments {
                if let StepArgument::Constant(constant) = &mut argument.value {
                    sorted_constant(constant);
                }
            }
        }
        FinishStep::Base(_)
        | FinishStep::Select(_)
        | FinishStep::Union(_)
        | FinishStep::Antijoin(_)
        | FinishStep::Axis(_)
        | FinishStep::Closure(_)
        | FinishStep::Follow(_)
        | FinishStep::OrderedDfs(_)
        | FinishStep::GroupCount(_)
        | FinishStep::GroupMin(_)
        | FinishStep::GroupMax(_)
        | FinishStep::GroupOrdinal(_)
        | FinishStep::Project(_)
        | FinishStep::SpanBounds(_)
        | FinishStep::SpanBetween(_)
        | FinishStep::Replace(_)
        | FinishStep::SourceMatches(_)
        | FinishStep::Capture(_)
        | FinishStep::FitsRendered(_)
        | FinishStep::RenderSyntax(_)
        | FinishStep::Rows(_)
        | FinishStep::Prefixes(_)
        | FinishStep::SubtreeClass(_) => {}
    }
    identity
}

/// A constant with every map's entries in key order.
fn sorted_constant(constant: &mut PlanConstant) {
    match constant {
        PlanConstant::List(items) => items.iter_mut().for_each(sorted_constant),
        PlanConstant::Map(entries) => {
            entries.sort_by(|a: &ConstantEntry, b: &ConstantEntry| a.key.cmp(&b.key));
            for entry in entries {
                sorted_constant(&mut entry.value);
            }
        }
        PlanConstant::Text(_) | PlanConstant::Number(_) | PlanConstant::Absent => {}
    }
}
