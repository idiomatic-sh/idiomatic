//! The `binding` service: free functions named by verb over what a call binds its fragment's inputs and parameters to.

use crate::domain::linking::errors::LinkingError;
use crate::domain::linking::service::located;
use crate::domain::syntax::constants::BOOLEAN_WORDS;
use crate::domain::syntax::constants::fragment_scalar_kind_named;
use ryo_values::domain::query_bindings::types::ParameterType;
use ryo_values::domain::query_syntax::types::Argument;
use ryo_values::domain::query_syntax::types::ArgumentValue;
use ryo_values::domain::query_syntax::types::Call;
use ryo_values::domain::program::types::FragmentProgram;
use ryo_values::domain::program::types::LetStep;
use ryo_values::domain::query_tokens::types::QuerySourceId;
use ryo_values::domain::query_tokens::types::QueryTraceSite;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use super::types::{Bindings, BoundArguments, FragmentBindingKey, ScalarMatch};

/// Binds a call's named inputs and scalar parameters to a fragment signature.
#[expect(
    clippy::too_many_arguments,
    reason = "binding compares one call against its lexical inputs and declarations"
)]
pub(crate) fn bind_arguments(
    source: &QuerySourceId,
    fragment: &FragmentProgram,
    call: &Call,
    preceding: &[LetStep],
    inherited_inputs: &Bindings,
    inherited_parameters: &Bindings,
    root_parameters: &[ryo_values::domain::program::types::ParameterDeclaration],
    trace: &[QueryTraceSite],
) -> Result<BoundArguments, LinkingError> {
    let mut given = BTreeMap::new();
    for argument in &call.arguments {
        if given
            .insert(argument.name.0.clone(), &argument.value)
            .is_some()
        {
            return Err(located(
                source,
                call.span,
                trace.to_vec(),
                format!("argument `{}` is given more than once", argument.name.0),
            ));
        }
    }
    let expected = fragment
        .inputs
        .iter()
        .map(|input| input.name.0.as_ref())
        .chain(
            fragment
                .parameters
                .iter()
                .map(|parameter| parameter.name.0.as_ref()),
        )
        .collect::<BTreeSet<_>>();
    if let Some(unexpected) = given.keys().find(|name| !expected.contains(name.as_ref())) {
        return Err(located(
            source,
            call.span,
            trace.to_vec(),
            format!("fragment has no input or parameter named `{unexpected}`"),
        ));
    }
    let mut inputs = Bindings::default();
    for input in &fragment.inputs {
        let value = given.get(input.name.0.as_ref()).ok_or_else(|| {
            located(
                source,
                call.span,
                trace.to_vec(),
                format!("missing relation input `{}`", input.name.0),
            )
        })?;
        let value = bind_relation_argument(value, inherited_inputs, preceding);
        if !matches!(
            value,
            ArgumentValue::Text(_) | ArgumentValue::Step(_) | ArgumentValue::Word(_)
        ) {
            return Err(located(
                source,
                call.span,
                trace.to_vec(),
                format!(
                    "relation input `{}` must name a relation or step",
                    input.name.0
                ),
            ));
        }
        let _previous_input = inputs
            .0
            .insert(FragmentBindingKey(input.name.0.clone()), value);
    }
    let mut parameters = Bindings::default();
    for parameter in &fragment.parameters {
        let value = given.get(parameter.name.0.as_ref()).ok_or_else(|| {
            located(
                source,
                call.span,
                trace.to_vec(),
                format!("missing scalar parameter `{}`", parameter.name.0),
            )
        })?;
        let value = substitute_value(value, inherited_parameters);
        if scalar_matches(&parameter.kind, &value, root_parameters) == ScalarMatch::Differs {
            return Err(located(
                source,
                call.span,
                trace.to_vec(),
                format!(
                    "parameter `{}` has a value of the wrong type",
                    parameter.name.0
                ),
            ));
        }
        let _previous_parameter = parameters
            .0
            .insert(FragmentBindingKey(parameter.name.0.clone()), value);
    }
    Ok(BoundArguments { inputs, parameters })
}

/// Resolves a relation argument against inherited bindings and visible preceding steps.
pub(crate) fn bind_relation_argument(
    value: &ArgumentValue,
    inherited: &Bindings,
    preceding: &[LetStep],
) -> ArgumentValue {
    match value {
        ArgumentValue::Word(word) => inherited.0.get(&*word.0).cloned().unwrap_or_else(|| {
            preceding
                .iter()
                .rev()
                .find(|step| step.name.0 == word.0)
                .map_or_else(
                    || value.clone(),
                    |step| ArgumentValue::Step(step.name.clone()),
                )
        }),
        ArgumentValue::Text(_)
        | ArgumentValue::Number(_)
        | ArgumentValue::Step(_)
        | ArgumentValue::Parameter(_)
        | ArgumentValue::List(_)
        | ArgumentValue::Map(_) => substitute_value(value, inherited),
    }
}

/// Checks a fragment scalar argument against its declared parameter type.
pub(crate) fn scalar_matches(
    kind: &ryo_values::domain::query_tokens::types::TypeName,
    value: &ArgumentValue,
    root_parameters: &[ryo_values::domain::program::types::ParameterDeclaration],
) -> ScalarMatch {
    let Some(expected) = fragment_scalar_kind_named(&kind.0) else {
        return ScalarMatch::Differs;
    };
    let actual = match value {
        ArgumentValue::Text(_) => Some(ParameterType::Text),
        ArgumentValue::Number(_) => Some(ParameterType::Number),
        ArgumentValue::Word(word) if BOOLEAN_WORDS.contains(&word.0.as_ref()) => {
            Some(ParameterType::Flag)
        }
        ArgumentValue::Parameter(parameter) => root_parameters
            .iter()
            .find(|declared| declared.name.0.as_ref() == parameter.0.as_ref())
            .and_then(|declared| fragment_scalar_kind_named(&declared.kind.0)),
        ArgumentValue::Word(_)
        | ArgumentValue::Step(_)
        | ArgumentValue::List(_)
        | ArgumentValue::Map(_) => None,
    };
    if actual == Some(expected) {
        ScalarMatch::Matches
    } else {
        ScalarMatch::Differs
    }
}

/// Recursively substitutes scalar parameters in an argument value.
pub(crate) fn substitute_value(value: &ArgumentValue, bindings: &Bindings) -> ArgumentValue {
    match value {
        ArgumentValue::Parameter(parameter) => bindings
            .0
            .get(&*parameter.0)
            .cloned()
            .unwrap_or_else(|| value.clone()),
        ArgumentValue::List(values) => ArgumentValue::List(
            values
                .iter()
                .map(|nested_value| substitute_value(nested_value, bindings))
                .collect(),
        ),
        ArgumentValue::Map(arguments) => ArgumentValue::Map(
            arguments
                .iter()
                .map(|argument| Argument {
                    name: argument.name.clone(),
                    value: substitute_value(&argument.value, bindings),
                })
                .collect(),
        ),
        ArgumentValue::Text(_)
        | ArgumentValue::Number(_)
        | ArgumentValue::Word(_)
        | ArgumentValue::Step(_) => value.clone(),
    }
}

/// Replaces an argument with its recursively substituted value.
pub(crate) fn substitute_argument_value(value: &mut ArgumentValue, bindings: &Bindings) {
    *value = substitute_value(value, bindings);
}

/// The first parameter reference in a call value that has no local fragment binding.
/// Finds the first parameter reference absent from the current fragment's declarations.
pub(crate) fn unbound_parameter<'a>(
    value: &'a ArgumentValue,
    bindings: &Bindings,
) -> Option<&'a ryo_values::domain::query_tokens::types::WrittenParameter> {
    match value {
        ArgumentValue::Parameter(parameter) => {
            (!bindings.0.contains_key(&*parameter.0)).then_some(parameter)
        }
        ArgumentValue::List(values) => values
            .iter()
            .find_map(|nested_value| unbound_parameter(nested_value, bindings)),
        ArgumentValue::Map(arguments) => arguments
            .iter()
            .find_map(|argument| unbound_parameter(&argument.value, bindings)),
        ArgumentValue::Text(_)
        | ArgumentValue::Number(_)
        | ArgumentValue::Word(_)
        | ArgumentValue::Step(_) => None,
    }
}
