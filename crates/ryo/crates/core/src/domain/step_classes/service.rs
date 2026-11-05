//! The `step_classes` service: the classes of the `select` steps of the call's rules over the elements of a file, made from the plans
//! the call runs, whatever they were written as, by a scan of their steps, with no IO and no ports.

use super::constants::ELEMENT_COLUMNS;
use super::types::{Merge, MergedStep, Rooted, StepClass};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::operator::types::StepPlace;
use crate::domain::positions::types::ClassPlace;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{Applied, ColumnIndex, Comparison, Input, Operand};
use ryo_values::domain::plans::types::{Operation, Plan, StepScope};

/// `operand` over the tree's columns, when `columns` says which column of the tree each column it reads is.
fn over_tree(operand: &Operand, columns: &[ColumnIndex]) -> Option<Operand> {
    Some(match operand {
        Operand::Column(column) => Operand::Column(
            columns
                .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                .copied()?,
        ),
        Operand::Function(applied) => Operand::Function(Applied {
            function: applied.function,
            arguments: applied
                .arguments
                .iter()
                .map(|each| over_tree(each, columns))
                .collect::<Option<Vec<_>>>()?,
        }),
        Operand::Parameter(_) | Operand::Literal(_) => operand.clone(),
    })
}

/// What the step `operation` keeps of the elements of the tree, when it is a `select` of them or of a step that is: the comparisons
/// that select them, over the tree's columns, and which column of the tree each of its own columns is.
fn rooted(below: &[Option<Rooted>], operation: &Operation) -> Option<Rooted> {
    let Operation::Select {
        from,
        comparisons,
        kept,
    } = operation
    else {
        return None;
    };
    let (mut chosen, columns) = match from {
        Input::Relation(Relation::SyntaxTree) => (
            Vec::new(),
            (0..u32::try_from(ELEMENT_COLUMNS).unwrap_or(u32::MAX))
                .map(ColumnIndex)
                .collect::<Vec<_>>(),
        ),
        Input::Step(step) => {
            let under = below
                .get(usize::try_from(step.0).unwrap_or(usize::MAX))?
                .as_ref()?;
            (under.comparisons.clone(), under.columns.clone())
        }
        Input::Relation(_) | Input::BoundObservation(_) => return None,
    };
    for comparison in comparisons {
        chosen.push(Comparison {
            test: comparison.test,
            left: over_tree(&comparison.left, &columns)?,
            right: over_tree(&comparison.right, &columns)?,
            origin: comparison.origin,
        });
    }
    let kept = if kept.is_empty() {
        columns
    } else {
        kept.iter()
            .map(|each| {
                columns
                    .get(usize::try_from(each.0).unwrap_or(usize::MAX))
                    .copied()
            })
            .collect::<Option<Vec<_>>>()?
    };
    Some(Rooted {
        comparisons: chosen,
        columns: kept,
    })
}

/// Compare only a predicate operand's referenced request bindings, including nested functions.
/// A missing referenced slot is not a value and never proves shared evaluation safe.
fn operand_bindings_match(
    operand: &Operand,
    left: &[ParameterBinding],
    right: &[ParameterBinding],
) -> bool {
    match operand {
        Operand::Parameter(id) => usize::try_from(id.0)
            .ok()
            .and_then(|at| left.get(at).zip(right.get(at)))
            .is_some_and(|(left, right)| left == right),
        Operand::Function(applied) => applied
            .arguments
            .iter()
            .all(|argument| operand_bindings_match(argument, left, right)),
        Operand::Column(_) | Operand::Literal(_) => true,
    }
}

/// Keep each rule's missing-vector fallback and compare exact typed values only where read.
fn predicate_bindings_match(
    comparisons: &[Comparison],
    left: Option<&[ParameterBinding]>,
    right: Option<&[ParameterBinding]>,
) -> bool {
    let (Some(left), Some(right)) = (left, right) else {
        return false;
    };
    comparisons.iter().all(|comparison| {
        operand_bindings_match(&comparison.left, left, right)
            && operand_bindings_match(&comparison.right, left, right)
    })
}

/// The classes of the steps that select from the elements of a file, and for each rule and step the class that has it. Steps that
/// have the same comparisons under equal bindings of only their referenced parameters are one class: a step that selects from a step that
/// selects from the tree is in the class of the comparisons of both.
#[must_use]
pub(crate) fn merge(plans: &[Plan], bindings: &[Vec<ParameterBinding>]) -> Merge {
    let mut merged = Merge::default();
    for (rule, plan) in plans.iter().enumerate() {
        let mut of_rule = vec![None; plan.steps.len()];
        let mut below: Vec<Option<Rooted>> = Vec::with_capacity(plan.steps.len());
        for (step, each) in plan.steps.iter().enumerate() {
            let chosen = if each.scope == StepScope::PerFile {
                rooted(&below, &each.operation)
            } else {
                None
            };
            below.push(chosen.clone());
            let Some(chosen) = chosen.filter(|rooted| !rooted.comparisons.is_empty()) else {
                continue;
            };
            let member = MergedStep {
                rule: RuleIndex(rule),
                step: StepPlace(step),
            };
            let same = merged.classes.iter().position(|class| {
                class.comparisons == chosen.comparisons
                    && predicate_bindings_match(
                        &class.comparisons,
                        bindings.get(class.first.rule.0).map(Vec::as_slice),
                        bindings.get(rule).map(Vec::as_slice),
                    )
            });
            let place = if let Some(place) = same {
                if let Some(class) = merged.classes.get_mut(place) {
                    class.members.push(member);
                }
                place
            } else {
                merged.classes.push(StepClass {
                    first: member,
                    members: vec![member],
                    comparisons: chosen.comparisons,
                });
                merged.classes.len() - 1
            };
            if let Some(slot) = of_rule.get_mut(step) {
                *slot = Some(ClassPlace(place));
            }
        }
        merged.class_of.push(of_rule);
    }
    merged
}
