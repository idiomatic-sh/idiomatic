//! The names of the `syntax` concept: the one place where an operation or an argument is mapped
//! to the text a query writes and back. Text enters the engine here and becomes a variant.

use ryo_relations::domain::relations::types::CellKind;

use super::types::ArgumentSlot;
use ryo_values::domain::plan_orders::types::OrderBound;
use ryo_values::domain::plan_parts::types::Direction;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::query_bindings::types::ParameterType;
use ryo_values::domain::query_operators::constants::OPERATORS;
use ryo_values::domain::query_operators::types::QueryOperator;

/// The text of the relation argument, and of the relation keyword.
pub(crate) const RELATION: &str = ryo_values::domain::descriptions::constants::RELATION;

/// The text of the destination argument.
pub(crate) const DESTINATION: &str = ryo_values::domain::descriptions::constants::DESTINATION;

/// The word a declaration writes for a flag.
pub(crate) const BOOLEAN: &str = "boolean";

/// The text a query writes for an operator.
#[must_use]
pub(crate) fn operator_name(operator: QueryOperator) -> &'static str {
    match operator {
        QueryOperator::SpanBetween => "span-between",
        QueryOperator::SpanBounds => "span-bounds",
        QueryOperator::SourceMatches => "source-matches",
        QueryOperator::RenderSyntax => "render-syntax",
        QueryOperator::FitsRendered => "fits-rendered",
        QueryOperator::Replace => "replace",
        QueryOperator::OrderedMatch => "ordered-match",
        QueryOperator::AsOf => "as-of",
        QueryOperator::OrderedDfs => "ordered-dfs",
        QueryOperator::Worklist => "worklist",
        QueryOperator::Select => ryo_values::domain::descriptions::constants::SELECT,
        QueryOperator::Project => ryo_values::domain::descriptions::constants::PROJECT,
        QueryOperator::Extend => "extend",
        QueryOperator::Join => "join",
        QueryOperator::Union => "union",
        QueryOperator::Intersection => "intersection",
        QueryOperator::Difference => "difference",
        QueryOperator::Antijoin => "antijoin",
        QueryOperator::GroupCount => "group-count",
        QueryOperator::GroupDigest => "group-digest",
        QueryOperator::GroupMin => "group-min",
        QueryOperator::GroupMax => "group-max",
        QueryOperator::GroupOrdinal => "group-ordinal",
        QueryOperator::Capture => "capture",
        QueryOperator::Words => "words",
        QueryOperator::Rank => "rank",
        QueryOperator::Axis => "axis",
        QueryOperator::Rows => "rows",
        QueryOperator::Closure => "closure",
        QueryOperator::Follow => "follow",
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
        ArgumentSlot::Node => "node",
        ArgumentSlot::Context => ryo_relations::domain::relations::constants::CONTEXT,
        ArgumentSlot::MaxLine => "max-line",
        ArgumentSlot::TabWidth => "tab-width",
        ArgumentSlot::Texts => "texts",
        ArgumentSlot::Literals => "literals",
        ArgumentSlot::Start => "start",
        ArgumentSlot::End => "end",
        ArgumentSlot::Span => "span",
        ArgumentSlot::StartOutput => "start-output",
        ArgumentSlot::EndOutput => "end-output",
        ArgumentSlot::BaseLine => "base-line",
        ArgumentSlot::LineOutput => "line-output",
        ArgumentSlot::CaptureName => "capture",
        ArgumentSlot::CaptureOutput => "capture-output",
        ArgumentSlot::CaptureTextOutput => "capture-text-output",
        ArgumentSlot::CaptureLineOutput => "capture-line-output",
        ArgumentSlot::Replacement => ryo_values::domain::descriptions::constants::REPLACEMENT,
        ArgumentSlot::LeftOrder => "left-order",
        ArgumentSlot::RightOrder => "right-order",
        ArgumentSlot::Bound => "bound",
        ArgumentSlot::From => "from",
        ArgumentSlot::Root => "root",
        ArgumentSlot::Order => "order",
        ArgumentSlot::Edge => "edge",
        ArgumentSlot::State => "state",
        ArgumentSlot::Before => "before",
        ArgumentSlot::After => "after",
        ArgumentSlot::Lattice => "lattice",
        ArgumentSlot::Lower => "lower",
        ArgumentSlot::Upper => "upper",
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
        ArgumentSlot::Relation => RELATION,
        ArgumentSlot::Direction => "direction",
        ArgumentSlot::Domain => "domain",
        ArgumentSlot::Edges => "edges",
        ArgumentSlot::Source => ryo_relations::domain::relations::constants::SOURCE,
        ArgumentSlot::Destination => DESTINATION,
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
        Direction::Parent => ryo_relations::domain::relations::constants::PARENT,
        Direction::Child => "child",
        Direction::Ancestor => "ancestor",
        Direction::Descendant => "descendant",
        Direction::NearestAncestor => "nearest-ancestor",
        Direction::Deep => "deep",
        Direction::PreviousSibling => "previous-sibling",
        Direction::NextSibling => "next-sibling",
    }
}

/// The bound of an `as-of` a query calls `text`.
#[must_use]
pub(crate) fn bound_named(text: &str) -> Option<OrderBound> {
    [
        OrderBound::Before,
        OrderBound::AtOrBefore,
        OrderBound::After,
        OrderBound::AtOrAfter,
    ]
    .into_iter()
    .find(|bound| bound_name(*bound) == text)
}

/// The text a query writes for the bound of an `as-of`.
#[must_use]
pub(crate) fn bound_name(bound: OrderBound) -> &'static str {
    match bound {
        OrderBound::Before => "before",
        OrderBound::AtOrBefore => "at-or-before",
        OrderBound::After => "after",
        OrderBound::AtOrAfter => "at-or-after",
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
        BOOLEAN => Some(CellKind::Flag),
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

/// The scalar kinds accepted by fragment declarations (distinct from root parameter syntax).
#[must_use]
pub(crate) fn fragment_scalar_kind_named(declared: &str) -> Option<ParameterType> {
    match declared {
        "string" | "text" => Some(ParameterType::Text),
        "integer" | ryo_relations::domain::relations::constants::NUMBER => {
            Some(ParameterType::Number)
        }
        BOOLEAN => Some(ParameterType::Flag),
        _ => None,
    }
}

/// The column kinds accepted by fragment relation signatures.
#[must_use]
pub(crate) fn relation_cell_kind_named(declared: &str) -> Option<CellKind> {
    match declared {
        "node" => Some(CellKind::Node),
        "symbol" | "text" => Some(CellKind::Symbol),
        "file" => Some(CellKind::File),
        "span" => Some(CellKind::Span),
        ryo_relations::domain::relations::constants::NUMBER | "integer" => Some(CellKind::Number),
        "flag" | BOOLEAN => Some(CellKind::Flag),
        _ => None,
    }
}

/// The two literal words that provide a fragment boolean argument.
pub(crate) const BOOLEAN_WORDS: [&str; 2] = ["true", "false"];

/// Written render context for assignment target.
pub(crate) const ASSIGNMENT_TARGET: &str = "assignment-target";

/// Written render context for attribute receiver.
pub(crate) const ATTRIBUTE_RECEIVER: &str = "attribute-receiver";

/// Written render context for positional argument.
pub(crate) const POSITIONAL_ARGUMENT: &str = "positional-argument";
