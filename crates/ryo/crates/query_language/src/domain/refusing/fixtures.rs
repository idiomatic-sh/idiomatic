//! The steps the `refusing` cases build: each kind of step that compares rows on a span, with and without a file key.

use crate::domain::lowering::types::{PlannedColumn, Uniqueness};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;

/// A step that compares rows on a span, by kind.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Compared {
    /// Join.
    Join,
    /// Antijoin.
    Antijoin,
    /// Ordered match.
    OrderedMatch,
    /// As-of.
    AsOf,
    /// Group count.
    GroupCount,
    /// Group digest.
    GroupDigest,
    /// Group bound.
    GroupBound,
    /// Group ordinal.
    GroupOrdinal,
    /// Closure.
    Closure,
    /// Follow.
    Follow,
    /// Ordered dfs.
    OrderedDfs,
    /// Cycles.
    Cycles,
    /// Union.
    Union,
    /// Intersection.
    Intersection,
    /// Difference.
    Difference,
    /// Distinct.
    Distinct,
}

/// Every kind of step that compares rows on a span.
pub(crate) const COMPARED: [Compared; 16] = [
    Compared::Join,
    Compared::Antijoin,
    Compared::OrderedMatch,
    Compared::AsOf,
    Compared::GroupCount,
    Compared::GroupDigest,
    Compared::GroupBound,
    Compared::GroupOrdinal,
    Compared::Closure,
    Compared::Follow,
    Compared::OrderedDfs,
    Compared::Cycles,
    Compared::Union,
    Compared::Intersection,
    Compared::Difference,
    Compared::Distinct,
];

/// A planned column of this kind.
pub(crate) fn column(kind: CellKind) -> PlannedColumn {
    PlannedColumn {
        name: ryo_values::domain::descriptions::types::ColumnName("c".into()),
        kind,
        unique: Uniqueness::Shared,
        words: ryo_relations::domain::terms::types::Vocabulary::Open,
    }
}

/// The step of this kind over `rows`, comparing them on the span at `span`; `other` is a column it reads besides.
pub(crate) fn compared(
    kind: Compared,
    rows: parts::Input,
    span: parts::ColumnIndex,
    other: parts::ColumnIndex,
) -> plans::Operation {
    match kind {
        Compared::Join => plans::Operation::Join {
            left: rows,
            right: rows,
            left_on: vec![span],
            right_on: vec![span],
            output: vec![],
            filter: vec![],
            view: vec![],
            reach: None,
        },
        Compared::Antijoin => plans::Operation::Antijoin {
            left: rows,
            right: rows,
            left_on: vec![span],
            right_on: vec![span],
        },
        Compared::OrderedMatch => plans::Operation::OrderedMatch {
            left: rows,
            right: rows,
            left_on: vec![span],
            right_on: vec![span],
            left_order: other,
            right_order: other,
            output: vec![],
        },
        Compared::AsOf => plans::Operation::AsOf {
            left: rows,
            right: rows,
            left_on: vec![span],
            right_on: vec![span],
            left_order: other,
            right_order: other,
            bound: ryo_values::domain::plan_orders::types::OrderBound::Before,
            output: vec![],
        },
        Compared::GroupBound => plans::Operation::GroupBound {
            from: rows,
            keys: vec![span],
            order: other,
            end: ryo_values::domain::plan_orders::types::GroupEnd::Greatest,
        },
        Compared::GroupOrdinal => plans::Operation::GroupOrdinal {
            from: rows,
            keys: vec![span],
            order: other,
        },
        Compared::Follow => plans::Operation::Follow {
            edges: rows,
            keys: vec![],
            source: span,
            destination: span,
        },
        Compared::GroupCount => plans::Operation::GroupCount {
            from: rows,
            keys: vec![span],
            distinct: None,
        },
        Compared::GroupDigest => plans::Operation::GroupDigest {
            from: rows,
            keys: vec![span],
            over: other,
        },
        Compared::Closure => plans::Operation::Closure {
            edges: rows,
            keys: vec![],
            source: span,
            destination: span,
        },
        Compared::OrderedDfs => plans::Operation::OrderedDfs {
            from: rows,
            edges: rows,
            root: span,
            source: span,
            destination: span,
            order: other,
            edge: other,
            root_keys: vec![],
            edge_keys: vec![],
        },
        Compared::Cycles => plans::Operation::Cycles {
            edges: rows,
            source: span,
            destination: span,
        },
        Compared::Union => plans::Operation::Union {
            left: rows,
            right: rows,
        },
        Compared::Intersection => plans::Operation::Intersection {
            left: rows,
            right: rows,
        },
        Compared::Difference => plans::Operation::Difference {
            left: rows,
            right: rows,
        },
        Compared::Distinct => plans::Operation::Project {
            from: rows,
            columns: vec![span],
            distinct: parts::RepeatedRows::Dropped,
        },
    }
}

/// The step of this kind over `rows`, comparing them on the span at `span` with the file at `file` beside it; `other` is a column
/// it reads besides. Cycles compares one column, its source, so its file key is that column.
pub(crate) fn compared_with_file(
    kind: Compared,
    rows: parts::Input,
    span: parts::ColumnIndex,
    file: parts::ColumnIndex,
    other: parts::ColumnIndex,
) -> plans::Operation {
    match kind {
        Compared::Join => plans::Operation::Join {
            left: rows,
            right: rows,
            left_on: vec![span, file],
            right_on: vec![span, file],
            output: vec![],
            filter: vec![],
            view: vec![],
            reach: None,
        },
        Compared::Antijoin => plans::Operation::Antijoin {
            left: rows,
            right: rows,
            left_on: vec![span, file],
            right_on: vec![span, file],
        },
        Compared::OrderedMatch => plans::Operation::OrderedMatch {
            left: rows,
            right: rows,
            left_on: vec![span, file],
            right_on: vec![span, file],
            left_order: other,
            right_order: other,
            output: vec![],
        },
        Compared::AsOf => plans::Operation::AsOf {
            left: rows,
            right: rows,
            left_on: vec![span, file],
            right_on: vec![span, file],
            left_order: other,
            right_order: other,
            bound: ryo_values::domain::plan_orders::types::OrderBound::Before,
            output: vec![],
        },
        Compared::GroupBound => plans::Operation::GroupBound {
            from: rows,
            keys: vec![span, file],
            order: other,
            end: ryo_values::domain::plan_orders::types::GroupEnd::Greatest,
        },
        Compared::GroupOrdinal => plans::Operation::GroupOrdinal {
            from: rows,
            keys: vec![span, file],
            order: other,
        },
        Compared::Follow => plans::Operation::Follow {
            edges: rows,
            keys: vec![file],
            source: span,
            destination: span,
        },
        Compared::GroupCount => plans::Operation::GroupCount {
            from: rows,
            keys: vec![span, file],
            distinct: None,
        },
        Compared::GroupDigest => plans::Operation::GroupDigest {
            from: rows,
            keys: vec![span, file],
            over: other,
        },
        Compared::Closure => plans::Operation::Closure {
            edges: rows,
            keys: vec![file],
            source: span,
            destination: span,
        },
        Compared::OrderedDfs => plans::Operation::OrderedDfs {
            from: rows,
            edges: rows,
            root: span,
            source: span,
            destination: span,
            order: other,
            edge: other,
            root_keys: vec![file],
            edge_keys: vec![file],
        },
        Compared::Cycles => plans::Operation::Cycles {
            edges: rows,
            source: file,
            destination: file,
        },
        Compared::Union | Compared::Intersection | Compared::Difference => {
            compared(kind, rows, span, other)
        }
        Compared::Distinct => plans::Operation::Project {
            from: rows,
            columns: vec![span, file],
            distinct: parts::RepeatedRows::Dropped,
        },
    }
}
