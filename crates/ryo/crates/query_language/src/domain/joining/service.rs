//! The `joining` service: the join a `select` writes, lowered to the step that makes it, and the columns of either side
//! its `where` names as `alias.column`.

use super::constants::SIDE_SEPARATOR;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{
    Builder, Joined, LastStep, PlannedColumn, SearchMatch, ShownColumns, SideColumn, Sourced,
    Uniqueness,
};
use crate::domain::planning::service::pushed;
use crate::domain::resolving::service::{column_of, find_column, position, read_source};
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::program::types::{self as syntax, QuerySpelling};

/// The join of a select over its source: the step that makes it, the fields it gives, and the columns of either side
/// its `where` names as `alias.column`. A side's column the select does not keep is given after the kept ones, for the
/// `where` to read.
pub(crate) fn select_join(
    builder: &mut Builder<'_>,
    name: &syntax::StepName,
    select: &syntax::Select,
    join: &syntax::Join,
    source: Sourced,
    last: LastStep,
) -> Result<Joined, LoweringError> {
    let Sourced { input, fields } = source;
    let Sourced {
        input: right_input,
        fields: right_fields,
    } = read_source(builder, &join.source)?;
    let left_alias = &select.from.alias;
    let (mut left_on, mut right_on) = (Vec::new(), Vec::new());
    for key in &join.on {
        // An `on` names its columns by the aliases of the two sides, as a `where` does: any other alias is refused.
        for reference in [&key.left, &key.right] {
            if let syntax::Alias::Written(alias) = &reference.alias
                && reference.alias != *left_alias
                && reference.alias != join.source.alias
            {
                return Err(unknown_side(builder, select, join, alias));
            }
        }
        let swapped = key.left.alias != syntax::Alias::Unwritten && key.left.alias != *left_alias;
        let (left_key, right_key) = if swapped {
            (&key.right, &key.left)
        } else {
            (&key.left, &key.right)
        };
        left_on.push(position(
            &fields,
            QuerySpelling(&left_key.column.0),
            builder.span,
        )?);
        right_on.push(position(
            &right_fields,
            QuerySpelling(&right_key.column.0),
            builder.span,
        )?);
    }
    let mut output = Vec::new();
    let mut kept = Vec::new();
    let listed: &[syntax::SelectColumn] = match &select.columns {
        syntax::Columns::Listed(columns) => columns,
        syntax::Columns::Every => &[],
    };
    for column in listed {
        let from_right =
            column.column.alias != syntax::Alias::Unwritten && column.column.alias != *left_alias;
        let side_fields = if from_right { &right_fields } else { &fields };
        let index = position(
            side_fields,
            QuerySpelling(&column.column.column.0),
            builder.span,
        )?;
        output.push(parts::JoinColumn {
            side: if from_right {
                parts::Side::Right
            } else {
                parts::Side::Left
            },
            column: index,
        });
        kept.push(PlannedColumn {
            name: match &column.rename {
                syntax::Renaming::Renamed(renamed) => renamed.clone(),
                syntax::Renaming::Unchanged => column.column.column.clone(),
            },
            kind: column_of(side_fields, index).kind,
            unique: Uniqueness::Shared,
            words: column_of(side_fields, index).words,
        });
    }
    let sides = side_columns(
        builder,
        select,
        join,
        [&fields, &right_fields],
        &mut output,
        &mut kept,
    )?;
    let operation = plans::Operation::Join {
        left: input,
        right: right_input,
        left_on,
        right_on,
        output,
        filter: Vec::new(),
        view: Vec::new(),
    };
    let id = pushed(
        builder,
        (last == LastStep::Join).then_some(name),
        operation,
        kept.clone(),
    );
    Ok(Joined {
        sourced: Sourced {
            input: parts::Input::Step(id),
            fields: kept,
        },
        sides,
    })
}

/// The refusal of `alias`, which names neither side of the join, listing the aliases its two sides have.
fn unknown_side(
    builder: &Builder<'_>,
    select: &syntax::Select,
    join: &syntax::Join,
    alias: &syntax::AliasName,
) -> LoweringError {
    LoweringError::UnknownSide {
        alias: alias.0.to_string(),
        sides: [&select.from.alias, &join.source.alias]
            .iter()
            .filter_map(|each| match each {
                syntax::Alias::Written(written) => Some(format!("`{}`", written.0)),
                syntax::Alias::Unwritten => None,
            })
            .collect::<Vec<_>>()
            .join(SIDE_SEPARATOR),
        span: builder.span,
    }
}

/// The columns of either side that the `where` of a join names as `alias.column`, each with its place in the rows of the
/// join: a column the select keeps is read there, any other is added to `output` and `kept` after the kept ones.
fn side_columns(
    builder: &Builder<'_>,
    select: &syntax::Select,
    join: &syntax::Join,
    both: [&[PlannedColumn]; 2],
    output: &mut Vec<parts::JoinColumn>,
    kept: &mut Vec<PlannedColumn>,
) -> Result<Vec<SideColumn>, LoweringError> {
    let mut sides = Vec::new();
    for reference in aliased(&select.filters) {
        let syntax::Alias::Written(alias) = &reference.alias else {
            continue;
        };
        let side = if reference.alias == select.from.alias {
            parts::Side::Left
        } else if reference.alias == join.source.alias {
            parts::Side::Right
        } else {
            return Err(unknown_side(builder, select, join, alias));
        };
        let [left_fields, right_fields] = both;
        let side_fields = match side {
            parts::Side::Left => left_fields,
            parts::Side::Right => right_fields,
        };
        let column = position(
            side_fields,
            QuerySpelling(&reference.column.0),
            builder.span,
        )
        .map_err(|_unknown| LoweringError::UnknownSideColumn {
            alias: alias.0.to_string(),
            name: reference.column.0.to_string(),
            span: builder.span,
        })?;
        let wanted = parts::JoinColumn { side, column };
        let index = if let Some(index) = find_column(output, |each| {
            if *each == wanted {
                SearchMatch::Matches
            } else {
                SearchMatch::Differs
            }
        }) {
            index
        } else {
            output.push(wanted);
            kept.push(PlannedColumn {
                name: reference.column.clone(),
                kind: column_of(side_fields, column).kind,
                unique: Uniqueness::Shared,
                words: column_of(side_fields, column).words,
            });
            parts::ColumnIndex(u32::try_from(output.len() - 1).unwrap_or(u32::MAX))
        };
        sides.push(SideColumn {
            alias: alias.clone(),
            column: reference.column.clone(),
            index,
        });
    }
    Ok(sides)
}

/// The columns the tests of `conditions` name with an alias, however deep in an `or`, in the order written.
fn aliased(conditions: &[syntax::Condition]) -> Vec<&syntax::ColumnReference> {
    let mut found = Vec::new();
    for condition in conditions {
        match condition {
            syntax::Condition::Test(filter) => {
                for operand in [&filter.left, &filter.right] {
                    operand_columns(operand, &mut found);
                }
            }
            syntax::Condition::Any(alternatives) => {
                for alternative in alternatives {
                    found.extend(aliased(alternative));
                }
            }
        }
    }
    found
}

/// Adds to `found` the columns an operand names with an alias, itself or through the arguments of its function.
fn operand_columns<'a>(
    operand: &'a syntax::WrittenOperand,
    found: &mut Vec<&'a syntax::ColumnReference>,
) {
    match operand {
        syntax::WrittenOperand::Column(reference) => {
            if matches!(reference.alias, syntax::Alias::Written(_)) {
                found.push(reference);
            }
        }
        syntax::WrittenOperand::Function(applied) => {
            for argument in &applied.arguments {
                operand_columns(argument, found);
            }
        }
        syntax::WrittenOperand::Parameter(_)
        | syntax::WrittenOperand::Text(_)
        | syntax::WrittenOperand::Number(_) => {}
    }
}

/// How many columns of a join's rows a `select` shows: those it lists, none for `*`.
pub(crate) fn shown_of(columns: &syntax::Columns) -> ShownColumns {
    ShownColumns(match columns {
        syntax::Columns::Listed(listed) => listed.len(),
        syntax::Columns::Every => 0,
    })
}
