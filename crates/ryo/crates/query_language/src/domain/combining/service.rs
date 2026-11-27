//! The `combining` service: the set calls (`union`, `intersection` and `difference`) lowered to steps. `intersection` and
//! `difference` are written as the projection and the antijoins that say them: the plan holds no step of their own for them.

use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{ArgumentFault, Builder, PlannedColumn, SearchMatch, Sourced};
use crate::domain::planning::service::{push, pushed};
use crate::domain::resolving::service::{bad_argument, column_of, find_column, input_argument};
use crate::domain::syntax::types::ArgumentSlot;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::query_operators::types::QueryOperator;
use ryo_values::domain::query_syntax::types as ast;
use ryo_values::domain::query_tokens::types as tokens;

/// Lowers a set call (`union`, `intersection` or `difference`) of two inputs with the same columns, as the step `name`.
pub(crate) fn call_set(
    builder: &mut Builder<'_>,
    name: &tokens::StepName,
    call: &ast::Call,
    operator: QueryOperator,
) -> Result<(), LoweringError> {
    let Sourced {
        input: left,
        fields,
    } = input_argument(builder, call, ArgumentSlot::Left)?;
    let Sourced {
        input: right,
        fields: right_fields,
    } = input_argument(builder, call, ArgumentSlot::Right)?;
    if fields.len() != right_fields.len() {
        return Err(bad_argument(
            call,
            Some(ArgumentSlot::Right),
            ArgumentFault::SameColumns,
        ));
    }
    // The columns of the two are matched by their names: the right one is put in the order of the left.
    let right = if right_fields
        .iter()
        .zip(&fields)
        .all(|(each, wanted)| each.name == wanted.name)
    {
        right
    } else {
        let order: Option<Vec<parts::ColumnIndex>> = fields
            .iter()
            .map(|wanted| {
                find_column(&right_fields, |each| {
                    if each.name == wanted.name {
                        SearchMatch::Matches
                    } else {
                        SearchMatch::Differs
                    }
                })
            })
            .collect();
        let Some(order) = order else {
            return Err(bad_argument(
                call,
                Some(ArgumentSlot::Right),
                ArgumentFault::SameColumns,
            ));
        };
        let aligned: Vec<PlannedColumn> = order
            .iter()
            .map(|at| column_of(&right_fields, *at).clone())
            .collect();
        let id = pushed(
            builder,
            None,
            plans::Operation::Project {
                from: right,
                columns: order,
                distinct: parts::RepeatedRows::Kept,
            },
            aligned,
        );
        parts::Input::Step(id)
    };
    let operation = match operator {
        QueryOperator::Union => plans::Operation::Union { left, right },
        // `intersection` and `difference` compare whole rows and answer each row once: the distinct left rows, less the ones
        // the right input holds on every column (a difference); and those rows less that difference again (an intersection).
        QueryOperator::Intersection | QueryOperator::Difference => {
            let every: Vec<parts::ColumnIndex> = (0..fields.len())
                .map(|place| parts::ColumnIndex(u32::try_from(place).unwrap_or(u32::MAX)))
                .collect();
            let whole_rows = |left, right| plans::Operation::Antijoin {
                left,
                right,
                left_on: every.clone(),
                right_on: every.clone(),
            };
            let distinct = plans::Operation::Project {
                from: left,
                columns: every.clone(),
                distinct: parts::RepeatedRows::Dropped,
            };
            let distinct = parts::Input::Step(pushed(builder, None, distinct, fields.clone()));
            let lacking = whole_rows(distinct, right);
            if operator == QueryOperator::Difference {
                lacking
            } else {
                let lacking = parts::Input::Step(pushed(builder, None, lacking, fields.clone()));
                whole_rows(distinct, lacking)
            }
        }
        QueryOperator::Select
        | QueryOperator::Project
        | QueryOperator::Extend
        | QueryOperator::Join
        | QueryOperator::Antijoin
        | QueryOperator::GroupCount
        | QueryOperator::Replace
        | QueryOperator::SpanBetween
        | QueryOperator::SpanBounds
        | QueryOperator::SourceMatches
        | QueryOperator::RenderSyntax
        | QueryOperator::FitsRendered
        | QueryOperator::OrderedMatch
        | QueryOperator::AsOf
        | QueryOperator::GroupMin
        | QueryOperator::GroupMax
        | QueryOperator::GroupOrdinal
        | QueryOperator::Follow
        | QueryOperator::OrderedDfs
        | QueryOperator::Worklist
        | QueryOperator::GroupDigest
        | QueryOperator::Capture
        | QueryOperator::Words
        | QueryOperator::Rank
        | QueryOperator::Axis
        | QueryOperator::Rows
        | QueryOperator::Closure
        | QueryOperator::Order
        | QueryOperator::Prefixes
        | QueryOperator::SubtreeClass
        | QueryOperator::Cycles => {
            return Err(bad_argument(call, None, ArgumentFault::NoSuchOperation));
        }
    };
    push(builder, Some(name), operation, fields);
    Ok(())
}
