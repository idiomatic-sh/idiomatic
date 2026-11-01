//! The `selecting` service: a `select` lowered to the steps that make it: a join, a filter and a
//! projection, each only when the query wrote it.

use crate::domain::comparing::service::comparison_of;
use crate::domain::joining::service::{select_join, shown_of};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{
    Addressable, Builder, Joined, LastStep, PlannedColumn, ShownColumns, Sourced,
};
use crate::domain::planning::service::{push, pushed};
use crate::domain::resolving::service::{column_of, position, read_source, repeated_rows};
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::program::types::{self as syntax, QuerySpelling};

/// The alternatives of a `where`, each the tests that all hold for it: one when there is no `or`, and an `or` inside an
/// alternative multiplies it into one alternative for each of its own.
fn alternatives_of(conditions: &[syntax::Condition]) -> Vec<Vec<&syntax::Filter>> {
    let mut alternatives: Vec<Vec<&syntax::Filter>> = vec![Vec::new()];
    for condition in conditions {
        match condition {
            syntax::Condition::Test(filter) => {
                for alternative in &mut alternatives {
                    alternative.push(filter);
                }
            }
            syntax::Condition::Any(choices) => {
                let mut multiplied = Vec::new();
                for alternative in &alternatives {
                    for choice in choices {
                        for inner in alternatives_of(choice) {
                            let mut both = alternative.clone();
                            both.extend(inner);
                            multiplied.push(both);
                        }
                    }
                }
                alternatives = multiplied;
            }
        }
    }
    alternatives
}

/// Puts in `kept` the columns a projection keeps, each named as the query renamed it, and in `indexes` where each sits among `fields`.
fn projected(
    fields: &[PlannedColumn],
    columns: &[syntax::SelectColumn],
    span: syntax::QuerySpan,
    kept: &mut Vec<PlannedColumn>,
    indexes: &mut Vec<parts::ColumnIndex>,
) -> Result<(), LoweringError> {
    for column in columns {
        let index = position(fields, QuerySpelling(&column.column.column.0), span)?;
        indexes.push(index);
        kept.push(PlannedColumn {
            name: match &column.rename {
                syntax::Renaming::Renamed(renamed) => renamed.clone(),
                syntax::Renaming::Unchanged => column.column.column.clone(),
            },
            kind: column_of(fields, index).kind,
            unique: column_of(fields, index).unique,
            words: column_of(fields, index).words,
        });
    }
    Ok(())
}

/// Lowers a `select` statement: its source, its join, its filters and its columns, into the steps that make the rows of
/// `name`: a join, a filter and a projection, each only when the query wrote it. A `where` with `or` filters once for each
/// alternative and the union of those keeps each row once; the columns a join gives only for its `where` are dropped after it.
pub(crate) fn lower_select(
    builder: &mut Builder<'_>,
    name: &syntax::StepName,
    select: &syntax::Select,
) -> Result<(), LoweringError> {
    let Sourced {
        mut input,
        mut fields,
    } = read_source(builder, &select.from)?;
    let projects = matches!(select.join, syntax::Joining::Alone)
        && matches!(select.columns, syntax::Columns::Listed(_));
    let filtered = !select.filters.is_empty();
    let last = if !filtered && !projects {
        LastStep::Join
    } else {
        LastStep::Later
    };
    let (mut sides, mut shown) = (Vec::new(), ShownColumns(fields.len()));
    if let syntax::Joining::Joined(join) = &select.join {
        shown = shown_of(&select.columns);
        Joined {
            sourced: Sourced { input, fields },
            sides,
        } = select_join(builder, name, select, join, Sourced { input, fields }, last)?;
    }
    // The columns only the `where` reads are dropped after it, so the filter is not the last step.
    let select_is_last = !projects && shown.0 == fields.len();
    let mut alternatives = comparisons_of(
        builder,
        &select.filters,
        Addressable {
            fields: &fields,
            sides: &sides,
        },
    )?;
    let mut kept = Vec::new();
    let mut indexes = Vec::new();
    if let (true, syntax::Columns::Listed(columns)) = (projects, &select.columns) {
        projected(&fields, columns, builder.span, &mut kept, &mut indexes)?;
    }
    let distinct = repeated_rows(&kept);
    if projects && distinct == parts::RepeatedRows::Kept && alternatives.len() == 1 {
        // The columns kept say which row each is, so no row repeats: the one step filters and keeps them.
        push(
            builder,
            Some(name),
            plans::Operation::Select {
                from: input,
                comparisons: alternatives.pop().unwrap_or_default(),
                kept: indexes,
            },
            kept,
        );
        return Ok(());
    }
    if filtered {
        input = filtered_by(
            builder,
            select_is_last.then_some(name),
            input,
            alternatives,
            &fields,
        );
    }
    if shown.0 < fields.len() {
        fields.truncate(shown.0);
        unshown(builder, name, input, fields);
        return Ok(());
    }
    if projects {
        push(
            builder,
            Some(name),
            plans::Operation::Project {
                from: input,
                columns: indexes,
                distinct,
            },
            kept,
        );
        return Ok(());
    }
    if !filtered && matches!(select.join, syntax::Joining::Alone) {
        push(
            builder,
            Some(name),
            plans::Operation::Select {
                from: input,
                comparisons: Vec::new(),
                kept: Vec::new(),
            },
            fields,
        );
    }
    Ok(())
}

/// The rows of `input` that one of the alternatives holds for: a filter for each, and the union of them after the first,
/// which keeps each row once. The last step is given `name`, when it has one.
fn filtered_by(
    builder: &mut Builder<'_>,
    name: Option<&syntax::StepName>,
    input: parts::Input,
    alternatives: Vec<Vec<parts::Comparison>>,
    fields: &[PlannedColumn],
) -> parts::Input {
    let count = alternatives.len();
    let mut united = input;
    for (place, comparisons) in alternatives.into_iter().enumerate() {
        let last = place + 1 == count;
        let alternative = parts::Input::Step(pushed(
            builder,
            name.filter(|_| last && count == 1),
            plans::Operation::Select {
                from: input,
                comparisons,
                kept: Vec::new(),
            },
            fields.to_vec(),
        ));
        united = if place == 0 {
            alternative
        } else {
            parts::Input::Step(pushed(
                builder,
                name.filter(|_| last),
                plans::Operation::Union {
                    left: united,
                    right: alternative,
                },
                fields.to_vec(),
            ))
        };
    }
    united
}

/// The comparisons of each alternative of a `where`, resolved over `columns`.
fn comparisons_of(
    builder: &mut Builder<'_>,
    conditions: &[syntax::Condition],
    columns: Addressable<'_>,
) -> Result<Vec<Vec<parts::Comparison>>, LoweringError> {
    let mut alternatives = Vec::new();
    for tests in alternatives_of(conditions) {
        let mut comparisons = Vec::new();
        for filter in tests {
            comparisons.push(comparison_of(builder, filter, columns)?);
        }
        alternatives.push(comparisons);
    }
    Ok(alternatives)
}

/// The rows of `input` cut to `shown`, its first columns: the columns a join gave only for its `where` are dropped, and
/// a row they alone told apart is kept once. The step is given `name`.
fn unshown(
    builder: &mut Builder<'_>,
    name: &syntax::StepName,
    input: parts::Input,
    shown: Vec<PlannedColumn>,
) {
    let columns = (0..shown.len())
        .map(|place| parts::ColumnIndex(u32::try_from(place).unwrap_or(u32::MAX)))
        .collect();
    let distinct = repeated_rows(&shown);
    push(
        builder,
        Some(name),
        plans::Operation::Project {
            from: input,
            columns,
            distinct,
        },
        shown,
    );
}
