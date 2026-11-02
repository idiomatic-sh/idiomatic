//! The names of the `syntax` concept: the one place where an operation or an argument is mapped
//! to the text a query writes and back. Text enters the engine here and becomes a variant.

use ryo_language::domain::relations::types::CellKind;

use super::types::ArgumentSlot;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plans::types::ParameterType;
use ryo_values::domain::plans::types::QueryOperator;

/// Every operator, in the order the language documents them.
pub(crate) const OPERATORS: [QueryOperator; 23] = [
    QueryOperator::SpanBetween,
    QueryOperator::SpanBounds,
    QueryOperator::SourceMatches,
    QueryOperator::Replace,
    QueryOperator::OrderedMatch,
    QueryOperator::OrderedDfs,
    QueryOperator::Select,
    QueryOperator::Project,
    QueryOperator::Extend,
    QueryOperator::Join,
    QueryOperator::Union,
    QueryOperator::Intersection,
    QueryOperator::Difference,
    QueryOperator::Antijoin,
    QueryOperator::GroupCount,
    QueryOperator::GroupDigest,
    QueryOperator::Capture,
    QueryOperator::Words,
    QueryOperator::Rank,
    QueryOperator::Axis,
    QueryOperator::Rows,
    QueryOperator::Closure,
    QueryOperator::Cycles,
];

/// The text a query writes for the `select` operator, which is also its keyword.
pub(crate) const SELECT: &str = "select";

/// The text a query writes for an operator.
#[must_use]
pub(crate) fn operator_name(operator: QueryOperator) -> &'static str {
    match operator {
        QueryOperator::SpanBetween => "span-between",
        QueryOperator::SpanBounds => "span-bounds",
        QueryOperator::SourceMatches => "source-matches",
        QueryOperator::Replace => "replace",
        QueryOperator::OrderedMatch => "ordered-match",
        QueryOperator::OrderedDfs => "ordered-dfs",
        QueryOperator::Select => SELECT,
        QueryOperator::Project => "project",
        QueryOperator::Extend => "extend",
        QueryOperator::Join => "join",
        QueryOperator::Union => "union",
        QueryOperator::Intersection => "intersection",
        QueryOperator::Difference => "difference",
        QueryOperator::Antijoin => "antijoin",
        QueryOperator::GroupCount => "group-count",
        QueryOperator::GroupDigest => "group-digest",
        QueryOperator::Capture => "capture",
        QueryOperator::Words => "words",
        QueryOperator::Rank => "rank",
        QueryOperator::Axis => "axis",
        QueryOperator::Rows => "rows",
        QueryOperator::Closure => "closure",
        QueryOperator::Cycles => "cycles",
    }
}

/// The operator a query calls `text`.
#[must_use]
pub(crate) fn operator_named(text: &str) -> Option<QueryOperator> {
    OPERATORS
        .into_iter()
        .find(|operator| operator_name(*operator) == text)
}

/// The text a query writes for an argument.
#[must_use]
pub(crate) fn slot_name(slot: ArgumentSlot) -> &'static str {
    match slot {
        ArgumentSlot::Start => "start",
        ArgumentSlot::End => "end",
        ArgumentSlot::Span => "span",
        ArgumentSlot::StartOutput => "start-output",
        ArgumentSlot::EndOutput => "end-output",
        ArgumentSlot::BaseLine => "base-line",
        ArgumentSlot::LineOutput => "line-output",
        ArgumentSlot::Replacement => ryo_values::domain::descriptions::constants::REPLACEMENT,
        ArgumentSlot::LeftOrder => "left-order",
        ArgumentSlot::RightOrder => "right-order",
        ArgumentSlot::From => "from",
        ArgumentSlot::Root => "root",
        ArgumentSlot::Order => "order",
        ArgumentSlot::Edge => "edge",
        ArgumentSlot::Columns => ryo_values::domain::descriptions::constants::COLUMNS,
        ArgumentSlot::Values => "values",
        ArgumentSlot::Left => "left",
        ArgumentSlot::Right => "right",
        ArgumentSlot::LeftOn => ryo_values::domain::descriptions::constants::LEFT_ON,
        ArgumentSlot::RightOn => ryo_values::domain::descriptions::constants::RIGHT_ON,
        ArgumentSlot::Output => ryo_values::domain::descriptions::constants::OUTPUT,
        ArgumentSlot::Keys => "keys",
        ArgumentSlot::Distinct => "distinct",
        ArgumentSlot::Count => "count",
        ArgumentSlot::Digest => ryo_values::domain::descriptions::constants::DIGEST,
        ArgumentSlot::Path => "path",
        ArgumentSlot::Pattern => ryo_values::domain::descriptions::constants::PATTERN,
        ArgumentSlot::Captures => "captures",
        ArgumentSlot::Text => "text",
        ArgumentSlot::Word => "word",
        ArgumentSlot::Relation => "relation",
        ArgumentSlot::Direction => "direction",
        ArgumentSlot::Domain => "domain",
        ArgumentSlot::Edges => "edges",
        ArgumentSlot::Source => ryo_language::domain::relations::constants::SOURCE,
        ArgumentSlot::Destination => "destination",
        ArgumentSlot::Column => ryo_values::domain::descriptions::constants::COLUMN,
        ArgumentSlot::Sets => "sets",
    }
}

/// The direction a query calls `text`.
#[must_use]
pub(crate) fn direction_named(text: &str) -> Option<Direction> {
    [
        Direction::Parent,
        Direction::Child,
        Direction::Ancestor,
        Direction::Descendant,
        Direction::NearestAncestor,
        Direction::PreviousSibling,
        Direction::NextSibling,
    ]
    .into_iter()
    .find(|direction| direction_name(*direction) == text)
}

/// The text a query writes for a direction.
#[must_use]
pub(crate) fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Parent => ryo_language::domain::relations::constants::PARENT,
        Direction::Child => "child",
        Direction::Ancestor => "ancestor",
        Direction::Descendant => "descendant",
        Direction::NearestAncestor => "nearest-ancestor",
        Direction::Deep => "deep",
        Direction::PreviousSibling => "previous-sibling",
        Direction::NextSibling => "next-sibling",
    }
}

/// The side a query calls `text`.
#[must_use]
pub(crate) fn side_named(text: &str) -> Option<Side> {
    [Side::Left, Side::Right]
        .into_iter()
        .find(|side| side_name(*side) == text)
}

/// The text a query writes for a side.
#[must_use]
pub(crate) fn side_name(side: Side) -> &'static str {
    match side {
        Side::Left => "left",
        Side::Right => "right",
    }
}

/// The kind of parameter a declared type is: `integer`, `list<string>`, `set<path>`.
#[must_use]
pub(crate) fn parameter_kind_named(declared: &str) -> Option<ParameterType> {
    // A list of tables of text, each a row: `list<table>`.
    if declared == "list<table>" {
        return Some(ParameterType::Tables);
    }
    // A list of sets of text, as the layers of a project: `list<set<string>>`.
    if let Some(inner) = declared
        .strip_prefix("list<set<")
        .and_then(|rest| rest.strip_suffix(">>"))
        && matches!(inner, "string" | "path" | "glob")
    {
        return Some(ParameterType::TextSets);
    }
    let element = declared
        .strip_prefix("list<")
        .or_else(|| declared.strip_prefix("set<"))
        .and_then(|rest| rest.strip_suffix('>'));
    let scalar = |text: &str| match text {
        "boolean" => Some(CellKind::Flag),
        "integer" => Some(CellKind::Number),
        "string" | "path" | "glob" => Some(CellKind::Symbol),
        _ => None,
    };
    match (scalar(declared), element.and_then(scalar)) {
        (Some(CellKind::Flag), _) => Some(ParameterType::Flag),
        (Some(CellKind::Number), _) => Some(ParameterType::Number),
        (Some(_), _) => Some(ParameterType::Text),
        (None, Some(CellKind::Number)) => Some(ParameterType::Numbers),
        (None, Some(CellKind::Symbol)) => Some(ParameterType::Texts),
        (None, Some(_) | None) => None,
    }
}
