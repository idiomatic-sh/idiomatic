//! The types of the `plans` concept: a compiled query: its parameters, its steps and what it emits. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::plan_parts::types::CapturePattern;
use crate::domain::plan_parts::types::ColumnIndex;
use crate::domain::plan_parts::types::Comparison;
use crate::domain::plan_parts::types::Direction;
use crate::domain::plan_parts::types::Heading;
use crate::domain::plan_parts::types::HostedNumber;
use crate::domain::plan_parts::types::Input;
use crate::domain::plan_parts::types::JoinColumn;
use crate::domain::plan_parts::types::Operand;
use crate::domain::plan_parts::types::ParameterId;
use crate::domain::plan_parts::types::ParameterName;
use crate::domain::plan_parts::types::RepeatedRows;
use crate::domain::plan_parts::types::StepId;
use crate::domain::plan_parts::types::TableKey;
use crate::domain::plan_parts::types::Test;
use crate::domain::program::types::QueryOrigin;

/// A compiled query: its parameters, its steps in the order to run them, and what it emits.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// The `param` lines.
    pub parameters: Vec<PlanParameter>,
    /// The steps, each reading only the relations and the steps before it.
    pub steps: Vec<PlanStep>,
    /// Source origins indexed by diagnostic-only comparison IDs.
    pub comparison_origins: Vec<QueryOrigin>,
    /// The `emit` line.
    pub emit: Emit,
    /// What the per-file steps hand to the cross-file steps: the relations and the per-file steps
    /// that a cross-file step reads. Empty when every step is per-file.
    pub frontier: Vec<Input>,
}

/// A parameter the host supplies when it runs the plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanParameter {
    /// The parameter's name, as the host supplies it: text that came from the query.
    pub name: ParameterName,
    /// What it holds.
    pub kind: ParameterType,
}

/// What a parameter holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterType {
    /// A yes or no.
    Flag,
    /// A whole number.
    Number,
    /// Text: a string, a path or a glob.
    Text,
    /// A list or a set of numbers.
    Numbers,
    /// A list or a set of text.
    Texts,
    /// A list of sets of text: the sets in order.
    TextSets,
    /// A list of tables of text, each a row: `list<table>`.
    Tables,
}

/// One operation of the plan.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanStep {
    /// What the step does.
    pub operation: Operation,
    /// Where it runs.
    pub scope: StepScope,
}

/// Where a step runs: in the worker that has the file, or once after the workers on the rows they
/// handed over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepScope {
    /// On one file's rows, in the worker.
    PerFile,
    /// On the frontier rows of every file.
    CrossFile,
}

/// The operations of the language, one variant each, with typed fields.
#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    /// Keep the rows for which every comparison holds.
    Select {
        /// What it filters.
        from: Input,
        /// The comparisons, all of which must hold.
        comparisons: Vec<Comparison>,
        /// The columns kept, in order; none named means every column.
        kept: Vec<ColumnIndex>,
    },
    /// Keep the columns at these places, dropping repeated rows.
    Project {
        /// What it projects.
        from: Input,
        /// The columns kept, in order.
        columns: Vec<ColumnIndex>,
        /// Whether repeated rows are dropped: not when a column kept says which row it is.
        distinct: RepeatedRows,
    },
    /// Append columns holding a number or a parameter.
    Extend {
        /// What it extends.
        from: Input,
        /// One operand for each new column.
        values: Vec<Operand>,
    },
    /// Combine rows with equal keys.
    Join {
        /// The left input.
        left: Input,
        /// The right input.
        right: Input,
        /// The left key columns.
        left_on: Vec<ColumnIndex>,
        /// The right key columns.
        right_on: Vec<ColumnIndex>,
        /// The columns of the answer.
        output: Vec<JoinColumn>,
        /// What the rows of the right input must satisfy to be joined, as the comparisons of a filter of it.
        filter: Vec<Comparison>,
        /// The columns of the right input the keys and the answer name, in order; none named means all of them.
        view: Vec<ColumnIndex>,
        /// Source-keyed closure demand; the right input is physical edges when present.
        reach: Option<crate::domain::plan_parts::types::ReachLookup>,
    },
    /// Rows in either input.
    Union {
        /// The left input.
        left: Input,
        /// The right input.
        right: Input,
    },
    /// Rows in both inputs.
    Intersection {
        /// The left input.
        left: Input,
        /// The right input.
        right: Input,
    },
    /// Left rows not in the right input.
    Difference {
        /// The left input.
        left: Input,
        /// The right input.
        right: Input,
    },
    /// Left rows with no match on the right.
    Antijoin {
        /// The left input.
        left: Input,
        /// The right input.
        right: Input,
        /// The left key columns.
        left_on: Vec<ColumnIndex>,
        /// The right key columns.
        right_on: Vec<ColumnIndex>,
    },
    /// Group rows and append a count.
    GroupCount {
        /// What it groups.
        from: Input,
        /// The columns that make a group.
        keys: Vec<ColumnIndex>,
        /// Count the distinct values of this column instead of the rows.
        distinct: Option<ColumnIndex>,
    },
    /// Group rows and append a digest of the set of distinct values of a column, and how many there are.
    GroupDigest {
        /// What it groups.
        from: Input,
        /// The columns that make a group.
        keys: Vec<ColumnIndex>,
        /// The column whose distinct values are digested.
        over: ColumnIndex,
    },
    /// Match paths against a pattern with slots and append what the slots matched.
    Capture {
        /// What it matches.
        from: Input,
        /// The column holding the path.
        path: ColumnIndex,
        /// The pattern, with `{slot}` where text is captured: text from the query.
        pattern: CapturePattern,
    },
    /// Append the place of the first of an ordered list of sets of text that holds the text of a column: the first set is 0. A
    /// row whose text no set holds is dropped.
    Rank {
        /// What it ranks.
        from: Input,
        /// The column holding the text.
        column: ColumnIndex,
        /// The parameter holding the sets, in order.
        sets: ParameterId,
    },
    /// Append text after literal replacement of every regex match.
    Replace {
        /// Input rows.
        from: Input,
        /// Text column to transform.
        text: ColumnIndex,
        /// Constant regular expression.
        pattern: crate::domain::plan_parts::types::LiteralText,
        /// Constant literal replacement text.
        replacement: crate::domain::plan_parts::types::LiteralText,
    },
    /// Consume the earliest later right row for each ordered left row in an equal-key group.
    OrderedMatch {
        /// Left rows.
        left: Input,
        /// Right rows.
        right: Input,
        /// Left equality keys.
        left_on: Vec<ColumnIndex>,
        /// Right equality keys.
        right_on: Vec<ColumnIndex>,
        /// Unique numeric order within each left group.
        left_order: ColumnIndex,
        /// Unique numeric order within each right group.
        right_order: ColumnIndex,
        /// Explicit output projection.
        output: Vec<JoinColumn>,
    },
    /// Split a text column into words, one row for each.
    Words {
        /// What it splits.
        from: Input,
        /// The column holding the text.
        text: ColumnIndex,
    },
    /// Walk a hierarchy from each row's entity.
    Axis {
        /// The rows to start from.
        from: Input,
        /// The parent and child edges.
        edges: Input,
        /// The rows proving valid answers.
        domain: Input,
        /// Which way to walk.
        direction: Direction,
        /// The starting entity's column in `from`.
        id: ColumnIndex,
        /// The parent column in `edges`.
        parent: ColumnIndex,
        /// The child column in `edges`.
        child: ColumnIndex,
        /// The entity column in `domain`.
        domain_id: ColumnIndex,
        /// What the rows of the domain must satisfy to prove an answer valid, as the comparisons of a filter of it.
        domain_filter: Vec<Comparison>,
        /// The columns of the domain the entity column names, in order; none named means all of them.
        domain_view: Vec<ColumnIndex>,
        /// The columns of the answer, each of the starting row (left) or of the element reached (right); none named means the
        /// starting row and the entity reached after it.
        output: Vec<JoinColumn>,
    },
    /// Every pair joined by a chain of edges.
    Closure {
        /// The edges.
        edges: Input,
        /// Independent graph partitions, in output order; empty means one graph.
        keys: Vec<ColumnIndex>,
        /// The source column.
        source: ColumnIndex,
        /// The destination column.
        destination: ColumnIndex,
    },
    /// Ordered depth-first edge visits for each distinct partition/root.
    OrderedDfs {
        /// Root input.
        from: Input,
        /// Edge input.
        edges: Input,
        /// Root entity column.
        root: ColumnIndex,
        /// Source entity column of edges.
        source: ColumnIndex,
        /// Destination entity column of edges.
        destination: ColumnIndex,
        /// Numeric outgoing-edge order column.
        order: ColumnIndex,
        /// Edge identity column, preserved on each visit.
        edge: ColumnIndex,
        /// Partition columns of roots, in output order.
        root_keys: Vec<ColumnIndex>,
        /// Matching partition columns of edges.
        edge_keys: Vec<ColumnIndex>,
    },
    /// One row for each entity that reaches itself.
    Cycles {
        /// The edges.
        edges: Input,
        /// The source column.
        source: ColumnIndex,
        /// The destination column.
        destination: ColumnIndex,
    },
    /// The rows a `list<table>` parameter holds, one for each table, a column for each key: the same for every file.
    Rows {
        /// The parameter holding the tables.
        parameter: ParameterId,
        /// The keys whose values fill the columns, in order.
        keys: Vec<TableKey>,
    },
    /// The rows of a relation the host declared: they come with the run, and are the same for every file.
    Hosted {
        /// The relation, by its place among the ones declared.
        relation: HostedNumber,
    },
    /// Append a span from the first input's start to the second input's end.
    SpanBetween {
        /// Input rows in one file.
        from: Input,
        /// Span supplying the start offset.
        start: ColumnIndex,
        /// Span supplying the end offset.
        end: ColumnIndex,
    },
    /// Emit each regular-expression occurrence in a physical source span.
    SourceMatches {
        /// Per-file input rows.
        from: Input,
        /// Physical source span to search.
        span: ColumnIndex,
        /// Optional one-based line of the input span's first byte.
        base_line: Option<ColumnIndex>,
        /// Authored regular expression.
        pattern: crate::domain::plan_parts::types::LiteralText,
    },
    /// Append checked byte offsets of an existing physical source span.
    SpanBounds {
        /// Per-file input rows.
        from: Input,
        /// Span whose endpoints are projected.
        span: ColumnIndex,
    },
}

/// The kinds of operation the language has, one variant each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperationKind {
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
    /// `span-bounds`.
    SpanBounds,
    /// Ordered depth-first edge visits.
    OrderedDfs,
}

/// Something of the language a query uses, which a run must be shown to exercise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Feature {
    /// An operation.
    Operation(OperationKind),
    /// A way of comparing, in a comparison of a `select`, a `join` filter or a domain filter.
    Family(Test),
    /// A direction of an `axis`.
    Direction(Direction),
    /// A relation a step reads.
    Relation(ryo_language::domain::relations::types::Relation),
}

/// What the query emits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Emit {
    /// The step whose rows answer the query.
    pub step: StepId,
    /// The column holding the file a row is about.
    pub subject: ColumnIndex,
    /// The columns that identify a row.
    pub key: Vec<ColumnIndex>,
    /// The headings of the answer's columns: text that leaves the engine.
    pub headings: Vec<Heading>,
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
    /// `span-bounds(from, span, start-output, end-output)`.
    SpanBounds,
    /// `ordered-dfs(from, root, edges, source, destination, order, edge, keys, output)`.
    OrderedDfs,
}
