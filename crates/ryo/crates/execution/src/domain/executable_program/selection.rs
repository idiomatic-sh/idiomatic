//! Pure, request-qualified stored-selection alternatives.
//! Acquisition, population identity, capacity, and release stay with the program owner.

use super::types::region::value::Region;
use super::types::SelectionAlternative;
use super::types::{InputSource, OperandSource, RegionId, RuleProgram};
use crate::domain::{
    coverage::types::RuleIndex, operator::contract::types::RequestOperators,
    stored_selection::types::Selector,
};
use ryo_relations::domain::relations::{service::kinds, types::Relation};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input, RepeatedRows, StepScope},
    plans::types::Operation,
};
use std::sync::Arc;

fn direct_relation(operation: &Operation) -> Option<Relation> {
    match operation {
        Operation::Select {
            from: Input::Relation(relation),
            ..
        }
        | Operation::Project {
            from: Input::Relation(relation),
            ..
        } => Some(*relation),
        _ => None,
    }
}

enum OutputMap {
    Exact,
    Mismatch,
}

fn exact_output(operation: &Operation, selector: &Selector, relation: Relation) -> OutputMap {
    let columns = match operation {
        Operation::Select { kept, .. } if kept.is_empty() => {
            let mut columns = Vec::with_capacity(kinds(relation).len());
            for at in 0..kinds(relation).len() {
                let Ok(at) = u32::try_from(at) else {
                    return OutputMap::Mismatch;
                };
                columns.push(ColumnIndex(at));
            }
            Some(columns)
        }
        Operation::Select { kept, .. } => Some(kept.clone()),
        Operation::Project { columns, .. } => Some(columns.clone()),
        _ => None,
    };
    let distinct = match operation {
        Operation::Project { distinct, .. } => *distinct,
        _ => RepeatedRows::Kept,
    };
    if columns.is_some_and(|columns| columns == selector.columns)
        && selector.distinct == distinct
        && selector.relation == relation
    {
        OutputMap::Exact
    } else {
        OutputMap::Mismatch
    }
}

/// Compile all eligible producer rules, including observation-only producers.
/// A chain or fused region stays on its ordinary route: its ancestor demand and
/// error order cannot be inferred from a terminal selector alone.
pub(crate) fn compile(
    rules: &[RuleProgram],
    logical_regions: &[RegionId],
    regions: &[Region],
    prepared: &RequestOperators,
) -> Vec<SelectionAlternative> {
    let mut alternatives = Vec::new();
    for (rule, mapping) in rules.iter().enumerate() {
        let Some(selected) = prepared.selection_plans.get(rule) else {
            continue;
        };
        let Some(physical) = prepared.physical.get(rule) else {
            continue;
        };
        if selected.steps.len() != mapping.original_steps.len()
            || physical.len() != mapping.original_steps.len()
        {
            continue;
        }
        for (at, origin) in mapping.original_steps.iter().enumerate() {
            let Ok(origin) = origin else { continue };
            if origin.rule != RuleIndex(rule) {
                continue;
            }
            let Some(selector) = selected.steps[at].as_ref() else {
                continue;
            };
            if !physical[at]
                .selector
                .as_ref()
                .is_some_and(|original| Arc::ptr_eq(original, selector))
                || !prepared
                    .selection_views
                    .by_selector
                    .contains_key(&(Arc::as_ptr(selector) as usize))
            {
                continue;
            }
            let Some(&target) = logical_regions.get(origin.logical.0) else {
                continue;
            };
            let Some(region) = regions.get(target.0) else {
                continue;
            };
            if region.id() != target
                || region.phase() != StepScope::PerFile
                || region.producer() != RuleIndex(rule)
                || region.instructions().len() != 1
                || region.inputs().len() != 1
            {
                continue;
            }
            let instruction = &region.instructions()[0];
            let Some(relation) = direct_relation(instruction.operation()) else {
                continue;
            };
            if instruction.logical() != origin.logical
                || instruction.producer() != RuleIndex(rule)
                || instruction.inputs() != [OperandSource::Boundary(super::types::InputId(0))]
                || region.inputs()[0] != InputSource::Relation(relation)
                || !matches!(
                    exact_output(instruction.operation(), selector, relation),
                    OutputMap::Exact
                )
                || selector.identity.comparisons.len()
                    != match instruction.operation() {
                        Operation::Select { comparisons, .. } => comparisons.len(),
                        Operation::Project { .. } => 0,
                        _ => unreachable!(),
                    }
            {
                continue;
            }
            alternatives.push(SelectionAlternative {
                target,
                selector: Arc::clone(selector),
                producer: RuleIndex(rule),
                logical: origin.logical,
            });
        }
    }
    alternatives
}
