//! The `calling` service: the calls that shape rows (`project`, `extend`, `join` and `antijoin`) and the dispatch of every call to its lowering.

use crate::domain::combining::service::call_set;
use crate::domain::idioms::service::push_same_sets;
use crate::domain::joining::service::join_output;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::ArgumentFault;
use crate::domain::lowering::types::Builder;
use crate::domain::lowering::types::PlannedColumn;
use crate::domain::lowering::types::SearchMatch;
use crate::domain::lowering::types::Sourced;
use crate::domain::lowering::types::Uniqueness;
use crate::domain::lowering::types::Widened;
use crate::domain::planning::service::push;
use crate::domain::reshaping::service::call_capture;
use crate::domain::reshaping::service::call_group_count;
use crate::domain::reshaping::service::call_group_digest;
use crate::domain::reshaping::service::call_rank;
use crate::domain::reshaping::service::call_rows;
use crate::domain::reshaping::service::call_words;
use crate::domain::resolving::service::argument_named;
use crate::domain::resolving::service::bad_argument;
use crate::domain::resolving::service::column_of;
use crate::domain::resolving::service::find_parameter;
use crate::domain::resolving::service::input_argument;
use crate::domain::resolving::service::parameter_of;
use crate::domain::resolving::service::position;
use crate::domain::resolving::service::positions_of;
use crate::domain::resolving::service::repeated_rows;
use crate::domain::resolving::service::words_of;
use crate::domain::rewriting::service::same_sets;
use crate::domain::syntax::constants::operator_named;
use crate::domain::syntax::types::ArgumentSlot;
use crate::domain::traversing::service::call_axis;
use crate::domain::traversing::service::call_closure;
use crate::domain::traversing::service::call_ordered_dfs;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::terms::types::Vocabulary;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::program::types as syntax;
use ryo_values::domain::query_operators::types::QueryOperator;
use ryo_values::domain::query_syntax::types as ast;
use ryo_values::domain::query_tokens::types as tokens;
use ryo_values::domain::query_tokens::types::QuerySpelling;

/// `name(argument=value, ...)`: one step of the operation the call names.
pub(crate) fn lower_call(
    builder: &mut Builder<'_>,
    name: &tokens::StepName,
    call: &ast::Call,
) -> Result<(), LoweringError> {
    let Some(operator) = operator_named(&call.name.0) else {
        return Err(bad_argument(call, None, ArgumentFault::NoSuchOperation));
    };
    match operator {
        QueryOperator::Select => return Err(bad_argument(call, None, ArgumentFault::NotACall)),
        QueryOperator::Project => call_project(builder, name, call)?,
        QueryOperator::Extend => call_extend(builder, name, call)?,
        QueryOperator::SpanBetween => {
            crate::domain::spanning::service::call_span_between(builder, name, call)?;
        }
        QueryOperator::SourceMatches => {
            crate::domain::substituting::service::call_source_matches(builder, name, call)?;
        }
        QueryOperator::RenderSyntax => {
            crate::domain::rendering::service::call_render_syntax(builder, name, call)?;
        }
        QueryOperator::FitsRendered => {
            crate::domain::rendering::service::call_fits_rendered(builder, name, call)?;
        }
        QueryOperator::SpanBounds => {
            crate::domain::spanning::service::call_span_bounds(builder, name, call)?;
        }
        QueryOperator::Join | QueryOperator::OrderedMatch => call_join(builder, name, call)?,
        QueryOperator::Replace => {
            crate::domain::substituting::service::call_replace(builder, name, call)?;
        }
        QueryOperator::Union | QueryOperator::Intersection | QueryOperator::Difference => {
            call_set(builder, name, call, operator)?;
        }
        QueryOperator::Antijoin => call_antijoin(builder, name, call)?,
        QueryOperator::GroupCount => call_group_count(builder, name, call)?,
        QueryOperator::GroupDigest => call_group_digest(builder, name, call)?,
        QueryOperator::Capture => call_capture(builder, name, call)?,
        QueryOperator::Words => call_words(builder, name, call)?,
        QueryOperator::Rank => call_rank(builder, name, call)?,
        QueryOperator::OrderedDfs => call_ordered_dfs(builder, name, call)?,
        QueryOperator::Axis => call_axis(builder, name, call)?,
        QueryOperator::Rows => call_rows(builder, name, call)?,
        QueryOperator::Closure | QueryOperator::Cycles => {
            call_closure(builder, name, call, operator)?;
        }
    }
    Ok(())
}

/// Lowers a `project` call: its input's rows with only the named columns, in the order named, as the step `name`.
pub(crate) fn call_project(
    builder: &mut Builder<'_>,
    name: &tokens::StepName,
    call: &ast::Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: from,
        fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let names = words_of(call, ArgumentSlot::Columns)?;
    let indexes = positions_of(builder, &fields, &names)?;
    let kept: Vec<PlannedColumn> = indexes
        .iter()
        .map(|index| column_of(&fields, *index).clone())
        .collect();
    let distinct = repeated_rows(&kept);
    push(
        builder,
        Some(name),
        plans::Operation::Project {
            from,
            columns: indexes,
            distinct,
        },
        kept,
    );
    Ok(())
}

/// Lowers an `extend` call: its input's rows with the columns its `values` map adds, as the step `name`.
pub(crate) fn call_extend(
    builder: &mut Builder<'_>,
    name: &tokens::StepName,
    call: &ast::Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: from,
        mut fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let ast::ArgumentValue::Map(values) = argument_named(call, ArgumentSlot::Values)? else {
        return Err(bad_argument(
            call,
            Some(ArgumentSlot::Values),
            ArgumentFault::ExpectedMap,
        ));
    };
    let mut operands = Vec::new();
    for value in values {
        let (operand, kind) = match &value.value {
            ast::ArgumentValue::Number(number) => (
                parts::Operand::Literal(parts::QueryLiteral::Number(
                    ryo_relations::domain::relations::types::Integer(number.0),
                )),
                CellKind::Number,
            ),
            ast::ArgumentValue::Text(text) => (
                parts::Operand::Literal(parts::QueryLiteral::Text(parts::LiteralText(
                    text.0.clone(),
                ))),
                CellKind::Symbol,
            ),
            ast::ArgumentValue::Parameter(parameter) => {
                let id = find_parameter(builder.parameters, |declared| {
                    if declared.name == *parameter {
                        SearchMatch::Matches
                    } else {
                        SearchMatch::Differs
                    }
                })
                .ok_or_else(|| {
                    bad_argument(call, Some(ArgumentSlot::Values), ArgumentFault::Undeclared)
                })?;
                let kind = match *parameter_of(&builder.kinds, id) {
                    ryo_values::domain::query_bindings::types::ParameterType::Flag => {
                        CellKind::Flag
                    }
                    ryo_values::domain::query_bindings::types::ParameterType::Number => {
                        CellKind::Number
                    }
                    ryo_values::domain::query_bindings::types::ParameterType::Text => {
                        CellKind::Symbol
                    }
                    ryo_values::domain::query_bindings::types::ParameterType::Numbers
                    | ryo_values::domain::query_bindings::types::ParameterType::Texts
                    | ryo_values::domain::query_bindings::types::ParameterType::TextSets
                    | ryo_values::domain::query_bindings::types::ParameterType::Tables => {
                        return Err(bad_argument(
                            call,
                            Some(ArgumentSlot::Values),
                            ArgumentFault::NumbersAndFlagsOnly,
                        ));
                    }
                };
                (parts::Operand::Parameter(id), kind)
            }
            ast::ArgumentValue::Word(_)
            | ast::ArgumentValue::Step(_)
            | ast::ArgumentValue::List(_)
            | ast::ArgumentValue::Map(_) => {
                return Err(bad_argument(
                    call,
                    Some(ArgumentSlot::Values),
                    ArgumentFault::NumbersAndParametersOnly,
                ));
            }
        };
        operands.push(operand);
        fields.push(PlannedColumn {
            name: ryo_values::domain::descriptions::types::ColumnName(value.name.0.clone()),
            kind,
            unique: Uniqueness::Shared,
            words: Vocabulary::Open,
        });
    }
    push(
        builder,
        Some(name),
        plans::Operation::Extend {
            from,
            values: operands,
        },
        fields,
    );
    Ok(())
}

/// Lowers a `join` call: the rows of its left input matched to those of its right on the named columns, as the step `name`.
pub(crate) fn call_join(
    builder: &mut Builder<'_>,
    name: &tokens::StepName,
    call: &ast::Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: left,
        fields: left_fields,
    } = input_argument(builder, call, ArgumentSlot::Left)?;
    let Sourced {
        input: right,
        fields: right_fields,
    } = input_argument(builder, call, ArgumentSlot::Right)?;
    let left_on = positions_of(
        builder,
        &left_fields,
        &words_of(call, ArgumentSlot::LeftOn)?,
    )?;
    let right_on = positions_of(
        builder,
        &right_fields,
        &words_of(call, ArgumentSlot::RightOn)?,
    )?;
    let Widened {
        output,
        fields: kept,
    } = join_output(call, &left_fields, &right_fields)?;
    let operation = if operator_named(&call.name.0) == Some(QueryOperator::OrderedMatch) {
        for (index, field) in kept.iter().enumerate() {
            if kept
                .iter()
                .take(index)
                .any(|before| before.name == field.name)
            {
                return Err(LoweringError::DuplicateOutput {
                    name: field.name.0.to_string(),
                    span: call.span,
                });
            }
        }
        if left_on.len() != right_on.len()
            || left_on.iter().zip(&right_on).any(|(left_key, right_key)| {
                column_of(&left_fields, *left_key).kind != column_of(&right_fields, *right_key).kind
            })
        {
            return Err(LoweringError::KindMismatch { span: call.span });
        }
        let left_order = position(
            &left_fields,
            QuerySpelling(
                &crate::domain::resolving::service::word_of(call, ArgumentSlot::LeftOrder)?.0,
            ),
            call.span,
        )?;
        let right_order = position(
            &right_fields,
            QuerySpelling(
                &crate::domain::resolving::service::word_of(call, ArgumentSlot::RightOrder)?.0,
            ),
            call.span,
        )?;
        if column_of(&left_fields, left_order).kind != CellKind::Number
            || column_of(&right_fields, right_order).kind != CellKind::Number
        {
            return Err(LoweringError::KindMismatch { span: call.span });
        }
        plans::Operation::OrderedMatch {
            left,
            right,
            left_on,
            right_on,
            left_order,
            right_order,
            output,
        }
    } else {
        plans::Operation::Join {
            left,
            right,
            left_on,
            right_on,
            output,
            filter: Vec::new(),
            view: Vec::new(),

            reach: None,
        }
    };
    push(builder, Some(name), operation, kept);
    Ok(())
}

/// Lowers an `antijoin` call: the rows of its left input that no row of its right input matches on the named columns, as the step `name`.
pub(crate) fn call_antijoin(
    builder: &mut Builder<'_>,
    name: &tokens::StepName,
    call: &ast::Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: left,
        fields,
    } = input_argument(builder, call, ArgumentSlot::Left)?;
    let Sourced {
        input: right,
        fields: right_fields,
    } = input_argument(builder, call, ArgumentSlot::Right)?;
    let left_on = positions_of(builder, &fields, &words_of(call, ArgumentSlot::LeftOn)?)?;
    let right_on = positions_of(
        builder,
        &right_fields,
        &words_of(call, ArgumentSlot::RightOn)?,
    )?;
    if left_on.is_empty() || left_on.len() != right_on.len() {
        return Err(bad_argument(
            call,
            Some(ArgumentSlot::LeftOn),
            ArgumentFault::MatchingKeys,
        ));
    }
    if let Some(idiom) = same_sets(&builder.steps, left, right, &left_on, &right_on) {
        push_same_sets(builder, name, idiom);
        return Ok(());
    }
    push(
        builder,
        Some(name),
        plans::Operation::Antijoin {
            left,
            right,
            left_on,
            right_on,
        },
        fields,
    );
    Ok(())
}
