//! The types of the `query_operators` concept: the operators of the engine and the output choices of their steps. Plain data, public fields and derives, with no constructor and no inherent impl.

/// Whether the source-matches step appends the captured physical text as a Symbol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TextOutput {
    /// The text is appended.
    Appended,
    /// The text is left out.
    Omitted,
}

/// Whether the source-matches step appends the captured start line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LineOutput {
    /// The line is appended.
    Appended,
    /// The line is left out.
    Omitted,
}

/// The operators of the engine, one variant each, in the order the documentation lists them: the `select` sugar and every call a
/// query can use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QueryOperator {
    /// Literal regular-expression replacement.
    Replace,
    /// Ordered consumption of right rows.
    OrderedMatch,
    /// `select ... from ... where ...`.
    Select,
    /// `project(from, columns)`.
    Project,
    /// `extend(from, values)`.
    Extend,
    /// `join(left, right, left-on, right-on, output)`.
    Join,
    /// `union(left, right)`.
    Union,
    /// `intersection(left, right)`.
    Intersection,
    /// `difference(left, right)`.
    Difference,
    /// `antijoin(left, right, left-on, right-on)`.
    Antijoin,
    /// `group-count(from, keys, distinct, count)`.
    GroupCount,
    /// `group-digest(from, keys, distinct, digest, count)`.
    GroupDigest,
    /// `capture(from, path, pattern, captures)`.
    Capture,
    /// `words(from, text, word)`.
    Words,
    /// `rank(from, column, sets, output)`.
    Rank,
    /// `axis(from, relation, direction, domain)`.
    Axis,
    /// The rows a `list<table>` parameter holds.
    Rows,
    /// `closure(edges, source, destination)`.
    Closure,
    /// `cycles(edges, source, destination)`.
    Cycles,
    /// `span-between(from, start, end, output)`.
    SpanBetween,
    /// `source-matches(from, span, pattern, output)`.
    SourceMatches,
    /// `render-syntax(from, node, context, output)`.
    RenderSyntax,
    /// `fits-rendered(from, text, span, max-line, tab-width)`.
    FitsRendered,
    /// `span-bounds(from, span, start-output, end-output)`.
    SpanBounds,
    /// `ordered-dfs(from, root, edges, source, destination, order, edge, keys, output)`.
    OrderedDfs,
}

/// Maximum physical display columns of a source line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MaxLine(pub i64);

/// Physical tab stop width in display columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TabWidth(pub u32);

/// Keep the feature catalog's variants generated from the same declaration as
/// the kind itself. Adding a kind cannot silently omit it from `ALL`.
macro_rules! operation_kinds {
    ($( $(#[$meta:meta])* $variant:ident, )+) => {
        /// The kinds of operation the language has, one variant each.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum OperationKind {
            $( $(#[$meta])* $variant, )+
        }

        impl OperationKind {
            /// Every declared operation kind, in its stable catalog order.
            pub const ALL: &'static [Self] = &[ $( Self::$variant, )+ ];
        }
    };
}

operation_kinds! {
    /// Literal regular-expression replacement.
    Replace,
    /// Ordered consumption of right rows.
    OrderedMatch,
    /// `select`.
    Select,
    /// `project`.
    Project,
    /// `extend`.
    Extend,
    /// `join`.
    Join,
    /// `union`.
    Union,
    /// `intersection`.
    Intersection,
    /// `difference`.
    Difference,
    /// `antijoin`.
    Antijoin,
    /// `group-count`.
    GroupCount,
    /// `group-digest`.
    GroupDigest,
    /// `capture`.
    Capture,
    /// `rank`.
    Rank,
    /// `words`.
    Words,
    /// `axis`.
    Axis,
    /// `closure`.
    Closure,
    /// `cycles`.
    Cycles,
    /// The rows a `list<table>` parameter holds.
    Rows,
    /// The rows of a relation the host declared.
    Hosted,
    /// `span-between`.
    SpanBetween,
    /// `source-matches`.
    SourceMatches,
    /// Candidate-driven semantic syntax rendering.
    RenderSyntax,
    /// Physical display-width admission for rendered text.
    FitsRendered,
    /// `span-bounds`.
    SpanBounds,
    /// Ordered depth-first edge visits.
    OrderedDfs,
}
