//! The `given` service: a call's arguments taken one by one, each as the shape its operator takes.

use super::types::GivenArguments;
use ryo_values::domain::finish_plans::arguments::groups::types::GroupOrderArguments;
use ryo_values::domain::finish_plans::arguments::joins::types::{AsOfBound, OutputColumn};
use ryo_values::domain::finish_plans::arguments::literals::types::{ArgumentValue, ConstantRow, ExtendedValue};
use ryo_values::domain::finish_plans::arguments::texts::types::OptionalColumn;
use ryo_values::domain::finish_plans::arguments::types::StepNumber;
use ryo_values::domain::finish_plans::arguments::walks::types::{EdgeArguments, EdgeKeys};
use ryo_values::domain::finish_plans::constants::{
    ARG_BOUND, ARG_DESTINATION, ARG_DIRECTION, ARG_EDGES, ARG_FROM, ARG_KEYS, ARG_ORDER, ARG_OUTPUT, ARG_SOURCE,
    ARG_VALUES, BOUND_AFTER, BOUND_AT_OR_AFTER, BOUND_AT_OR_BEFORE, BOUND_BEFORE, DIRECTION_ANCESTOR, DIRECTION_CHILD,
    DIRECTION_DEEP, DIRECTION_DESCENDANT, DIRECTION_NEAREST_ANCESTOR, DIRECTION_NEXT_SIBLING, DIRECTION_PARENT,
    DIRECTION_PREVIOUS_SIBLING, SIDE_LEFT, SIDE_RIGHT,
};
use ryo_values::domain::finish_plans::expressions::types::{PlanConstant, PlanNumber};
use crate::domain::plan_lowering::errors::PlanLoweringError;
use crate::domain::plan_lowering::types::WrittenText;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::plan_parts::types::{Direction, LiteralText, Side, WholeNumber};

/// The refusal of an argument of the call.
pub(crate) fn refused(given: &GivenArguments<'_>, argument: &str) -> PlanLoweringError {
    PlanLoweringError::OperatorArgument {
        operator: WrittenText(given.operator.into()),
        argument: WrittenText(argument.into()),
    }
}

/// Takes the argument `name` out of the call: none when it is not given.
pub(crate) fn taken(given: &mut GivenArguments<'_>, name: &str) -> Option<ArgumentValue> {
    let at = given
        .arguments
        .iter()
        .position(|argument| &*argument.name.0 == name)?;
    Some(given.arguments.remove(at).value)
}

/// The constant argument `name`, which the call must give.
pub(crate) fn given_constant(
    given: &mut GivenArguments<'_>,
    name: &str,
) -> Result<PlanConstant, PlanLoweringError> {
    match taken(given, name) {
        Some(ArgumentValue::Constant(constant)) => Ok(constant),
        Some(ArgumentValue::Step(_)) | None => Err(refused(given, name)),
    }
}

/// The relation argument `name`: its step.
pub(crate) fn step(given: &mut GivenArguments<'_>, name: &str) -> Result<StepNumber, PlanLoweringError> {
    match taken(given, name) {
        Some(ArgumentValue::Step(step)) => Ok(step),
        Some(ArgumentValue::Constant(_)) | None => Err(refused(given, name)),
    }
}

/// A constant that is text.
pub(crate) fn as_text(constant: PlanConstant) -> Option<Box<str>> {
    match constant {
        PlanConstant::Text(text) => Some(text.0),
        PlanConstant::Number(_) | PlanConstant::List(_) | PlanConstant::Map(_) | PlanConstant::Absent => {
            None
        }
    }
}

/// The text argument `name`.
pub(crate) fn text(given: &mut GivenArguments<'_>, name: &str) -> Result<LiteralText, PlanLoweringError> {
    let constant = given_constant(given, name)?;
    as_text(constant)
        .map(LiteralText)
        .ok_or_else(|| refused(given, name))
}

/// The column argument `name`.
pub(crate) fn column(given: &mut GivenArguments<'_>, name: &str) -> Result<ColumnName, PlanLoweringError> {
    Ok(ColumnName(text(given, name)?.0))
}

/// The column argument `name`, when the call gives it.
pub(crate) fn optional_column(
    given: &mut GivenArguments<'_>,
    name: &str,
) -> Result<OptionalColumn, PlanLoweringError> {
    if given
        .arguments
        .iter()
        .any(|argument| &*argument.name.0 == name)
    {
        Ok(OptionalColumn::Written(column(given, name)?))
    } else {
        Ok(OptionalColumn::Unwritten)
    }
}

/// A constant list of texts.
pub(crate) fn text_list(constant: PlanConstant) -> Option<Vec<Box<str>>> {
    match constant {
        PlanConstant::List(items) => items.into_iter().map(as_text).collect(),
        PlanConstant::Text(_) | PlanConstant::Number(_) | PlanConstant::Map(_) | PlanConstant::Absent => {
            None
        }
    }
}

/// The text list argument `name`.
pub(crate) fn texts(given: &mut GivenArguments<'_>, name: &str) -> Result<Vec<LiteralText>, PlanLoweringError> {
    let constant = given_constant(given, name)?;
    text_list(constant)
        .map(|items| items.into_iter().map(LiteralText).collect())
        .ok_or_else(|| refused(given, name))
}

/// The column list argument `name`.
pub(crate) fn columns(given: &mut GivenArguments<'_>, name: &str) -> Result<Vec<ColumnName>, PlanLoweringError> {
    Ok(texts(given, name)?
        .into_iter()
        .map(|text| ColumnName(text.0))
        .collect())
}

/// The whole number argument `name`.
pub(crate) fn whole(given: &mut GivenArguments<'_>, name: &str) -> Result<WholeNumber, PlanLoweringError> {
    match given_constant(given, name)? {
        PlanConstant::Number(PlanNumber::Whole(number)) => Ok(number),
        PlanConstant::Number(PlanNumber::Fraction(_))
        | PlanConstant::Text(_)
        | PlanConstant::List(_)
        | PlanConstant::Map(_)
        | PlanConstant::Absent => Err(refused(given, name)),
    }
}

/// An axis's `direction`.
pub(crate) fn direction(given: &mut GivenArguments<'_>) -> Result<Direction, PlanLoweringError> {
    let written = text(given, ARG_DIRECTION)?;
    Ok(match &*written.0 {
        DIRECTION_PARENT => Direction::Parent,
        DIRECTION_CHILD => Direction::Child,
        DIRECTION_ANCESTOR => Direction::Ancestor,
        DIRECTION_DESCENDANT => Direction::Descendant,
        DIRECTION_NEAREST_ANCESTOR => Direction::NearestAncestor,
        DIRECTION_DEEP => Direction::Deep,
        DIRECTION_PREVIOUS_SIBLING => Direction::PreviousSibling,
        DIRECTION_NEXT_SIBLING => Direction::NextSibling,
        _ => return Err(refused(given, ARG_DIRECTION)),
    })
}

/// An `as-of`'s `bound`.
pub(crate) fn bound(given: &mut GivenArguments<'_>) -> Result<AsOfBound, PlanLoweringError> {
    let written = text(given, ARG_BOUND)?;
    Ok(match &*written.0 {
        BOUND_AT_OR_BEFORE => AsOfBound::AtOrBefore,
        BOUND_BEFORE => AsOfBound::Before,
        BOUND_AT_OR_AFTER => AsOfBound::AtOrAfter,
        BOUND_AFTER => AsOfBound::After,
        _ => return Err(refused(given, ARG_BOUND)),
    })
}

/// An ordered step's `output`: a map from each column written to `[side, column]`.
pub(crate) fn output_columns(given: &mut GivenArguments<'_>) -> Result<Vec<OutputColumn>, PlanLoweringError> {
    let PlanConstant::Map(entries) = given_constant(given, ARG_OUTPUT)? else {
        return Err(refused(given, ARG_OUTPUT));
    };
    let mut written = Vec::with_capacity(entries.len());
    for entry in entries {
        let read = text_list(entry.value).ok_or_else(|| refused(given, ARG_OUTPUT))?;
        let [side, column] = <[Box<str>; 2]>::try_from(read).map_err(|_| refused(given, ARG_OUTPUT))?;
        let side = match &*side {
            SIDE_LEFT => Side::Left,
            SIDE_RIGHT => Side::Right,
            _ => return Err(refused(given, ARG_OUTPUT)),
        };
        written.push(OutputColumn {
            name: ColumnName(entry.key.0),
            side,
            column: ColumnName(column),
        });
    }
    Ok(written)
}

/// An `extend`'s `values`: a map from each column added to its constant.
pub(crate) fn extended_values(given: &mut GivenArguments<'_>) -> Result<Vec<ExtendedValue>, PlanLoweringError> {
    let PlanConstant::Map(entries) = given_constant(given, ARG_VALUES)? else {
        return Err(refused(given, ARG_VALUES));
    };
    Ok(entries
        .into_iter()
        .map(|entry| ExtendedValue {
            name: ColumnName(entry.key.0),
            value: entry.value,
        })
        .collect())
}

/// A `rows`'s `from`: its constant table, each row a list of texts.
pub(crate) fn constant_rows(given: &mut GivenArguments<'_>) -> Result<Vec<ConstantRow>, PlanLoweringError> {
    let PlanConstant::List(rows) = given_constant(given, ARG_FROM)? else {
        return Err(refused(given, ARG_FROM));
    };
    let mut table = Vec::with_capacity(rows.len());
    for row in rows {
        let cells = text_list(row).ok_or_else(|| refused(given, ARG_FROM))?;
        table.push(ConstantRow {
            cells: cells.into_iter().map(LiteralText).collect(),
        });
    }
    Ok(table)
}

/// A `closure`'s or a `follow`'s arguments.
pub(crate) fn edges(given: &mut GivenArguments<'_>) -> Result<EdgeArguments, PlanLoweringError> {
    Ok(EdgeArguments {
        edges: step(given, ARG_EDGES)?,
        source: column(given, ARG_SOURCE)?,
        destination: column(given, ARG_DESTINATION)?,
        keys: if given
            .arguments
            .iter()
            .any(|argument| &*argument.name.0 == ARG_KEYS)
        {
            EdgeKeys::Written(columns(given, ARG_KEYS)?)
        } else {
            EdgeKeys::Unwritten
        },
    })
}

/// A `group-min`'s or a `group-max`'s arguments.
pub(crate) fn group_order(given: &mut GivenArguments<'_>) -> Result<GroupOrderArguments, PlanLoweringError> {
    Ok(GroupOrderArguments {
        from: step(given, ARG_FROM)?,
        keys: columns(given, ARG_KEYS)?,
        order: column(given, ARG_ORDER)?,
    })
}
