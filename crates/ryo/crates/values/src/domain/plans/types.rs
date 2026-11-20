//! The types of the `plans` concept: the steps of a compiled query and the operations they are. Plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::plan_orders::types::{GroupEnd, OrderBound};
use crate::domain::plan_parts::types::CapturePattern;
use crate::domain::plan_parts::types::ColumnIndex;
use crate::domain::plan_parts::types::Direction;
use crate::domain::plan_parts::types::HostedNumber;
use crate::domain::plan_parts::types::Input;
use crate::domain::plan_parts::types::JoinColumn;
use crate::domain::plan_parts::types::Operand;
use crate::domain::plan_parts::types::ParameterId;
use crate::domain::plan_parts::types::PlanComparison;
use crate::domain::plan_parts::types::RepeatedRows;
use crate::domain::plan_parts::types::StepScope;
use crate::domain::plan_parts::types::TableKey;
use crate::domain::query_operators::types::{LineOutput, MaxLine, TabWidth, TextOutput};
use crate::domain::written_rows::types::RowsSource;

/// One operation of the plan.
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct PlanStep {
    /// What the step does.
    pub operation: Operation,
    /// Where it runs.
    pub scope: StepScope,
}

/// The operations of the language, one variant each, with typed fields.
#[derive(Clone, Debug, PartialEq, Hash)]
pub enum Operation {
    /// Keep the rows for which every comparison holds.
    Select {
        /// What it filters.
        from: Input,
        /// The comparisons, all of which must hold.
        comparisons: Vec<PlanComparison>,
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
        filter: Vec<PlanComparison>,
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
    /// Pair each left row with the right rows of its key group nearest to it by a number order; rows tied for nearest all pair.
    AsOf {
        /// Left rows.
        left: Input,
        /// Right rows.
        right: Input,
        /// Left equality keys; none means one group.
        left_on: Vec<ColumnIndex>,
        /// Right equality keys.
        right_on: Vec<ColumnIndex>,
        /// The left number order column.
        left_order: ColumnIndex,
        /// The right number order column.
        right_order: ColumnIndex,
        /// Which right rows are nearest: below or above the left order, strictly or not.
        bound: OrderBound,
        /// Explicit output projection.
        output: Vec<JoinColumn>,
    },
    /// Keep each key group's rows at one end of a number order; rows tied there are all kept.
    GroupBound {
        /// What it groups.
        from: Input,
        /// The columns that make a group; none means one group.
        keys: Vec<ColumnIndex>,
        /// The number order column.
        order: ColumnIndex,
        /// Which end is kept: the least or the greatest.
        end: GroupEnd,
    },
    /// Append to each distinct row how many distinct rows of its key group have a strictly smaller number order.
    GroupOrdinal {
        /// What it groups.
        from: Input,
        /// The columns that make a group; none means one group.
        keys: Vec<ColumnIndex>,
        /// The number order column.
        order: ColumnIndex,
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
        domain_filter: Vec<PlanComparison>,
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
    /// Each source with every entity it reaches over edges that has an edge to itself.
    Follow {
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
    /// A first-in-first-out worklist fixpoint over a graph, one run for each distinct partition and root: a state for each
    /// entity from a declared lattice, the edges whose before-state an entity holds when it leaves the queue each joining their
    /// after-state into their destination, and one row for each edge so followed.
    Worklist {
        /// The root rows: where each run starts, and in what state.
        from: Input,
        /// The edge rows: source, destination, order, and the before-state and after-state of each.
        edges: Input,
        /// The lattice rows: each a lower state and an upper state above it.
        lattice: Input,
        /// Root entity column of `from`.
        root: ColumnIndex,
        /// The root's starting state column of `from`.
        state: ColumnIndex,
        /// Source entity column of `edges`.
        source: ColumnIndex,
        /// Destination entity column of `edges`.
        destination: ColumnIndex,
        /// Numeric order column of `edges`: a source's edges are followed in ascending order.
        order: ColumnIndex,
        /// The state column of `edges` the source must hold for the edge to be followed.
        before: ColumnIndex,
        /// The state column of `edges` the edge joins into its destination.
        after: ColumnIndex,
        /// The lower state column of `lattice`.
        lower: ColumnIndex,
        /// The upper state column of `lattice`.
        upper: ColumnIndex,
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
    /// The rows a `list<table>` parameter holds, one for each table, or the rows the query wrote: a column for each key, the same for every file.
    Rows {
        /// Where the rows come from: the parameter holding the tables, or the rows written.
        source: RowsSource,
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
        /// Optional required named capture; a nonparticipating capture emits no row.
        capture: Option<crate::domain::plan_parts::types::LiteralText>,
        /// Append the captured physical text as a source-backed Symbol.
        capture_text: TextOutput,
        /// Append the captured start line after any whole-match line output.
        capture_line: LineOutput,
    },
    /// Render candidate syntax in a declared embedding role on demand.
    RenderSyntax {
        /// Per-file candidate rows.
        from: Input,
        /// The node whose syntax is needed.
        node: ColumnIndex,
        /// The role in which the node will be embedded.
        context: ryo_language::domain::semantics::types::RenderContext,
    },
    /// Keep rows whose rendered candidate fits the source's physical line width.
    FitsRendered {
        /// Per-file candidate rows.
        from: Input,
        /// Ordered rendered text columns.
        texts: Vec<ColumnIndex>,
        /// Literal text before, between and after columns; exactly one more than `texts`.
        literals: Vec<crate::domain::plan_parts::types::LiteralText>,
        /// Original physical statement span.
        span: ColumnIndex,
        /// Maximum physical display columns.
        max_line: MaxLine,
        /// Physical tab stop width.
        tab_width: TabWidth,
    },
    /// Append checked byte offsets of an existing physical source span.
    SpanBounds {
        /// Per-file input rows.
        from: Input,
        /// Span whose endpoints are projected.
        span: ColumnIndex,
    },
}
