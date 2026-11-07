//! Equivalent typed source predicates and pure projections, with no IO and no ports.

use super::types::{BindingsAgreement, Merge, MergedStep, Rooted, StepClass};
use crate::domain::coverage::types::RuleIndex;
use crate::domain::operator::types::StepPlace;
use crate::domain::step_classes::types::ClassPlace;
use ryo_values::domain::plan_parts::types::{Applied, ColumnIndex, Input, Operand, PlanComparison};
use ryo_values::domain::plans::types::{Operation, Plan};
use ryo_values::domain::query_bindings::types::ParameterBinding;

/// `operand` over the source's columns, using the projection's column map.
fn over_source(operand: &Operand, columns: &[ColumnIndex]) -> Option<Operand> {
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
                .map(|each| over_source(each, columns))
                .collect::<Option<Vec<_>>>()?,
        }),
        Operand::Parameter(_) | Operand::Literal(_) => operand.clone(),
    })
}

/// Map pure selection/projection predicates and columns to their original relation.
fn rooted(below: &[Option<Rooted>], operation: &Operation) -> Option<Rooted> {
    let (from, comparisons, kept) = match operation {
        Operation::Select {
            from,
            comparisons,
            kept,
        } => (from, comparisons.as_slice(), kept),
        Operation::Project {
            from,
            columns,
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
        } => (from, &[][..], columns),
        _ => return None,
    };
    let (source, mut chosen, columns) = match from {
        Input::Relation(relation) => {
            let Ok(width) =
                u32::try_from(ryo_relations::domain::relations::service::columns(*relation).len())
            else {
                return None;
            };
            (
                *relation,
                Vec::new(),
                (0..width).map(ColumnIndex).collect::<Vec<_>>(),
            )
        }
        Input::Step(step) => {
            let under = below
                .get(usize::try_from(step.0).unwrap_or(usize::MAX))?
                .as_ref()?;
            (
                under.source,
                under.comparisons.clone(),
                under.columns.clone(),
            )
        }
        Input::BoundObservation(_) => return None,
    };
    for comparison in comparisons {
        chosen.push(PlanComparison {
            test: comparison.test,
            left: over_source(&comparison.left, &columns)?,
            right: over_source(&comparison.right, &columns)?,
            origin: comparison.origin,
        });
    }
    let kept = if kept.is_empty() && matches!(operation, Operation::Select { .. }) {
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
        source,
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
) -> BindingsAgreement {
    match operand {
        Operand::Parameter(id) => {
            let Ok(at) = usize::try_from(id.0) else {
                return BindingsAgreement::Differ;
            };
            match left.get(at).zip(right.get(at)) {
                Some((left, right)) if left == right => BindingsAgreement::Equal,
                _ => BindingsAgreement::Differ,
            }
        }
        Operand::Function(applied) => {
            if applied.arguments.iter().all(|argument| {
                operand_bindings_match(argument, left, right) == BindingsAgreement::Equal
            }) {
                BindingsAgreement::Equal
            } else {
                BindingsAgreement::Differ
            }
        }
        Operand::Column(_) | Operand::Literal(_) => BindingsAgreement::Equal,
    }
}

/// Keep each rule's missing-vector fallback and compare exact typed values only where read.
pub(crate) fn predicate_bindings_match(
    comparisons: &[PlanComparison],
    left: Option<&[ParameterBinding]>,
    right: Option<&[ParameterBinding]>,
) -> BindingsAgreement {
    let (Some(left), Some(right)) = (left, right) else {
        return BindingsAgreement::Differ;
    };
    if comparisons.iter().all(|comparison| {
        operand_bindings_match(&comparison.left, left, right) == BindingsAgreement::Equal
            && operand_bindings_match(&comparison.right, left, right) == BindingsAgreement::Equal
    }) {
        BindingsAgreement::Equal
    } else {
        BindingsAgreement::Differ
    }
}

/// Group selections by source relation, ordered predicates and referenced typed bindings.
/// Preceding selections contribute their predicates before the current step's predicates.
#[must_use]
pub(crate) fn merge(plans: &[Plan], bindings: &[Vec<ParameterBinding>]) -> Merge {
    let mut merged = Merge::default();
    // Classes bucketed by source relation: a class only ever matches its own source.
    let mut by_source: std::collections::HashMap<
        ryo_relations::domain::relations::types::Relation,
        Vec<usize>,
    > = std::collections::HashMap::new();
    for (rule, plan) in plans.iter().enumerate() {
        let mut of_rule = vec![None; plan.steps.len()];
        let mut below: Vec<Option<Rooted>> = Vec::with_capacity(plan.steps.len());
        for (step, each) in plan.steps.iter().enumerate() {
            // A step shares only the rows of its own partition: a step whose input is a step of
            // the other partition reads a frontier, never the rows that step selected.
            let crosses = matches!(
                &each.operation,
                Operation::Select { from: Input::Step(under), .. }
                    | Operation::Project { from: Input::Step(under), .. }
                    if plan
                        .steps
                        .get(usize::try_from(under.0).unwrap_or(usize::MAX))
                        .is_none_or(|under| under.scope != each.scope)
            );
            let chosen = if crosses {
                None
            } else {
                rooted(&below, &each.operation)
            };
            below.push(chosen.clone());
            let Some(chosen) = chosen else {
                continue;
            };
            let member = MergedStep {
                rule: RuleIndex(rule),
                step: StepPlace(step),
            };
            let same = by_source.get(&chosen.source).and_then(|candidates| {
                candidates.iter().copied().find(|&at| {
                    let class = &merged.classes[at];
                    class.scope == each.scope
                        && class.comparisons == chosen.comparisons
                        && predicate_bindings_match(
                            &class.comparisons,
                            bindings.get(class.first.rule.0).map(Vec::as_slice),
                            bindings.get(rule).map(Vec::as_slice),
                        ) == BindingsAgreement::Equal
                })
            });
            let place = if let Some(place) = same {
                if let Some(class) = merged.classes.get_mut(place) {
                    class.members.push(member);
                }
                place
            } else {
                by_source
                    .entry(chosen.source)
                    .or_default()
                    .push(merged.classes.len());
                merged.classes.push(StepClass {
                    source: chosen.source,
                    scope: each.scope,
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
