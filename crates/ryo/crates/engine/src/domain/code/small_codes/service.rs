//! The services of the `small_codes` part: each small code written from its value by a `match` that names every variant, and
//! read back by its number. A number no value has reads as none: a pack's bytes are never trusted to name one.

use crate::domain::code::types::{Short, WordFlag};
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::{AxisDirection, CallMeaning, RegexId, StepRows, TestOperation};
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::relations::types::NodeColumn;
use ryo_values::domain::plan_orders::types::OrderBound;

/// The code of a node column.
#[must_use]
pub(crate) const fn node_column_code(column: NodeColumn) -> Short {
    Short(match column {
        NodeColumn::Id => 0,
        NodeColumn::Kind => 1,
        NodeColumn::Parent => 2,
        NodeColumn::Field => 3,
        NodeColumn::Ordinal => 4,
        NodeColumn::Start => 5,
        NodeColumn::End => 6,
        NodeColumn::Span => 7,
        NodeColumn::File => 8,
        NodeColumn::Text => 9,
    })
}

/// The node column of a code.
#[must_use]
pub(crate) const fn node_column_of(code: Short) -> Option<NodeColumn> {
    Some(match code.0 {
        0 => NodeColumn::Id,
        1 => NodeColumn::Kind,
        2 => NodeColumn::Parent,
        3 => NodeColumn::Field,
        4 => NodeColumn::Ordinal,
        5 => NodeColumn::Start,
        6 => NodeColumn::End,
        7 => NodeColumn::Span,
        8 => NodeColumn::File,
        9 => NodeColumn::Text,
        10..=u16::MAX => return None,
    })
}

/// The code of a comparison; the regular expression of a `Matches` is the word's own operand.
#[must_use]
pub(crate) const fn comparison_code(test: TestOperation) -> Short {
    Short(match test {
        TestOperation::Equal => 0,
        TestOperation::NotEqual => 1,
        TestOperation::Less => 2,
        TestOperation::LessOrEqual => 3,
        TestOperation::Greater => 4,
        TestOperation::GreaterOrEqual => 5,
        TestOperation::In => 6,
        TestOperation::NotIn => 7,
        TestOperation::Matches(_) => 8,
        TestOperation::Glob => 9,
        TestOperation::NotGlob => 10,
        TestOperation::StartsWith => 11,
        TestOperation::EndsWith => 12,
        TestOperation::NumericAtLeast => 13,
        TestOperation::NaturalLess => 14,
        TestOperation::Successor => 15,
    })
}

/// The comparison of a code, with the word's regular expression.
#[must_use]
pub(crate) const fn comparison_of(code: Short, regex: RegexId) -> Option<TestOperation> {
    Some(match code.0 {
        0 => TestOperation::Equal,
        1 => TestOperation::NotEqual,
        2 => TestOperation::Less,
        3 => TestOperation::LessOrEqual,
        4 => TestOperation::Greater,
        5 => TestOperation::GreaterOrEqual,
        6 => TestOperation::In,
        7 => TestOperation::NotIn,
        8 => TestOperation::Matches(regex),
        9 => TestOperation::Glob,
        10 => TestOperation::NotGlob,
        11 => TestOperation::StartsWith,
        12 => TestOperation::EndsWith,
        13 => TestOperation::NumericAtLeast,
        14 => TestOperation::NaturalLess,
        15 => TestOperation::Successor,
        16..=u16::MAX => return None,
    })
}

/// The regular expression a comparison names, when it names one.
#[must_use]
pub(crate) const fn comparison_regex(test: TestOperation) -> Option<RegexId> {
    match test {
        TestOperation::Matches(regex) => Some(regex),
        TestOperation::Equal
        | TestOperation::NotEqual
        | TestOperation::Less
        | TestOperation::LessOrEqual
        | TestOperation::Greater
        | TestOperation::GreaterOrEqual
        | TestOperation::In
        | TestOperation::NotIn
        | TestOperation::Glob
        | TestOperation::NotGlob
        | TestOperation::StartsWith
        | TestOperation::EndsWith
        | TestOperation::NumericAtLeast
        | TestOperation::NaturalLess
        | TestOperation::Successor => None,
    }
}

/// The code of a per-row meaning; the regular expression of one that has one is the word's own operand.
#[must_use]
pub(crate) const fn meaning_code(meaning: CallMeaning) -> Short {
    Short(match meaning {
        CallMeaning::SpanBetween => 0,
        CallMeaning::SpanBounds => 1,
        CallMeaning::SourceMatches(_) => 2,
        CallMeaning::Capture => 3,
        CallMeaning::Replace(_) => 4,
        CallMeaning::Words => 5,
        CallMeaning::Rank => 6,
        CallMeaning::Concat => 7,
        CallMeaning::Lower => 8,
        CallMeaning::FitsRendered => 9,
    })
}

/// The per-row meaning of a code, with the word's regular expression.
#[must_use]
pub(crate) const fn meaning_of(code: Short, regex: RegexId) -> Option<CallMeaning> {
    Some(match code.0 {
        0 => CallMeaning::SpanBetween,
        1 => CallMeaning::SpanBounds,
        2 => CallMeaning::SourceMatches(regex),
        3 => CallMeaning::Capture,
        4 => CallMeaning::Replace(regex),
        5 => CallMeaning::Words,
        6 => CallMeaning::Rank,
        7 => CallMeaning::Concat,
        8 => CallMeaning::Lower,
        9 => CallMeaning::FitsRendered,
        10..=u16::MAX => return None,
    })
}

/// The regular expression a per-row meaning names, when it names one.
#[must_use]
pub(crate) const fn meaning_regex(meaning: CallMeaning) -> Option<RegexId> {
    match meaning {
        CallMeaning::SourceMatches(regex) | CallMeaning::Replace(regex) => Some(regex),
        CallMeaning::SpanBetween
        | CallMeaning::SpanBounds
        | CallMeaning::Capture
        | CallMeaning::Words
        | CallMeaning::Rank
        | CallMeaning::Concat
        | CallMeaning::Lower
        | CallMeaning::FitsRendered => None,
    }
}

/// The code of a render context.
#[must_use]
pub(crate) const fn context_code(context: RenderContext) -> Short {
    Short(match context {
        RenderContext::AssignmentTarget => 0,
        RenderContext::AttributeReceiver => 1,
        RenderContext::PositionalArgument => 2,
    })
}

/// The render context of a code.
#[must_use]
pub(crate) const fn context_of(code: Short) -> Option<RenderContext> {
    Some(match code.0 {
        0 => RenderContext::AssignmentTarget,
        1 => RenderContext::AttributeReceiver,
        2 => RenderContext::PositionalArgument,
        3..=u16::MAX => return None,
    })
}

/// The code of an axis direction: 0 to 4.
#[must_use]
pub(crate) const fn direction_code(direction: AxisDirection) -> WordFlag {
    WordFlag(match direction {
        AxisDirection::Parent => 0,
        AxisDirection::Ancestor => 1,
        AxisDirection::Descendant => 2,
        AxisDirection::NextSibling => 3,
        AxisDirection::PreviousSibling => 4,
    })
}

/// The axis direction of a code.
#[must_use]
pub(crate) const fn direction_of(code: WordFlag) -> Option<AxisDirection> {
    Some(match code.0 {
        0 => AxisDirection::Parent,
        1 => AxisDirection::Ancestor,
        2 => AxisDirection::Descendant,
        3 => AxisDirection::NextSibling,
        4 => AxisDirection::PreviousSibling,
        5..=u8::MAX => return None,
    })
}

/// The code of the side of a bound: 0 to 3.
#[must_use]
pub(crate) const fn side_code(side: OrderBound) -> WordFlag {
    WordFlag(match side {
        OrderBound::Before => 0,
        OrderBound::AtOrBefore => 1,
        OrderBound::After => 2,
        OrderBound::AtOrAfter => 3,
    })
}

/// The side of a bound of a code.
#[must_use]
pub(crate) const fn side_of(code: WordFlag) -> Option<OrderBound> {
    Some(match code.0 {
        0 => OrderBound::Before,
        1 => OrderBound::AtOrBefore,
        2 => OrderBound::After,
        3 => OrderBound::AtOrAfter,
        4..=u8::MAX => return None,
    })
}

/// The code of what a probe or a use does with what it finds: 0 to 2.
#[must_use]
pub(crate) const fn mode_code(mode: UseMode) -> WordFlag {
    WordFlag(match mode {
        UseMode::Rows => 0,
        UseMode::Present => 1,
        UseMode::Absent => 2,
    })
}

/// What a probe or a use does with what it finds, of a code.
#[must_use]
pub(crate) const fn mode_of(code: WordFlag) -> Option<UseMode> {
    Some(match code.0 {
        0 => UseMode::Rows,
        1 => UseMode::Present,
        2 => UseMode::Absent,
        3..=u8::MAX => return None,
    })
}

/// The code of where a stage reads its rows: 0 the node table, 1 a filled relation, 2 a made table, 3 a written one. A
/// definition's rows are its table's.
#[must_use]
pub(crate) const fn source_code(source: StepRows) -> WordFlag {
    WordFlag(match source {
        StepRows::Node(_) => 0,
        StepRows::Filled(_) => 1,
        StepRows::Made(_) | StepRows::Definition(_) => 2,
        StepRows::Written(_) => 3,
    })
}
