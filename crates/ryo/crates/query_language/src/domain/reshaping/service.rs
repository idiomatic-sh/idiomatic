//! The `reshaping` service: the calls that count, digest, bound, place, capture, split or rank rows, or write them, lowered to steps.

use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::terms::types::Vocabulary;

use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{
    ArgumentFault, Builder, PlannedColumn, SearchMatch, Sourced, Uniqueness,
};
use crate::domain::planning::service::push;
use crate::domain::resolving::service::{
    argument_named, bad_argument, column_of, find_parameter, input_argument, number_order,
    parameter_of, position, positions_of, word_of, words_of,
};
use crate::domain::syntax::constants::slot_name;
use crate::domain::syntax::types::ArgumentSlot;
use ryo_values::domain::plan_orders::types::GroupEnd;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::query_syntax::types::ArgumentValue;
use ryo_values::domain::query_tokens::types::QuerySpelling;
use ryo_values::domain::written_rows::types::{RowsSource, WrittenRow, WrittenRows};

/// Lowers a `group_count` call: one row for each group of the key columns with how many rows (or distinct values) it holds, as the step `name`.
pub(crate) fn call_group_count(
    builder: &mut Builder<'_>,
    name: &ryo_values::domain::query_tokens::types::StepName,
    call: &ryo_values::domain::query_syntax::types::Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: from,
        fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let keys = positions_of(builder, &fields, &words_of(call, ArgumentSlot::Keys)?)?;
    let distinct = if call
        .arguments
        .iter()
        .any(|argument| *argument.name.0 == *slot_name(ArgumentSlot::Distinct))
    {
        Some(position(
            &fields,
            QuerySpelling(&word_of(call, ArgumentSlot::Distinct)?.0),
            call.span,
        )?)
    } else {
        None
    };
    let mut kept: Vec<PlannedColumn> = keys
        .iter()
        .map(|index| column_of(&fields, *index).clone())
        .collect();
    kept.push(PlannedColumn {
        name: word_of(call, ArgumentSlot::Count)?,
        kind: CellKind::Number,
        unique: Uniqueness::Shared,
        words: Vocabulary::Open,
    });
    push(
        builder,
        Some(name),
        plans::Operation::GroupCount {
            from,
            keys,
            distinct,
        },
        kept,
    );
    Ok(())
}

/// Lowers a `group_digest` call: one row for each group of the key columns with a digest of the distinct values of one column, as the step `name`.
pub(crate) fn call_group_digest(
    builder: &mut Builder<'_>,
    name: &ryo_values::domain::query_tokens::types::StepName,
    call: &ryo_values::domain::query_syntax::types::Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: from,
        fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let keys = positions_of(builder, &fields, &words_of(call, ArgumentSlot::Keys)?)?;
    let over = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::Distinct)?.0),
        call.span,
    )?;
    let mut kept: Vec<PlannedColumn> = keys
        .iter()
        .map(|index| column_of(&fields, *index).clone())
        .collect();
    kept.push(PlannedColumn {
        name: word_of(call, ArgumentSlot::Digest)?,
        kind: CellKind::Number,
        unique: Uniqueness::Shared,
        words: Vocabulary::Open,
    });
    kept.push(PlannedColumn {
        name: word_of(call, ArgumentSlot::Count)?,
        kind: CellKind::Number,
        unique: Uniqueness::Shared,
        words: Vocabulary::Open,
    });
    push(
        builder,
        Some(name),
        plans::Operation::GroupDigest { from, keys, over },
        kept,
    );
    Ok(())
}

/// Lowers a `capture` call: its input's rows with the named captures of the pattern matched in the path column, as the step `name`.
pub(crate) fn call_capture(
    builder: &mut Builder<'_>,
    name: &ryo_values::domain::query_tokens::types::StepName,
    call: &ryo_values::domain::query_syntax::types::Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: from,
        mut fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let path = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::Path)?.0),
        call.span,
    )?;
    let pattern = parts::CapturePattern(word_of(call, ArgumentSlot::Pattern)?.0);
    for slot in words_of(call, ArgumentSlot::Captures)? {
        fields.push(PlannedColumn {
            name: slot,
            kind: CellKind::Symbol,
            unique: Uniqueness::Shared,
            words: Vocabulary::Open,
        });
    }
    push(
        builder,
        Some(name),
        plans::Operation::Capture {
            from,
            path,
            pattern,
        },
        fields,
    );
    Ok(())
}

/// Lowers a `rank` call: its input's rows with each one's rank within its group by the named column, as the step `name`.
pub(crate) fn call_rank(
    builder: &mut Builder<'_>,
    name: &ryo_values::domain::query_tokens::types::StepName,
    call: &ryo_values::domain::query_syntax::types::Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: from,
        mut fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let column = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::Column)?.0),
        call.span,
    )?;
    if !matches!(
        column_of(&fields, column).kind,
        CellKind::Symbol | CellKind::File
    ) {
        return Err(bad_argument(
            call,
            Some(ArgumentSlot::Column),
            ArgumentFault::ExpectedTextColumn,
        ));
    }
    let ArgumentValue::Parameter(declared) = argument_named(call, ArgumentSlot::Sets)? else {
        return Err(bad_argument(
            call,
            Some(ArgumentSlot::Sets),
            ArgumentFault::ExpectedTextSets,
        ));
    };
    let sets = find_parameter(builder.parameters, |each| {
        if each.parameter == *declared {
            SearchMatch::Matches
        } else {
            SearchMatch::Differs
        }
    })
    .ok_or_else(|| bad_argument(call, Some(ArgumentSlot::Sets), ArgumentFault::Undeclared))?;
    if *parameter_of(&builder.kinds, sets)
        != ryo_values::domain::query_bindings::types::ParameterType::TextSets
    {
        return Err(bad_argument(
            call,
            Some(ArgumentSlot::Sets),
            ArgumentFault::ExpectedTextSets,
        ));
    }
    fields.push(PlannedColumn {
        name: word_of(call, ArgumentSlot::Output)?,
        kind: CellKind::Number,
        unique: Uniqueness::Shared,
        words: Vocabulary::Open,
    });
    push(
        builder,
        Some(name),
        plans::Operation::Rank { from, column, sets },
        fields,
    );
    Ok(())
}

/// Lowers a `group-min` or `group-max` call: the rows of each group of the key columns at one end of a number order, every row
/// tied there, as the step `name`.
pub(crate) fn call_group_bound(
    builder: &mut Builder<'_>,
    name: &ryo_values::domain::query_tokens::types::StepName,
    call: &ryo_values::domain::query_syntax::types::Call,
    end: GroupEnd,
) -> Result<(), LoweringError> {
    let Sourced {
        input: from,
        fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let keys = positions_of(builder, &fields, &words_of(call, ArgumentSlot::Keys)?)?;
    let order = number_order(call, &fields, ArgumentSlot::Order)?;
    let operation = plans::Operation::GroupBound {
        from,
        keys,
        order,
        end,
    };
    push(builder, Some(name), operation, fields);
    Ok(())
}

/// Lowers a `group-ordinal` call: its input's distinct rows, each with its place in its group of the key columns by a number
/// order, under a column name the input does not have, as the step `name`.
pub(crate) fn call_group_ordinal(
    builder: &mut Builder<'_>,
    name: &ryo_values::domain::query_tokens::types::StepName,
    call: &ryo_values::domain::query_syntax::types::Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: from,
        mut fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let keys = positions_of(builder, &fields, &words_of(call, ArgumentSlot::Keys)?)?;
    let order = number_order(call, &fields, ArgumentSlot::Order)?;
    let place = word_of(call, ArgumentSlot::Output)?;
    if fields.iter().any(|field| field.name == place) {
        return Err(LoweringError::DuplicateOutput {
            name: place.0.to_string(),
            span: call.span,
        });
    }
    fields.push(PlannedColumn {
        name: place,
        kind: CellKind::Number,
        unique: Uniqueness::Shared,
        words: Vocabulary::Open,
    });
    let operation = plans::Operation::GroupOrdinal { from, keys, order };
    push(builder, Some(name), operation, fields);
    Ok(())
}

/// The rows a `rows` call writes, each a list of as many texts as the call names columns; a row of another width is refused.
fn written_rows(
    call: &ryo_values::domain::query_syntax::types::Call,
    rows: &[ArgumentValue],
    columns: &[ryo_values::domain::descriptions::types::ColumnName],
) -> Result<WrittenRows, LoweringError> {
    let width = columns.len();
    let expected = || bad_argument(call, Some(ArgumentSlot::From), ArgumentFault::ExpectedRows);
    let mut written = Vec::with_capacity(rows.len());
    for (at, row) in rows.iter().enumerate() {
        let ArgumentValue::List(cells) = row else {
            return Err(expected());
        };
        if cells.len() != width {
            return Err(LoweringError::WrittenRowWidth {
                row: at.saturating_add(1),
                expected: width,
                found: cells.len(),
                span: call.span,
            });
        }
        let mut texts = Vec::with_capacity(width);
        for cell in cells {
            let ArgumentValue::Text(text) = cell else {
                return Err(expected());
            };
            texts.push(parts::LiteralText(text.0.clone()));
        }
        written.push(WrittenRow(texts));
    }
    Ok(WrittenRows(written))
}

/// `rows(from=$tables, columns=["key", …])`: a row for each table of the parameter, a text column for each key. Or
/// `rows(from=[["a", "b"], …], columns=[…])`: the rows written in the query, a text column for each name.
pub(crate) fn call_rows(
    builder: &mut Builder<'_>,
    name: &ryo_values::domain::query_tokens::types::StepName,
    call: &ryo_values::domain::query_syntax::types::Call,
) -> Result<(), LoweringError> {
    let names = words_of(call, ArgumentSlot::Columns)?;
    let source = match argument_named(call, ArgumentSlot::From)? {
        ArgumentValue::List(rows) => RowsSource::Written(written_rows(call, rows, &names)?),
        ArgumentValue::Parameter(declared) => {
            let parameter = find_parameter(builder.parameters, |each| {
                if each.parameter == *declared {
                    SearchMatch::Matches
                } else {
                    SearchMatch::Differs
                }
            })
            .ok_or_else(|| {
                bad_argument(call, Some(ArgumentSlot::From), ArgumentFault::Undeclared)
            })?;
            if *parameter_of(&builder.kinds, parameter)
                != ryo_values::domain::query_bindings::types::ParameterType::Tables
            {
                return Err(bad_argument(
                    call,
                    Some(ArgumentSlot::From),
                    ArgumentFault::ExpectedTables,
                ));
            }
            RowsSource::Parameter(parameter)
        }
        ArgumentValue::Text(_)
        | ArgumentValue::Number(_)
        | ArgumentValue::Word(_)
        | ArgumentValue::Step(_)
        | ArgumentValue::Map(_) => {
            return Err(bad_argument(
                call,
                Some(ArgumentSlot::From),
                ArgumentFault::ExpectedTables,
            ));
        }
    };
    let keys = names
        .iter()
        .map(|each| parts::TableKey(each.0.clone()))
        .collect();
    let fields = names
        .into_iter()
        .map(|each| PlannedColumn {
            name: each,
            kind: CellKind::Symbol,
            unique: Uniqueness::Shared,
            words: Vocabulary::Open,
        })
        .collect();
    push(
        builder,
        Some(name),
        plans::Operation::Rows { source, keys },
        fields,
    );
    Ok(())
}

/// Lowers a `words` call: one row for each word of the text column of each input row, as the step `name`.
pub(crate) fn call_words(
    builder: &mut Builder<'_>,
    name: &ryo_values::domain::query_tokens::types::StepName,
    call: &ryo_values::domain::query_syntax::types::Call,
) -> Result<(), LoweringError> {
    let Sourced {
        input: from,
        mut fields,
    } = input_argument(builder, call, ArgumentSlot::From)?;
    let text = position(
        &fields,
        QuerySpelling(&word_of(call, ArgumentSlot::Text)?.0),
        call.span,
    )?;
    fields.push(PlannedColumn {
        name: word_of(call, ArgumentSlot::Word)?,
        kind: CellKind::Symbol,
        unique: Uniqueness::Shared,
        words: Vocabulary::Open,
    });
    push(
        builder,
        Some(name),
        plans::Operation::Words { from, text },
        fields,
    );
    Ok(())
}
