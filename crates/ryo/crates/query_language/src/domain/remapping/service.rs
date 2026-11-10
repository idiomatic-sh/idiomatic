//! The `remapping` service: free functions named by verb over the rewriting of a fragment step's identities, inputs and parameters at a call site.

use super::types::ParameterCheck;
use crate::domain::binding::service::substitute_argument_value;
use crate::domain::binding::service::unbound_parameter;
use crate::domain::binding::types::Bindings;
use crate::domain::linking::errors::LinkingError;
use crate::domain::linking::service::located;
use crate::domain::syntax::service::shape_named;
use ryo_values::domain::program::types::LetStep;
use ryo_values::domain::query_syntax::types::ArgumentValue;
use ryo_values::domain::query_syntax::types::Call;
use ryo_values::domain::query_syntax::types::Condition;
use ryo_values::domain::query_syntax::types::Expression;
use ryo_values::domain::query_syntax::types::Filter;
use ryo_values::domain::query_syntax::types::Select;
use ryo_values::domain::query_syntax::types::SourceReference;
use ryo_values::domain::query_syntax::types::WrittenApplied;
use ryo_values::domain::query_syntax::types::WrittenOperand;
use ryo_values::domain::query_tokens::types::QuerySourceId;
use ryo_values::domain::query_tokens::types::QuerySpan;
use ryo_values::domain::query_tokens::types::QueryTraceSite;
use ryo_values::domain::query_tokens::types::StepIdentity;
use ryo_values::domain::query_tokens::types::StepName;
use std::collections::BTreeMap;

/// Rewrites one fragment step's local identities, inputs, parameters, and nested calls.
pub(crate) fn remap_step(
    step: &mut LetStep,
    identities: &BTreeMap<StepIdentity, StepIdentity>,
    inputs: &Bindings,
    parameters: &Bindings,
    source: &QuerySourceId,
    preceding: &[LetStep],
    trace: &[QueryTraceSite],
) -> Result<(), LinkingError> {
    if let Some(identity) = ryo_values::domain::program::service::identity_of(step.name.1) {
        step.name.1 =
            ryo_values::domain::program::service::identified(identities.get(&identity).copied());
    }
    match &mut step.expression {
        Expression::Select(select) => {
            remap_select(select, identities, inputs, parameters, source, trace)?;
        }
        Expression::Call(call) => {
            remap_call(
                call,
                inputs,
                parameters,
                preceding,
                source,
                trace,
                ParameterCheck::Strict,
            )?;
        }
    }
    Ok(())
}

/// Rewrites step and relation sources in a select to their caller bindings.
pub(crate) fn remap_select(
    select: &mut Select,
    identities: &BTreeMap<StepIdentity, StepIdentity>,
    inputs: &Bindings,
    parameters: &Bindings,
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
) -> Result<(), LinkingError> {
    for query_source in std::iter::once(&mut select.from).chain(match &mut select.join {
        ryo_values::domain::query_syntax::types::Joining::Alone => None,
        ryo_values::domain::query_syntax::types::Joining::Joined(join) => Some(&mut join.source),
    }) {
        let SourceReference::Step(reference) = &query_source.reference else {
            continue;
        };
        let replacement = if let Some(identity) =
            ryo_values::domain::program::service::identity_of(reference.1)
        {
            identities.get(&identity).copied().map(|identity| {
                SourceReference::Step(StepName(
                    reference.0.clone().into(),
                    ryo_values::domain::query_tokens::types::StepIdentified::Assigned(identity),
                ))
            })
        } else {
            inputs
                .0
                .get(&crate::domain::binding::types::FragmentBindingKey(
                    reference.0.0.clone(),
                ))
                .and_then(|input| match input {
                    ArgumentValue::Text(name) => Some(SourceReference::Relation(
                        ryo_values::domain::query_tokens::types::WrittenRelation(name.0.clone()),
                    )),
                    ArgumentValue::Step(step) => Some(SourceReference::Step(step.clone())),
                    ArgumentValue::Word(word) => Some(SourceReference::Step(StepName(
                        word.0.clone().into(),
                        ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
                    ))),
                    ArgumentValue::Number(_)
                    | ArgumentValue::Parameter(_)
                    | ArgumentValue::List(_)
                    | ArgumentValue::Map(_) => None,
                })
        };
        if let Some(replacement) = replacement {
            query_source.reference = replacement;
        }
    }
    for condition in &mut select.filters {
        remap_condition(condition, parameters, source, trace)?;
    }
    Ok(())
}

/// Rewrites parameters through a nested boolean condition.
pub(crate) fn remap_condition(
    condition: &mut Condition,
    parameters: &Bindings,
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
) -> Result<(), LinkingError> {
    match condition {
        Condition::Test(filter) => remap_filter(filter, parameters, source, trace),
        Condition::Any(alternatives) => {
            for alternative in alternatives {
                for nested_condition in alternative {
                    remap_condition(nested_condition, parameters, source, trace)?;
                }
            }
            Ok(())
        }
    }
}

/// Rewrites both operands in one filter.
pub(crate) fn remap_filter(
    filter: &mut Filter,
    parameters: &Bindings,
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
) -> Result<(), LinkingError> {
    remap_operand(&mut filter.left, parameters, filter.span, source, trace)?;
    remap_operand(&mut filter.right, parameters, filter.span, source, trace)
}

/// Rewrites a parameter operand using the current fragment bindings.
pub(crate) fn remap_operand(
    operand: &mut WrittenOperand,
    parameters: &Bindings,
    span: QuerySpan,
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
) -> Result<(), LinkingError> {
    match operand {
        WrittenOperand::Parameter(parameter) => {
            let value = parameters.0.get(&*parameter.0).ok_or_else(|| {
                located(
                    source,
                    span,
                    trace.to_vec(),
                    format!("fragment parameter `${}` was not bound", parameter.0),
                )
            })?;
            *operand = match value {
                ArgumentValue::Text(value) => WrittenOperand::Text(value.clone()),
                ArgumentValue::Number(value) => WrittenOperand::Number(*value),
                ArgumentValue::Parameter(value) => WrittenOperand::Parameter(value.clone()),
                ArgumentValue::Word(_)
                | ArgumentValue::Step(_)
                | ArgumentValue::List(_)
                | ArgumentValue::Map(_) => {
                    return Err(located(
                        source,
                        span,
                        trace.to_vec(),
                        super::constants::INVALID_FILTER_PARAMETER,
                    ));
                }
            };
        }
        WrittenOperand::Function(WrittenApplied { arguments, .. }) => {
            for argument in arguments {
                remap_operand(argument, parameters, span, source, trace)?;
            }
        }
        WrittenOperand::Column(_) | WrittenOperand::Text(_) | WrittenOperand::Number(_) => {}
    }
    Ok(())
}

/// Substitutes fragment arguments and resolves relation inputs at a call site.
pub(crate) fn remap_call(
    call: &mut Call,
    inputs: &Bindings,
    parameters: &Bindings,
    preceding: &[LetStep],
    source: &QuerySourceId,
    trace: &[QueryTraceSite],
    strictness: ParameterCheck,
) -> Result<(), LinkingError> {
    if strictness == ParameterCheck::Strict
        && let Some(parameter) = call
            .arguments
            .iter()
            .find_map(|argument| unbound_parameter(&argument.value, parameters))
    {
        return Err(located(
            source,
            call.span,
            trace.to_vec(),
            format!("fragment parameter `${}` is not declared", parameter.0),
        ));
    }
    for argument in &mut call.arguments {
        substitute_argument_value(&mut argument.value, parameters);
    }
    let input_slots = shape_named(&call.name).map_or(&[][..], |shape| shape.inputs);
    for argument in &mut call.arguments {
        let is_input = crate::domain::syntax::service::is_input(input_slots, &argument.name)
            == crate::domain::syntax::types::ArgumentRole::Input;
        if !is_input {
            continue;
        }
        if let ArgumentValue::Word(word) = &argument.value {
            argument.value = inputs.0.get(&*word.0).cloned().unwrap_or_else(|| {
                preceding
                    .iter()
                    .rev()
                    .find(|step| *step.name.0 == *word.0)
                    .map_or_else(
                        || argument.value.clone(),
                        |step| ArgumentValue::Step(step.name.clone()),
                    )
            });
        }
        if !matches!(
            argument.value,
            ArgumentValue::Text(_) | ArgumentValue::Step(_) | ArgumentValue::Word(_)
        ) {
            return Err(located(
                source,
                call.span,
                trace.to_vec(),
                format!(
                    "relation input `{}` must name a relation or step",
                    argument.name.0
                ),
            ));
        }
    }
    Ok(())
}
