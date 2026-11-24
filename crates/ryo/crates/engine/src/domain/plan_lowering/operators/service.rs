//! The `operators` service: a call's arguments as its operator's step.

use ryo_values::domain::finish_plans::arguments::groups::types::{
    CountedRows, GroupCountArguments, GroupOrdinalArguments, ProjectArguments,
};
use ryo_values::domain::finish_plans::arguments::joins::types::{
    AntijoinArguments, AsOfArguments, OrderedMatchArguments, UnionArguments,
};
use ryo_values::domain::finish_plans::arguments::literals::types::{
    ExtendArguments, RowsArguments, UnlistedArgument, UnlistedArguments,
};
use ryo_values::domain::finish_plans::arguments::texts::types::{
    CaptureArguments, CapturedText, FitsRenderedArguments, OptionalColumn, PrefixesArguments, RenderSyntaxArguments,
    ReplaceArguments, SourceMatchesArguments, SpanBetweenArguments, SpanBoundsArguments,
};
use ryo_values::domain::finish_plans::arguments::walks::types::{
    AxisArguments, OrderedDfsArguments, SubtreeClassArguments,
};
use ryo_values::domain::finish_plans::constants::{
    ARG_BASE_LINE, ARG_CAPTURE, ARG_CAPTURE_LINE_OUTPUT, ARG_CAPTURE_OUTPUT, ARG_CAPTURE_TEXT_OUTPUT, ARG_CAPTURE_TEXTS, ARG_CAPTURES, ARG_CHILD, ARG_ID, ARG_PARENT,
    ARG_COLUMNS, ARG_CONTEXT, ARG_COUNT, ARG_DESTINATION, ARG_DISTINCT, ARG_DOMAIN, ARG_EDGE, ARG_EDGES, ARG_END,
    ARG_END_OUTPUT, ARG_FROM, ARG_KEYS, ARG_LABEL, ARG_LABELS, ARG_LEAF_LABEL, ARG_LEAVES, ARG_LEFT, ARG_LEFT_ON,
    ARG_LEFT_ORDER, ARG_LINE_OUTPUT, ARG_LITERALS, ARG_MAX_LINE, ARG_NODE, ARG_ORDER, ARG_OUTPUT, ARG_PATH,
    ARG_PATTERN, ARG_RELATION, ARG_REPLACEMENT, ARG_RIGHT, ARG_RIGHT_ON, ARG_RIGHT_ORDER, ARG_ROOT, ARG_SEPARATOR,
    ARG_SOURCE, ARG_SPAN, ARG_START, ARG_START_OUTPUT, ARG_STEP, ARG_TAB_WIDTH, ARG_TEXT, ARG_TEXTS, OP_ANTIJOIN,
    OP_AS_OF, OP_AXIS, OP_CAPTURE, OP_CLOSURE, OP_EXTEND, OP_FITS_RENDERED, OP_FOLLOW, OP_GROUP_COUNT, OP_GROUP_MAX,
    OP_GROUP_MIN, OP_GROUP_ORDINAL, OP_ORDERED_DFS, OP_ORDERED_MATCH, OP_PREFIXES, OP_PROJECT, OP_RENDER_SYNTAX,
    OP_REPLACE, OP_ROWS, OP_SOURCE_MATCHES, OP_SPAN_BETWEEN, OP_SPAN_BOUNDS, OP_SUBTREE_CLASS, OP_UNION,
};
use ryo_values::domain::finish_plans::types::{AxisStep, ExistsMark, FinishStep};
use crate::domain::plan_lowering::errors::PlanLoweringError;
use crate::domain::plan_lowering::given::service::{
    bound, column, columns, constant_rows, direction, edges, extended_values, group_order, optional_column,
    optional_text, output_columns, refused, step, text, text_map, texts, whole,
};
use crate::domain::plan_lowering::given::types::GivenArguments;
use ryo_values::domain::descriptions::types::{ColumnName, OperatorName};

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
        OP_ROWS => {
            let columns = columns(&mut given, ARG_COLUMNS)?;
            let rows = constant_rows(&mut given, &columns)?;
            FinishStep::Rows(RowsArguments { rows, columns })
        }
        other => text_step(&mut given, other)?,
    };
    match given.arguments.first() {
        None => Ok(step),
        Some(left) => Err(refused(&given, &left.name.0)),
    }
}

/// The steps of the operators over spans and text, and `prefixes` and `subtree-class`; an operator outside the step set as written.
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
            capture_texts: text_map(given, ARG_CAPTURE_TEXTS)?
                .into_iter()
                .map(|(group, output)| CapturedText { group: ColumnName(group), output: ColumnName(output) })
                .collect(),
        }),
        OP_CAPTURE => FinishStep::Capture(CaptureArguments {
            from: step(given, ARG_FROM)?,
            path: column(given, ARG_PATH)?,
            pattern: text(given, ARG_PATTERN)?,
            captures: columns(given, ARG_CAPTURES)?,
            separator: optional_text(given, ARG_SEPARATOR)?,
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
        OP_PREFIXES => FinishStep::Prefixes(PrefixesArguments {
            from: step(given, ARG_FROM)?,
            text: column(given, ARG_TEXT)?,
            separator: text(given, ARG_SEPARATOR)?,
            output: column(given, ARG_OUTPUT)?,
        }),
        OP_SUBTREE_CLASS => FinishStep::SubtreeClass(SubtreeClassArguments {
            from: step(given, ARG_FROM)?,
            relation: step(given, ARG_RELATION)?,
            labels: step(given, ARG_LABELS)?,
            leaves: step(given, ARG_LEAVES)?,
            label: columns(given, ARG_LABEL)?,
            leaf_label: columns(given, ARG_LEAF_LABEL)?,
            step: columns(given, ARG_STEP)?,
            output: column(given, ARG_OUTPUT)?,
            id: column(given, ARG_ID)?,
            parent: column(given, ARG_PARENT)?,
            child: column(given, ARG_CHILD)?,
        }),
        unlisted => FinishStep::Unlisted(UnlistedArguments {
            operator: OperatorName(unlisted.into()),
            arguments: std::mem::take(&mut given.arguments),
        }),
    })
}

