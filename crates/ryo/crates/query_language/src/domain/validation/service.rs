//! The `validation` service: free functions that check a program against ryo's own relations,
//! with no IO and no ports. It checks that every name a query writes names something; the types
//! are checked where the names are resolved, in `lowering`.

use ryo_relations::domain::relations::service::relation_named;
use ryo_relations::domain::relations::types::NameRef;
use ryo_relations::domain::relations::types::Relation;

use ryo_values::domain::hosted::types::HostedName;

use super::errors::ValidationError;
use super::types::Known;
use crate::domain::syntax::constants::{operator_named, slot_name};
use crate::domain::syntax::service::{is_input, shape_named};
use crate::domain::syntax::types::ArgumentRole;
use ryo_values::domain::program::types::{
    ArgumentValue, Call, Columns, Condition, Expression, Filter, Joining, ParameterDeclaration,
    Program, QuerySource, QuerySpan, QuerySpelling, Select, SourceReference, WrittenOperand,
};
use std::collections::BTreeSet;

/// Checks that every relation, step, parameter and operation a program names exists, and that
/// every call has its required arguments.
///
/// # Errors
///
/// The first thing wrong, with the place it is written.
#[cfg(test)]
pub(crate) fn validate(
    program: &Program,
    catalog: &[Relation],
    hosted: &[HostedName],
) -> Result<(), ValidationError> {
    validate_with_observations(program, catalog, hosted, &[])
}

/// Validate declared typed named inputs without admitting arbitrary unknown step names.
pub(crate) fn validate_with_observations(
    program: &Program,
    catalog: &[Relation],
    hosted: &[HostedName],
    observations: &[ryo_values::domain::observations::types::BoundObservationSchema],
) -> Result<(), ValidationError> {
    if program.linked {
        let mut identities = BTreeSet::new();
        for step in &program.steps {
            let Some(identity) = step.name.1 else {
                return Err(ValidationError::UnknownStep {
                    name: step.name.0.to_string(),
                    span: step.span,
                });
            };
            if !identities.insert(identity) {
                return Err(ValidationError::DuplicateStep {
                    name: step.name.0.to_string(),
                    span: step.span,
                });
            }
        }
    }
    let mut known = Known {
        linked: program.linked,
        steps: Vec::new(),
        hosted,
        observations,
    };
    for step in &program.steps {
        if !program.linked && known.steps.iter().any(|known| known.0 == step.name.0) {
            return Err(ValidationError::DuplicateStep {
                name: step.name.0.to_string(),
                span: step.span,
            });
        }
        let checked = match &step.expression {
            Expression::Select(select) => {
                check_select(select, &known, catalog, &program.parameters)
            }
            Expression::Call(call) => check_call(call, &known, catalog, &program.parameters),
        };
        checked.map_err(|error| {
            match step
                .name
                .1
                .and_then(|identity| program.step_origins.get(&identity))
                .cloned()
            {
                Some(origin) => ValidationError::Located {
                    origin,
                    error: Box::new(error),
                },
                None => error,
            }
        })?;
        known.steps.push(&step.name);
    }
    let mut exports = BTreeSet::new();
    for output in &program.observations {
        let error = if output.name.0.trim().is_empty() {
            Some(ValidationError::BlankObservation { span: output.span })
        } else if !exports.insert(&output.name) {
            Some(ValidationError::DuplicateObservation {
                name: output.name.0.to_string(),
                span: output.span,
            })
        } else if !known.steps.contains(&&output.step) {
            Some(ValidationError::UnknownStep {
                name: output.step.0.to_string(),
                span: output.span,
            })
        } else {
            None
        };
        if let Some(error) = error {
            return Err(match output.origin.clone() {
                Some(origin) => ValidationError::Located {
                    origin,
                    error: Box::new(error),
                },
                None => error,
            });
        }
    }
    if !known.steps.contains(&&program.emit.step) {
        let error = ValidationError::UnknownStep {
            name: program.emit.step.0.to_string(),
            span: program.emit.span,
        };
        return Err(match program.emit_origin.clone() {
            Some(origin) => ValidationError::Located {
                origin,
                error: Box::new(error),
            },
            None => error,
        });
    }
    Ok(())
}

/// That a `select` reads known sources, names columns when it joins, and filters only on declared parameters.
fn check_select(
    select: &Select,
    known: &Known<'_>,
    catalog: &[Relation],
    parameters: &[ParameterDeclaration],
) -> Result<(), ValidationError> {
    check_source(&select.from, known, catalog)?;
    if let Joining::Joined(join) = &select.join {
        check_source(&join.source, known, catalog)?;
        if select.columns == Columns::Every {
            return Err(ValidationError::JoinNeedsColumns {
                span: select.from.span,
            });
        }
    }
    check_conditions(&select.filters, parameters)
}

/// That each parameter the tests of `conditions` read, however deep in an `or`, is declared.
fn check_conditions(
    conditions: &[Condition],
    parameters: &[ParameterDeclaration],
) -> Result<(), ValidationError> {
    for condition in conditions {
        match condition {
            Condition::Test(filter) => check_filter(filter, parameters)?,
            Condition::Any(alternatives) => {
                for alternative in alternatives {
                    check_conditions(alternative, parameters)?;
                }
            }
        }
    }
    Ok(())
}

/// That the source a statement reads, a relation or an earlier step, is one the program can read.
fn check_source(
    source: &QuerySource,
    known: &Known<'_>,
    catalog: &[Relation],
) -> Result<(), ValidationError> {
    match &source.reference {
        SourceReference::Relation(text) => {
            check_relation(QuerySpelling(&text.0), source.span, catalog, known)
        }
        SourceReference::Step(text) => {
            let exists = match text.1 {
                Some(identity) => known.steps.iter().any(|step| step.1 == Some(identity)),
                None if known
                    .observations
                    .iter()
                    .any(|bound| bound.name.0 == text.0) =>
                {
                    true
                }
                None if known.linked => false,
                None => known.steps.iter().any(|step| step.0 == text.0),
            };
            if exists {
                Ok(())
            } else {
                Err(ValidationError::UnknownStep {
                    name: text.0.to_string(),
                    span: source.span,
                })
            }
        }
    }
}

/// That the relation `text` names is one the host offers or one of the catalog's.
fn check_relation(
    text: QuerySpelling<'_>,
    span: QuerySpan,
    catalog: &[Relation],
    known: &Known<'_>,
) -> Result<(), ValidationError> {
    if known.hosted.iter().any(|name| *name.0 == *text.0)
        || known
            .observations
            .iter()
            .any(|bound| *bound.name.0 == *text.0)
    {
        return Ok(());
    }
    match relation_named(NameRef(text.0)) {
        Some(relation) if catalog.contains(&relation) => Ok(()),
        Some(_) | None => Err(ValidationError::UnknownRelation {
            name: text.0.to_owned(),
            span,
        }),
    }
}

/// That the step `text` reads is one the program has declared before it.
fn check_step(
    text: QuerySpelling<'_>,
    span: QuerySpan,
    known: &Known<'_>,
) -> Result<(), ValidationError> {
    if known.steps.iter().any(|step| *step.0 == *text.0)
        || known
            .observations
            .iter()
            .any(|bound| *bound.name.0 == *text.0)
    {
        return Ok(());
    }
    Err(ValidationError::UnknownStep {
        name: text.0.to_owned(),
        span,
    })
}

/// That `call` names an operation the language has and gives it the arguments its shape asks for, each of the right kind.
fn check_call(
    call: &Call,
    known: &Known<'_>,
    catalog: &[Relation],
    parameters: &[ParameterDeclaration],
) -> Result<(), ValidationError> {
    let Some(shape) = shape_named(&call.name) else {
        return Err(ValidationError::UnknownOperation {
            name: call.name.0.to_string(),
            span: call.span,
        });
    };
    debug_assert!(
        operator_named(&call.name.0).is_some(),
        "every operation with a shape is an operator"
    );
    for required in shape.inputs.iter().chain(shape.required) {
        let given = call
            .arguments
            .iter()
            .any(|argument| *argument.name.0 == *slot_name(*required));
        if !given {
            return Err(ValidationError::MissingArgument {
                operation: call.name.0.to_string(),
                argument: slot_name(*required).to_owned(),
                span: call.span,
            });
        }
    }
    for argument in &call.arguments {
        check_value(&argument.value, parameters, call)?;
        if is_input(shape.inputs, &argument.name) == ArgumentRole::Input {
            match &argument.value {
                ArgumentValue::Text(text) => {
                    check_relation(QuerySpelling(&text.0), call.span, catalog, known)?;
                }
                ArgumentValue::Word(text)
                    if known
                        .observations
                        .iter()
                        .any(|bound| bound.name.0 == text.0) =>
                {
                    ()
                }
                ArgumentValue::Word(text) if !known.linked => {
                    check_step(QuerySpelling(&text.0), call.span, known)?;
                }
                ArgumentValue::Word(text) => {
                    return Err(ValidationError::UnknownStep {
                        name: text.0.to_string(),
                        span: call.span,
                    });
                }
                ArgumentValue::Step(step) => {
                    if !(step.1.is_none()
                        && known
                            .observations
                            .iter()
                            .any(|bound| bound.name.0 == step.0))
                        && !known
                            .steps
                            .iter()
                            .any(|known| known.1 == step.1 && step.1.is_some())
                    {
                        return Err(ValidationError::UnknownStep {
                            name: step.0.to_string(),
                            span: call.span,
                        });
                    }
                }
                ArgumentValue::Number(_)
                | ArgumentValue::Parameter(_)
                | ArgumentValue::List(_)
                | ArgumentValue::Map(_) => {}
            }
        }
    }
    Ok(())
}

/// Every parameter an argument reads, however deep in a list or a map, was declared.
fn check_value(
    value: &ArgumentValue,
    parameters: &[ParameterDeclaration],
    call: &Call,
) -> Result<(), ValidationError> {
    match value {
        ArgumentValue::Parameter(name) => {
            check_parameter(QuerySpelling(&name.0), call.span, parameters)
        }
        ArgumentValue::List(values) => values
            .iter()
            .try_for_each(|inner| check_value(inner, parameters, call)),
        ArgumentValue::Map(arguments) => arguments
            .iter()
            .try_for_each(|argument| check_value(&argument.value, parameters, call)),
        ArgumentValue::Text(_)
        | ArgumentValue::Number(_)
        | ArgumentValue::Word(_)
        | ArgumentValue::Step(_) => Ok(()),
    }
}

/// That the parameter `name` is one the program declares.
fn check_parameter(
    name: QuerySpelling<'_>,
    span: QuerySpan,
    parameters: &[ParameterDeclaration],
) -> Result<(), ValidationError> {
    if parameters
        .iter()
        .any(|parameter| *parameter.name.0 == *name.0)
    {
        return Ok(());
    }
    Err(ValidationError::UnknownParameter {
        name: name.0.to_owned(),
        span,
    })
}

/// That each parameter the two operands of `filter` read is declared.
fn check_filter(
    filter: &Filter,
    parameters: &[ParameterDeclaration],
) -> Result<(), ValidationError> {
    for operand in [&filter.left, &filter.right] {
        check_operand(operand, filter.span, parameters)?;
    }
    Ok(())
}

/// That each parameter an operand reads, itself or through the arguments of its function, is declared.
fn check_operand(
    operand: &WrittenOperand,
    span: QuerySpan,
    parameters: &[ParameterDeclaration],
) -> Result<(), ValidationError> {
    match operand {
        WrittenOperand::Parameter(name) => {
            check_parameter(QuerySpelling(&name.0), span, parameters)
        }
        WrittenOperand::Function(applied) => {
            for argument in &applied.arguments {
                check_operand(argument, span, parameters)?;
            }
            Ok(())
        }
        WrittenOperand::Column(_) | WrittenOperand::Text(_) | WrittenOperand::Number(_) => Ok(()),
    }
}
