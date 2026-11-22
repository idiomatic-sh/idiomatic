//! The `operators` service: a call's arguments as its operator's step.

use super::types::GivenArguments;
use crate::domain::finish_plans::arguments::literals::types::{
    ArgumentValue, ConstantRow, ExtendArguments, ExtendedValue, RowsArguments, UnlistedArgument,
    UnlistedArguments,
};
use crate::domain::finish_plans::arguments::groups::types::{
    CountedRows, GroupCountArguments, GroupOrderArguments, GroupOrdinalArguments,
    ProjectArguments,
};
use crate::domain::finish_plans::arguments::joins::types::{
    AntijoinArguments, AsOfArguments, AsOfBound, OrderedMatchArguments, OutputColumn,
    UnionArguments,
};
use crate::domain::finish_plans::arguments::texts::types::{
    CaptureArguments, FitsRenderedArguments, OptionalColumn, RenderSyntaxArguments,
    ReplaceArguments, SourceMatchesArguments, SpanBetweenArguments, SpanBoundsArguments,
};
use crate::domain::finish_plans::arguments::types::StepNumber;
use crate::domain::finish_plans::arguments::walks::types::{
    AxisArguments, EdgeArguments, EdgeKeys, OrderedDfsArguments,
};
use crate::domain::finish_plans::constants::{
    ARG_BASE_LINE, ARG_BOUND, ARG_CAPTURE, ARG_CAPTURE_LINE_OUTPUT, ARG_CAPTURE_OUTPUT,
    ARG_CAPTURE_TEXT_OUTPUT, ARG_CAPTURES, ARG_COLUMNS, ARG_CONTEXT, ARG_COUNT, ARG_DESTINATION,
    ARG_DIRECTION, ARG_DISTINCT, ARG_DOMAIN, ARG_EDGE, ARG_EDGES, ARG_END, ARG_END_OUTPUT,
    ARG_FROM, ARG_KEYS, ARG_LEFT, ARG_LEFT_ON, ARG_LEFT_ORDER, ARG_LINE_OUTPUT, ARG_LITERALS,
    ARG_MAX_LINE, ARG_NODE, ARG_ORDER, ARG_OUTPUT, ARG_PATH, ARG_PATTERN, ARG_RELATION,
    ARG_REPLACEMENT, ARG_RIGHT, ARG_RIGHT_ON, ARG_RIGHT_ORDER, ARG_ROOT, ARG_SOURCE, ARG_SPAN,
    ARG_START, ARG_START_OUTPUT, ARG_TAB_WIDTH, ARG_TEXT, ARG_TEXTS, ARG_VALUES, BOUND_AFTER,
    BOUND_AT_OR_AFTER, BOUND_AT_OR_BEFORE, BOUND_BEFORE, DIRECTION_ANCESTOR, DIRECTION_CHILD,
    DIRECTION_DEEP, DIRECTION_DESCENDANT, DIRECTION_NEAREST_ANCESTOR, DIRECTION_NEXT_SIBLING,
    DIRECTION_PARENT, DIRECTION_PREVIOUS_SIBLING, OP_ANTIJOIN, OP_AS_OF, OP_AXIS, OP_CAPTURE,
    OP_CLOSURE, OP_EXTEND, OP_FITS_RENDERED, OP_FOLLOW, OP_GROUP_COUNT, OP_GROUP_MAX,
    OP_GROUP_MIN, OP_GROUP_ORDINAL, OP_ORDERED_DFS, OP_ORDERED_MATCH, OP_PROJECT,
    OP_RENDER_SYNTAX, OP_REPLACE, OP_ROWS, OP_SOURCE_MATCHES, OP_SPAN_BETWEEN, OP_SPAN_BOUNDS,
    OP_UNION, SIDE_LEFT, SIDE_RIGHT,
};
use crate::domain::finish_plans::expressions::types::{PlanConstant, PlanNumber};
use crate::domain::finish_plans::types::{AxisStep, ExistsMark, FinishStep};
use crate::domain::plan_lowering::errors::PlanLoweringError;
use crate::domain::plan_lowering::types::WrittenText;
use ryo_values::domain::descriptions::types::{ColumnName, OperatorName};
use ryo_values::domain::plan_parts::types::{Direction, LiteralText, Side, WholeNumber};

/// The step of a call of `operator` with `arguments`, in the order written: the operator's typed step, or for an operator
/// outside the step set the call as written.
///
/// # Errors
///
/// `OperatorArgument` when an operator of the step set is not given an argument it needs, is given one it does not
/// take, or is given one of a shape it does not take.
pub(crate) fn operator_step(
    operator: &str,
    arguments: Vec<UnlistedArgument>,
) -> Result<FinishStep, PlanLoweringError> {
    let mut given = GivenArguments {
        operator,
        arguments,
    };
    let step = match operator {
        OP_UNION => FinishStep::Union(UnionArguments {
            left: step(&mut given, ARG_LEFT)?,
            right: step(&mut given, ARG_RIGHT)?,
        }),
        OP_ANTIJOIN => FinishStep::Antijoin(AntijoinArguments {
            left: step(&mut given, ARG_LEFT)?,
            right: step(&mut given, ARG_RIGHT)?,
            left_on: columns(&mut given, ARG_LEFT_ON)?,
            right_on: columns(&mut given, ARG_RIGHT_ON)?,
        }),
        OP_AS_OF => FinishStep::AsOf(AsOfArguments {
            left: step(&mut given, ARG_LEFT)?,
            right: step(&mut given, ARG_RIGHT)?,
            left_on: columns(&mut given, ARG_LEFT_ON)?,
            right_on: columns(&mut given, ARG_RIGHT_ON)?,
            left_order: column(&mut given, ARG_LEFT_ORDER)?,
            right_order: column(&mut given, ARG_RIGHT_ORDER)?,
            bound: bound(&mut given)?,
            output: output_columns(&mut given)?,
        }),
        OP_ORDERED_MATCH => FinishStep::OrderedMatch(OrderedMatchArguments {
            left: step(&mut given, ARG_LEFT)?,
            right: step(&mut given, ARG_RIGHT)?,
            left_on: columns(&mut given, ARG_LEFT_ON)?,
            right_on: columns(&mut given, ARG_RIGHT_ON)?,
            left_order: column(&mut given, ARG_LEFT_ORDER)?,
            right_order: column(&mut given, ARG_RIGHT_ORDER)?,
            output: output_columns(&mut given)?,
        }),
        OP_AXIS => FinishStep::Axis(AxisStep {
            arguments: AxisArguments {
                from: step(&mut given, ARG_FROM)?,
                relation: step(&mut given, ARG_RELATION)?,
                direction: direction(&mut given)?,
                domain: step(&mut given, ARG_DOMAIN)?,
            },
            exists: ExistsMark::Unmarked,
        }),
        OP_CLOSURE => FinishStep::Closure(edges(&mut given)?),
        OP_FOLLOW => FinishStep::Follow(edges(&mut given)?),
        OP_ORDERED_DFS => FinishStep::OrderedDfs(OrderedDfsArguments {
            from: step(&mut given, ARG_FROM)?,
            root: column(&mut given, ARG_ROOT)?,
            edges: step(&mut given, ARG_EDGES)?,
            source: column(&mut given, ARG_SOURCE)?,
            destination: column(&mut given, ARG_DESTINATION)?,
            order: column(&mut given, ARG_ORDER)?,
            edge: column(&mut given, ARG_EDGE)?,
            keys: columns(&mut given, ARG_KEYS)?,
            output: column(&mut given, ARG_OUTPUT)?,
        }),
        OP_GROUP_COUNT => FinishStep::GroupCount(GroupCountArguments {
            from: step(&mut given, ARG_FROM)?,
            keys: columns(&mut given, ARG_KEYS)?,
            count: column(&mut given, ARG_COUNT)?,
            distinct: match optional_column(&mut given, ARG_DISTINCT)? {
                OptionalColumn::Unwritten => CountedRows::Every,
                OptionalColumn::Written(column) => CountedRows::Distinct(column),
            },
        }),
        OP_GROUP_MIN => FinishStep::GroupMin(group_order(&mut given)?),
        OP_GROUP_MAX => FinishStep::GroupMax(group_order(&mut given)?),
        OP_GROUP_ORDINAL => FinishStep::GroupOrdinal(GroupOrdinalArguments {
            from: step(&mut given, ARG_FROM)?,
            keys: columns(&mut given, ARG_KEYS)?,
            order: column(&mut given, ARG_ORDER)?,
            output: column(&mut given, ARG_OUTPUT)?,
        }),
        OP_PROJECT => FinishStep::Project(ProjectArguments {
            from: step(&mut given, ARG_FROM)?,
            columns: columns(&mut given, ARG_COLUMNS)?,
        }),
        OP_EXTEND => FinishStep::Extend(ExtendArguments {
            from: step(&mut given, ARG_FROM)?,
            values: extended_values(&mut given)?,
        }),
        OP_ROWS => FinishStep::Rows(RowsArguments {
            rows: constant_rows(&mut given)?,
            columns: columns(&mut given, ARG_COLUMNS)?,
        }),
        other => text_step(&mut given, other)?,
    };
    match given.arguments.first() {
        None => Ok(step),
        Some(left) => Err(refused(&given, &left.name.0)),
    }
}

/// The steps of the operators over spans and text; an operator outside the step set as written.
fn text_step(
    given: &mut GivenArguments<'_>,
    operator: &str,
) -> Result<FinishStep, PlanLoweringError> {
    Ok(match operator {
        OP_SPAN_BOUNDS => FinishStep::SpanBounds(SpanBoundsArguments {
            from: step(given, ARG_FROM)?,
            span: column(given, ARG_SPAN)?,
            start_output: column(given, ARG_START_OUTPUT)?,
            end_output: column(given, ARG_END_OUTPUT)?,
        }),
        OP_SPAN_BETWEEN => FinishStep::SpanBetween(SpanBetweenArguments {
            from: step(given, ARG_FROM)?,
            start: column(given, ARG_START)?,
            end: column(given, ARG_END)?,
            output: column(given, ARG_OUTPUT)?,
        }),
        OP_REPLACE => FinishStep::Replace(ReplaceArguments {
            from: step(given, ARG_FROM)?,
            text: column(given, ARG_TEXT)?,
            pattern: text(given, ARG_PATTERN)?,
            replacement: text(given, ARG_REPLACEMENT)?,
            output: column(given, ARG_OUTPUT)?,
        }),
        OP_SOURCE_MATCHES => FinishStep::SourceMatches(SourceMatchesArguments {
            from: step(given, ARG_FROM)?,
            span: column(given, ARG_SPAN)?,
            base_line: optional_column(given, ARG_BASE_LINE)?,
            pattern: text(given, ARG_PATTERN)?,
            output: column(given, ARG_OUTPUT)?,
            line_output: optional_column(given, ARG_LINE_OUTPUT)?,
            capture: optional_column(given, ARG_CAPTURE)?,
            capture_output: optional_column(given, ARG_CAPTURE_OUTPUT)?,
            capture_text_output: optional_column(given, ARG_CAPTURE_TEXT_OUTPUT)?,
            capture_line_output: optional_column(given, ARG_CAPTURE_LINE_OUTPUT)?,
        }),
        OP_CAPTURE => FinishStep::Capture(CaptureArguments {
            from: step(given, ARG_FROM)?,
            path: column(given, ARG_PATH)?,
            pattern: text(given, ARG_PATTERN)?,
            captures: columns(given, ARG_CAPTURES)?,
        }),
        OP_FITS_RENDERED => FinishStep::FitsRendered(FitsRenderedArguments {
            from: step(given, ARG_FROM)?,
            texts: columns(given, ARG_TEXTS)?,
            literals: texts(given, ARG_LITERALS)?,
            span: column(given, ARG_SPAN)?,
            max_line: whole(given, ARG_MAX_LINE)?,
            tab_width: whole(given, ARG_TAB_WIDTH)?,
        }),
        OP_RENDER_SYNTAX => FinishStep::RenderSyntax(RenderSyntaxArguments {
            from: step(given, ARG_FROM)?,
            node: column(given, ARG_NODE)?,
            context: text(given, ARG_CONTEXT)?,
            output: column(given, ARG_OUTPUT)?,
        }),
        unlisted => FinishStep::Unlisted(UnlistedArguments {
            operator: OperatorName(unlisted.into()),
            arguments: std::mem::take(&mut given.arguments),
        }),
    })
}

/// The refusal of an argument of the call.
fn refused(given: &GivenArguments<'_>, argument: &str) -> PlanLoweringError {
    PlanLoweringError::OperatorArgument {
        operator: WrittenText(given.operator.into()),
        argument: WrittenText(argument.into()),
    }
}

/// Takes the argument `name` out of the call: none when it is not given.
fn taken(given: &mut GivenArguments<'_>, name: &str) -> Option<ArgumentValue> {
    let at = given
        .arguments
        .iter()
        .position(|argument| &*argument.name.0 == name)?;
    Some(given.arguments.remove(at).value)
}

/// The constant argument `name`, which the call must give.
fn given_constant(
    given: &mut GivenArguments<'_>,
    name: &str,
) -> Result<PlanConstant, PlanLoweringError> {
    match taken(given, name) {
        Some(ArgumentValue::Constant(constant)) => Ok(constant),
        Some(ArgumentValue::Step(_)) | None => Err(refused(given, name)),
    }
}

/// The relation argument `name`: its step.
fn step(given: &mut GivenArguments<'_>, name: &str) -> Result<StepNumber, PlanLoweringError> {
    match taken(given, name) {
        Some(ArgumentValue::Step(step)) => Ok(step),
        Some(ArgumentValue::Constant(_)) | None => Err(refused(given, name)),
    }
}

/// A constant that is text.
fn as_text(constant: PlanConstant) -> Option<Box<str>> {
    match constant {
        PlanConstant::Text(text) => Some(text.0),
        PlanConstant::Number(_) | PlanConstant::List(_) | PlanConstant::Map(_) | PlanConstant::Absent => {
            None
        }
    }
}

/// The text argument `name`.
fn text(given: &mut GivenArguments<'_>, name: &str) -> Result<LiteralText, PlanLoweringError> {
    let constant = given_constant(given, name)?;
    as_text(constant)
        .map(LiteralText)
        .ok_or_else(|| refused(given, name))
}

/// The column argument `name`.
fn column(given: &mut GivenArguments<'_>, name: &str) -> Result<ColumnName, PlanLoweringError> {
    Ok(ColumnName(text(given, name)?.0))
}

/// The column argument `name`, when the call gives it.
fn optional_column(
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
fn text_list(constant: PlanConstant) -> Option<Vec<Box<str>>> {
    match constant {
        PlanConstant::List(items) => items.into_iter().map(as_text).collect(),
        PlanConstant::Text(_) | PlanConstant::Number(_) | PlanConstant::Map(_) | PlanConstant::Absent => {
            None
        }
    }
}

/// The text list argument `name`.
fn texts(given: &mut GivenArguments<'_>, name: &str) -> Result<Vec<LiteralText>, PlanLoweringError> {
    let constant = given_constant(given, name)?;
    text_list(constant)
        .map(|items| items.into_iter().map(LiteralText).collect())
        .ok_or_else(|| refused(given, name))
}

/// The column list argument `name`.
fn columns(given: &mut GivenArguments<'_>, name: &str) -> Result<Vec<ColumnName>, PlanLoweringError> {
    Ok(texts(given, name)?
        .into_iter()
        .map(|text| ColumnName(text.0))
        .collect())
}

/// The whole number argument `name`.
fn whole(given: &mut GivenArguments<'_>, name: &str) -> Result<WholeNumber, PlanLoweringError> {
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
fn direction(given: &mut GivenArguments<'_>) -> Result<Direction, PlanLoweringError> {
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
fn bound(given: &mut GivenArguments<'_>) -> Result<AsOfBound, PlanLoweringError> {
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
fn output_columns(given: &mut GivenArguments<'_>) -> Result<Vec<OutputColumn>, PlanLoweringError> {
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
fn extended_values(given: &mut GivenArguments<'_>) -> Result<Vec<ExtendedValue>, PlanLoweringError> {
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
fn constant_rows(given: &mut GivenArguments<'_>) -> Result<Vec<ConstantRow>, PlanLoweringError> {
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
fn edges(given: &mut GivenArguments<'_>) -> Result<EdgeArguments, PlanLoweringError> {
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
fn group_order(given: &mut GivenArguments<'_>) -> Result<GroupOrderArguments, PlanLoweringError> {
    Ok(GroupOrderArguments {
        from: step(given, ARG_FROM)?,
        keys: columns(given, ARG_KEYS)?,
        order: column(given, ARG_ORDER)?,
    })
}
