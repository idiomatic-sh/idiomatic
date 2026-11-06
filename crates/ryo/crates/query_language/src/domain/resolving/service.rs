//! The `resolving` service: where a name a query writes becomes what it names, and where the
//! arguments of a call are read, for the lowering of a program.

use ryo_relations::domain::relations::service::{column_name, columns, kind, relation_named};
use ryo_relations::domain::relations::service::{is_key, vocabulary_of};
use ryo_relations::domain::relations::types::NameRef;
use ryo_relations::domain::relations::types::{ColumnKeyness, Relation};
use ryo_relations::domain::terms::types::Vocabulary;

use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{
    ArgumentFault, Builder, PlannedColumn, SearchMatch, Sourced, StepLabel, Uniqueness,
};
use crate::domain::syntax::constants::slot_name;
use crate::domain::syntax::types::ArgumentSlot;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::program::types as syntax;
use ryo_values::domain::query_tokens::types::QuerySpelling;

/// Where the column called `name` sits among `fields`.
pub(crate) fn position(
    fields: &[PlannedColumn],
    name: QuerySpelling<'_>,
    span: ryo_values::domain::query_tokens::types::QuerySpan,
) -> Result<parts::ColumnIndex, LoweringError> {
    find_column(fields, |field| {
        if *field.name.0 == *name.0 {
            SearchMatch::Matches
        } else {
            SearchMatch::Differs
        }
    })
    .ok_or_else(|| LoweringError::UnknownColumn {
        name: name.0.to_owned(),
        span,
    })
}

/// The columns of a relation, as a step would hold them.
pub(crate) fn relation_fields(relation: Relation) -> Vec<PlannedColumn> {
    columns(relation)
        .into_iter()
        .map(|column| PlannedColumn {
            name: ryo_values::domain::descriptions::types::ColumnName(Box::from(
                column_name(column).0,
            )),
            kind: kind(column),
            unique: if is_key(column) == ColumnKeyness::Key {
                Uniqueness::Unique
            } else {
                Uniqueness::Shared
            },
            words: vocabulary_of(column),
        })
        .collect()
}

/// Whether a projection of these fields drops repeated rows: not when a column kept says which row it is.
pub(crate) fn repeated_rows(kept: &[PlannedColumn]) -> parts::RepeatedRows {
    if kept.iter().any(|field| field.unique == Uniqueness::Unique) {
        parts::RepeatedRows::Kept
    } else {
        parts::RepeatedRows::Dropped
    }
}

/// The field of `items` that the column `index` names.
#[expect(
    clippy::indexing_slicing,
    reason = "the ids are the lowering's own: it numbers each column, step and parameter as it adds it to the list it is looked up in, so they are in bounds by construction"
)]
pub(crate) fn column_of<T>(items: &[T], index: parts::ColumnIndex) -> &T {
    &items[usize::try_from(index.0).unwrap_or(usize::MAX)]
}

/// The one of `items` that the step `id` names.
#[expect(
    clippy::indexing_slicing,
    reason = "the ids are the lowering's own: it numbers each column, step and parameter as it adds it to the list it is looked up in, so they are in bounds by construction"
)]
pub(crate) fn step_of<T>(items: &[T], id: parts::StepId) -> &T {
    &items[usize::try_from(id.0).unwrap_or(usize::MAX)]
}

/// The one of `items` that the parameter `id` names.
#[expect(
    clippy::indexing_slicing,
    reason = "the ids are the lowering's own: it numbers each column, step and parameter as it adds it to the list it is looked up in, so they are in bounds by construction"
)]
pub(crate) fn parameter_of<T>(items: &[T], id: parts::ParameterId) -> &T {
    &items[usize::try_from(id.0).unwrap_or(usize::MAX)]
}

/// The column of the first of `items` that `holds` for.
pub(crate) fn find_column<T>(
    items: &[T],
    holds: impl Fn(&T) -> SearchMatch,
) -> Option<parts::ColumnIndex> {
    items
        .iter()
        .position(|item| holds(item) == SearchMatch::Matches)
        .map(|place| parts::ColumnIndex(u32::try_from(place).unwrap_or(u32::MAX)))
}

/// The step of the first of `items` that `holds` for.
pub(crate) fn find_step<T>(
    items: &[T],
    holds: impl Fn(&T) -> SearchMatch,
) -> Option<parts::StepId> {
    items
        .iter()
        .position(|item| holds(item) == SearchMatch::Matches)
        .map(|place| parts::StepId(u32::try_from(place).unwrap_or(u32::MAX)))
}

/// The parameter of the first of `items` that `holds` for.
pub(crate) fn find_parameter<T>(
    items: &[T],
    holds: impl Fn(&T) -> SearchMatch,
) -> Option<parts::ParameterId> {
    items
        .iter()
        .position(|item| holds(item) == SearchMatch::Matches)
        .map(|place| parts::ParameterId(u32::try_from(place).unwrap_or(u32::MAX)))
}

/// The step the `let` called `name` made.
pub(crate) fn step_named(
    builder: &Builder<'_>,
    name: &ryo_values::domain::query_tokens::types::StepName,
) -> Result<parts::StepId, LoweringError> {
    find_step(&builder.lowered, |lowered| {
        if matches!(&lowered.name, StepLabel::Named(named) if match (ryo_values::domain::program::service::identity_of(named.1), ryo_values::domain::program::service::identity_of(name.1)) {
            (Some(left), Some(right)) => left == right,
            _ => named.0 == name.0,
        }) {
            SearchMatch::Matches
        } else {
            SearchMatch::Differs
        }
    })
    .ok_or_else(|| LoweringError::UnknownStep {
        name: name.0.to_string(),
        span: builder.span,
    })
}

/// What a source reads, and the columns it has.
pub(crate) fn read_source(
    builder: &Builder<'_>,
    source: &ryo_values::domain::query_syntax::types::QuerySource,
) -> Result<Sourced, LoweringError> {
    let external = match &source.reference {
        ryo_values::domain::query_syntax::types::SourceReference::Relation(text) => {
            Some(text.0.as_ref())
        }
        ryo_values::domain::query_syntax::types::SourceReference::Step(text)
            if ryo_values::domain::program::service::identity_of(text.1).is_none() =>
        {
            Some(text.0.as_ref())
        }
        ryo_values::domain::query_syntax::types::SourceReference::Step(_) => None,
    };
    if let Some((index, bound)) = builder
        .observation_inputs
        .iter()
        .enumerate()
        .find(|(_, bound)| Some(bound.name.0.as_ref()) == external)
    {
        let index = u32::try_from(index).map_err(|_| LoweringError::UnknownRelation {
            name: bound.name.0.to_string(),
            span: source.span,
        })?;
        return Ok(Sourced {
            input: parts::Input::BoundObservation(
                ryo_values::domain::plan_parts::types::ObservationBindingId(index),
            ),
            fields: bound
                .columns
                .iter()
                .map(|column| PlannedColumn {
                    name: ryo_values::domain::descriptions::types::ColumnName(
                        column.name.0.clone(),
                    ),
                    kind: column.kind,
                    unique: Uniqueness::Shared,
                    words: Vocabulary::Open,
                })
                .collect(),
        });
    }

    match &source.reference {
        ryo_values::domain::query_syntax::types::SourceReference::Relation(text) => {
            if let Some(hosted) = builder
                .hosted_steps
                .iter()
                .find(|hosted| *hosted.name.0 == *text.0)
            {
                return Ok(Sourced {
                    input: parts::Input::Step(hosted.step),
                    fields: step_of(&builder.lowered, hosted.step).fields.clone(),
                });
            }
            let relation = relation_named(NameRef(&text.0))
                .filter(|relation| builder.catalog.contains(relation))
                .ok_or_else(|| LoweringError::UnknownRelation {
                    name: text.0.to_string(),
                    span: source.span,
                })?;
            Ok(Sourced {
                input: parts::Input::Relation(relation),
                fields: relation_fields(relation),
            })
        }
        ryo_values::domain::query_syntax::types::SourceReference::Step(text) => {
            let id = step_named(builder, text)?;
            Ok(Sourced {
                input: parts::Input::Step(id),
                fields: step_of(&builder.lowered, id).fields.clone(),
            })
        }
    }
}

/// What an argument that names an input reads, and the columns it has.
pub(crate) fn input_argument(
    builder: &Builder<'_>,
    call: &ryo_values::domain::query_syntax::types::Call,
    argument: ArgumentSlot,
) -> Result<Sourced, LoweringError> {
    let value = argument_named(call, argument)?;
    let reference = match value {
        ryo_values::domain::query_syntax::types::ArgumentValue::Text(name) => {
            ryo_values::domain::query_syntax::types::SourceReference::Relation(
                ryo_values::domain::query_tokens::types::WrittenRelation(name.0.clone()),
            )
        }
        ryo_values::domain::query_syntax::types::ArgumentValue::Word(name) => {
            ryo_values::domain::query_syntax::types::SourceReference::Step(
                ryo_values::domain::query_tokens::types::StepName(
                    name.0.clone().into(),
                    ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
                ),
            )
        }
        ryo_values::domain::query_syntax::types::ArgumentValue::Step(step) => {
            ryo_values::domain::query_syntax::types::SourceReference::Step(step.clone())
        }
        ryo_values::domain::query_syntax::types::ArgumentValue::Number(_)
        | ryo_values::domain::query_syntax::types::ArgumentValue::Parameter(_)
        | ryo_values::domain::query_syntax::types::ArgumentValue::List(_)
        | ryo_values::domain::query_syntax::types::ArgumentValue::Map(_) => {
            return Err(bad_argument(
                call,
                Some(argument),
                ArgumentFault::ExpectedInput,
            ));
        }
    };
    read_source(
        builder,
        &ryo_values::domain::query_syntax::types::QuerySource {
            reference,
            alias: ryo_values::domain::query_syntax::types::Alias::Unwritten,
            span: call.span,
        },
    )
}

/// The error that `call` was given its `argument` wrongly, for `reason`, at the call.
pub(crate) fn bad_argument(
    call: &ryo_values::domain::query_syntax::types::Call,
    argument: Option<ArgumentSlot>,
    reason: ArgumentFault,
) -> LoweringError {
    LoweringError::BadArgument {
        operation: call.name.0.to_string(),
        argument: argument.map(slot_name).unwrap_or_default().to_owned(),
        reason,
        span: call.span,
    }
}

/// The value of the argument of `call` in `slot`; refused when the call does not give it.
pub(crate) fn argument_named(
    call: &ryo_values::domain::query_syntax::types::Call,
    slot: ArgumentSlot,
) -> Result<&ryo_values::domain::query_syntax::types::ArgumentValue, LoweringError> {
    call.arguments
        .iter()
        .find(|argument| *argument.name.0 == *slot_name(slot))
        .map(|argument| &argument.value)
        .ok_or_else(|| bad_argument(call, Some(slot), ArgumentFault::Missing))
}

/// The column name the argument of `call` in `name` holds, written as a word or a text; refused for any other value.
pub(crate) fn word_of(
    call: &ryo_values::domain::query_syntax::types::Call,
    name: ArgumentSlot,
) -> Result<ryo_values::domain::descriptions::types::ColumnName, LoweringError> {
    match argument_named(call, name)? {
        ryo_values::domain::query_syntax::types::ArgumentValue::Text(text) => Ok(
            ryo_values::domain::descriptions::types::ColumnName(text.0.clone()),
        ),
        ryo_values::domain::query_syntax::types::ArgumentValue::Word(word) => Ok(
            ryo_values::domain::descriptions::types::ColumnName(word.0.clone()),
        ),
        ryo_values::domain::query_syntax::types::ArgumentValue::Number(_)
        | ryo_values::domain::query_syntax::types::ArgumentValue::Parameter(_)
        | ryo_values::domain::query_syntax::types::ArgumentValue::Step(_)
        | ryo_values::domain::query_syntax::types::ArgumentValue::List(_)
        | ryo_values::domain::query_syntax::types::ArgumentValue::Map(_) => {
            Err(bad_argument(call, Some(name), ArgumentFault::ExpectedName))
        }
    }
}

/// The column names the list argument of `call` in `name` holds; refused when it is not a list of words or texts.
pub(crate) fn words_of(
    call: &ryo_values::domain::query_syntax::types::Call,
    name: ArgumentSlot,
) -> Result<Vec<ryo_values::domain::descriptions::types::ColumnName>, LoweringError> {
    let ryo_values::domain::query_syntax::types::ArgumentValue::List(values) =
        argument_named(call, name)?
    else {
        return Err(bad_argument(call, Some(name), ArgumentFault::ExpectedNames));
    };
    values
        .iter()
        .map(|value| match value {
            ryo_values::domain::query_syntax::types::ArgumentValue::Text(text) => Ok(
                ryo_values::domain::descriptions::types::ColumnName(text.0.clone()),
            ),
            ryo_values::domain::query_syntax::types::ArgumentValue::Word(word) => Ok(
                ryo_values::domain::descriptions::types::ColumnName(word.0.clone()),
            ),
            ryo_values::domain::query_syntax::types::ArgumentValue::Number(_)
            | ryo_values::domain::query_syntax::types::ArgumentValue::Parameter(_)
            | ryo_values::domain::query_syntax::types::ArgumentValue::Step(_)
            | ryo_values::domain::query_syntax::types::ArgumentValue::List(_)
            | ryo_values::domain::query_syntax::types::ArgumentValue::Map(_) => {
                Err(bad_argument(call, Some(name), ArgumentFault::ExpectedNames))
            }
        })
        .collect()
}

/// The places among `fields` of the columns `names` names, in order; refused at the step for a name no field has.
pub(crate) fn positions_of(
    builder: &Builder<'_>,
    fields: &[PlannedColumn],
    names: &[ryo_values::domain::descriptions::types::ColumnName],
) -> Result<Vec<parts::ColumnIndex>, LoweringError> {
    names
        .iter()
        .map(|name| position(fields, QuerySpelling(&name.0), builder.span))
        .collect()
}
