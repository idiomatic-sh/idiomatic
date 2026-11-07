//! The names of the `operator` concept: what each operator is called and how it is described,
//! text that leaves the engine in `operator list` and `operator describe`.

use crate::domain::describing::constants::{DESTINATION, RELATION, described, field};
use ryo_relations::domain::relations::constants::SOURCE;
use ryo_values::domain::descriptions::constants::{
    COLUMN, COLUMNS, DIGEST, OUTPUT, PROJECT, SELECT,
};
use ryo_values::domain::descriptions::types::{FieldRequirement, OperatorDescription};
use ryo_values::domain::query_operators::types::QueryOperator;

/// What a render demand says when the file admitted no semantic session.
pub(crate) const NO_SEMANTIC_SESSION: &str = "no file semantic session was admitted";

/// Every operator, in the order the documentation lists them.
pub(crate) const OPERATORS: [QueryOperator; 25] = [
    QueryOperator::SpanBetween,
    QueryOperator::SpanBounds,
    QueryOperator::SourceMatches,
    QueryOperator::RenderSyntax,
    QueryOperator::FitsRendered,
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

/// What an operator is, what it takes and an example.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one exhaustive match with one arm for each operator, each arm the text of that operator's description; splitting it would split the one table of descriptions"
)]
pub(crate) fn describe(operator: QueryOperator) -> OperatorDescription {
    match operator {
        QueryOperator::SpanBetween => described(
            "span-between",
            "Append [start.start, end.end) in the current file, preserving every input row. Absent spans propagate; invalid byte ranges fail.",
            vec![
                field("from", "Per-file input rows.", FieldRequirement::Required),
                field(
                    "start",
                    "Span column supplying the start offset.",
                    FieldRequirement::Required,
                ),
                field(
                    "end",
                    "Span column supplying the end offset.",
                    FieldRequirement::Required,
                ),
                field(OUTPUT, "New span column.", FieldRequirement::Required),
            ],
            &[],
            r#"let example = span-between(from="function", start="span", end="span", output="diagnostic_span"); emit example subject file key(diagnostic_span);"#,
        ),
        QueryOperator::SpanBounds => described(
            "span-bounds",
            "Append zero-based start and exclusive end byte offsets of each present ordered span. Input order and duplicates are preserved; absent, reversed, or out-of-file spans fail before their row emits.",
            vec![
                field("from", "Input rows.", FieldRequirement::Required),
                field(
                    "span",
                    "Physical span column to inspect.",
                    FieldRequirement::Required,
                ),
                field(
                    "start-output",
                    "New zero-based start byte column.",
                    FieldRequirement::Required,
                ),
                field(
                    "end-output",
                    "New exclusive end byte column.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"let example = span-bounds(from="function", span="span", start-output="begin", end-output="finish"); emit example subject file key(span);"#,
        ),
        QueryOperator::SourceMatches => described(
            "source-matches",
            "For each per-file input row, append one absolute physical span for each non-overlapping regex occurrence in its checked source span. Input order and duplicates are preserved; zero-width matches use the regex engine's ordered match positions. Absent spans emit no rows; invalid ranges fail before that row emits.",
            vec![
                field("from", "Per-file input rows.", FieldRequirement::Required),
                field(
                    "span",
                    "Physical source span column to search.",
                    FieldRequirement::Required,
                ),
                field(
                    "pattern",
                    "Literal regular expression.",
                    FieldRequirement::Required,
                ),
                field(
                    OUTPUT,
                    "New absolute occurrence span column.",
                    FieldRequirement::Required,
                ),
                field(
                    "base-line",
                    "Optional positive one-based line at the input span's start; requires line-output.",
                    FieldRequirement::Optional,
                ),
                field(
                    "line-output",
                    "Optional output line at each match start; requires base-line.",
                    FieldRequirement::Optional,
                ),
            ],
            &[],
            r#"let example = source-matches(from="syntax-tree", span="span", pattern="TODO", output="match_span"); emit example subject file key(match_span);"#,
        ),
        QueryOperator::RenderSyntax => described(
            "render-syntax",
            "Append exact provider-rendered syntax for an already selected node in a declared embedding context. Unsupported valid syntax makes the consuming rule incomplete.",
            vec![
                field(
                    "from",
                    "Candidate rows from this file.",
                    FieldRequirement::Required,
                ),
                field("node", "Node column to render.", FieldRequirement::Required),
                field(
                    "context",
                    "Embedding role: assignment-target, attribute-receiver, or positional-argument.",
                    FieldRequirement::Required,
                ),
                field(OUTPUT, "New exact text column.", FieldRequirement::Required),
            ],
            &[],
            r#"let example = render-syntax(from="syntax-tree", node="id", context="assignment-target", output="rendered"); emit example subject file key(id);"#,
        ),
        QueryOperator::FitsRendered => described(
            "fits-rendered",
            "Keep rendered candidates whose physical display lines fit a declared maximum, including the original source prefix on the first line and tab stops.",
            vec![
                field(
                    "from",
                    "Rendered candidate rows from this file.",
                    FieldRequirement::Required,
                ),
                field(
                    "texts",
                    "Ordered exact rendered text columns.",
                    FieldRequirement::Required,
                ),
                field(
                    "literals",
                    "Literal separators, one more than the text columns.",
                    FieldRequirement::Required,
                ),
                field(
                    "span",
                    "Original physical statement span.",
                    FieldRequirement::Required,
                ),
                field(
                    "max-line",
                    "Maximum display columns.",
                    FieldRequirement::Required,
                ),
                field(
                    "tab-width",
                    "Physical tab stop width.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"let example = fits-rendered(from="function", texts=["name"], literals=["", ""], span="span", max-line=88, tab-width=8); emit example subject file key(id);"#,
        ),
        QueryOperator::Replace => described(
            "replace",
            "Append text with literal replacement of regex matches.",
            vec![
                field("from", "Input rows.", FieldRequirement::Required),
                field("text", "Text column.", FieldRequirement::Required),
                field(
                    ryo_values::domain::descriptions::constants::PATTERN,
                    "Constant regular expression.",
                    FieldRequirement::Required,
                ),
                field(
                    ryo_values::domain::descriptions::constants::REPLACEMENT,
                    "Literal replacement text.",
                    FieldRequirement::Required,
                ),
                field(
                    ryo_values::domain::descriptions::constants::OUTPUT,
                    "New output column.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"let example = replace(from="function", text="name", pattern="_", replacement="-", output="label"); emit example subject file key(label);"#,
        ),
        QueryOperator::OrderedDfs => described(
            "ordered-dfs",
            "Visit outgoing edges in numeric order, recursively depth first for each distinct partition/root.",
            vec![
                field("from", "Root rows.", FieldRequirement::Required),
                field("root", "Root entity column.", FieldRequirement::Required),
                field("edges", "Directed edge rows.", FieldRequirement::Required),
                field("source", "Edge source column.", FieldRequirement::Required),
                field(
                    DESTINATION,
                    "Edge destination column.",
                    FieldRequirement::Required,
                ),
                field(
                    "order",
                    "Numeric outgoing-edge order; ties are rejected.",
                    FieldRequirement::Required,
                ),
                field(
                    "edge",
                    "Edge identity column retained on every visit.",
                    FieldRequirement::Required,
                ),
                field(
                    "keys",
                    "Matching partition columns of roots and edges, in output order; omitted means one graph.",
                    FieldRequirement::Optional,
                ),
                field(
                    OUTPUT,
                    "Name of the zero-based visit ordinal column.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"let roots = select id as root, file from "function"; let edges = select parent as source, child as destination, ordinal as position, child as base, file from "syntax-child"; let example = ordered-dfs(from=roots,root=root,edges=edges,source=source,destination=destination,order=position,edge=base,keys=[file],output=visit); emit example subject file key(base,visit);"#,
        ),
        QueryOperator::OrderedMatch => described(
            "ordered-match",
            "Consume the earliest later right row for each ordered left row in an equal-key group.",
            vec![
                field("left", "Left input.", FieldRequirement::Required),
                field("right", "Right input.", FieldRequirement::Required),
                field(
                    ryo_values::domain::descriptions::constants::LEFT_ON,
                    "Left equality keys.",
                    FieldRequirement::Required,
                ),
                field(
                    ryo_values::domain::descriptions::constants::RIGHT_ON,
                    "Right equality keys.",
                    FieldRequirement::Required,
                ),
                field(
                    "left-order",
                    "Left numeric order.",
                    FieldRequirement::Required,
                ),
                field(
                    "right-order",
                    "Right numeric order.",
                    FieldRequirement::Required,
                ),
                field(
                    ryo_values::domain::descriptions::constants::OUTPUT,
                    "Named columns with explicit input sources.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"let requests = extend(from="function", values={sequence: 1}); let replies = extend(from="function", values={sequence: 2}); let example = ordered-match(left=requests, right=replies, left-on=[file,name], right-on=[file,name], left-order="sequence", right-order="sequence", output={file: [left,file], name: [right,name]}); emit example subject file key(name); "#,
        ),
        QueryOperator::Select => described(
            SELECT,
            "Keep rows for which every comparison is true.",
            vec![
                field("from", "Input to filter.", FieldRequirement::Required),
                field(
                    "where",
                    "Typed comparisons of columns, literals, parameters or functions of them.",
                    FieldRequirement::Required,
                ),
            ],
            &[
                "comparison family: equality",
                "comparison family: ordering",
                "comparison family: float-text",
                "comparison family: numeric, numeric_gte(value, 86400) compares canonical integer/binary64 text or integer cells without rounding integers; NaN and malformed text are false",
                "integer_successor(left, right): true only when right is exactly left plus one; canonical integer text or integer cells, no float coercion",
                "comparison family: membership",
                "comparison family: glob",
                "comparison family: affix, starts_with(path, \"src/\") or ends_with(name, \"Service\"), each side text",
                "function: concat(a, b, ...) joins two or more texts, concat(pascal(p.pkg), \"Service\") is TodoListsService for todo_lists",
                "function: pascal(text) writes todo_lists as TodoLists",
                "function: camel(text) writes todo_lists as todoLists",
                "function: snake(text) writes TodoLists as todo_lists",
                "function: kebab(text) writes TodoLists as todo-lists",
            ],
            r#"let example = select * from "function" where name = "example"; emit example subject file key(span);"#,
        ),
        QueryOperator::Project => described(
            PROJECT,
            "Keep named columns and deduplicate equal rows.",
            vec![
                field("from", "Input to project.", FieldRequirement::Required),
                field(
                    COLUMNS,
                    "Columns retained in output.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"let example = select file, name from "function"; emit example subject file key(name);"#,
        ),
        QueryOperator::Extend => described(
            "extend",
            "Append numeric or flag columns.",
            vec![
                field("from", "Input to extend.", FieldRequirement::Required),
                field(
                    "values",
                    "New columns from numbers or parameters.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"let example = extend(from="function", values={layer: 1}); emit example subject file key(layer);"#,
        ),
        QueryOperator::Join => described(
            "join",
            "Combine rows with equal typed keys.",
            vec![
                field("left", "Left input.", FieldRequirement::Required),
                field("right", "Right input.", FieldRequirement::Required),
                field(
                    ryo_values::domain::descriptions::constants::LEFT_ON,
                    "Left equality keys; empty for Cartesian product.",
                    FieldRequirement::Required,
                ),
                field(
                    ryo_values::domain::descriptions::constants::RIGHT_ON,
                    "Right equality keys.",
                    FieldRequirement::Required,
                ),
                field(
                    ryo_values::domain::descriptions::constants::OUTPUT,
                    "Output names and their explicit input sources.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"let example = select l.file, r.name from "function" as l join "function" as r on l.file = r.file; emit example subject file key(name);"#,
        ),
        QueryOperator::Union => described(
            "union",
            "Rows present in either same-schema input.",
            vec![
                field("left", "Left input.", FieldRequirement::Required),
                field("right", "Right input.", FieldRequirement::Required),
            ],
            &[],
            r#"let example = union(left="function", right="function"); emit example subject file key(span);"#,
        ),
        QueryOperator::Intersection => described(
            "intersection",
            "Rows present in both same-schema inputs.",
            vec![
                field("left", "Left input.", FieldRequirement::Required),
                field("right", "Right input.", FieldRequirement::Required),
            ],
            &[],
            r#"let example = intersection(left="function", right="function"); emit example subject file key(span);"#,
        ),
        QueryOperator::Difference => described(
            "difference",
            "Left rows absent from a complete same-schema right input.",
            vec![
                field("left", "Rows to retain.", FieldRequirement::Required),
                field("right", "Rows to subtract.", FieldRequirement::Required),
            ],
            &[],
            r#"let example = difference(left="function", right="function"); emit example subject file key(span);"#,
        ),
        QueryOperator::Antijoin => described(
            "antijoin",
            "Left rows without a complete matching right key.",
            vec![
                field(
                    "left",
                    "Rows to retain when unmatched.",
                    FieldRequirement::Required,
                ),
                field(
                    "right",
                    "Rows consulted for matches.",
                    FieldRequirement::Required,
                ),
                field(
                    ryo_values::domain::descriptions::constants::LEFT_ON,
                    "Nonempty left match keys.",
                    FieldRequirement::Required,
                ),
                field(
                    ryo_values::domain::descriptions::constants::RIGHT_ON,
                    "Nonempty right match keys.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"let example = antijoin(left="function", right="function", left-on=[file], right-on=[file]); emit example subject file key(span);"#,
        ),
        QueryOperator::GroupCount => described(
            "group-count",
            "Group rows and append a count.",
            vec![
                field("from", "Rows to aggregate.", FieldRequirement::Required),
                field(
                    "keys",
                    "Columns identifying one group.",
                    FieldRequirement::Required,
                ),
                field(
                    "distinct",
                    "Count distinct values of this column.",
                    FieldRequirement::Optional,
                ),
                field("count", "Output count column.", FieldRequirement::Required),
            ],
            &[],
            r#"let example = group-count(from="function", keys=[file], distinct=name, count=functions); emit example subject file key(functions);"#,
        ),
        QueryOperator::GroupDigest => described(
            "group-digest",
            "Group rows and append a digest of the set of distinct values of a column, and how many there are.",
            vec![
                field("from", "Rows to aggregate.", FieldRequirement::Required),
                field(
                    "keys",
                    "Columns identifying one group.",
                    FieldRequirement::Required,
                ),
                field(
                    "distinct",
                    "The column whose distinct values are digested.",
                    FieldRequirement::Required,
                ),
                field(
                    DIGEST,
                    "Output digest column: equal sets give equal digests, whatever the order or repeats.",
                    FieldRequirement::Required,
                ),
                field(
                    "count",
                    "Output column holding how many distinct values the group has.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"let example = group-digest(from="function", keys=[file], distinct=name, digest=names, count=how-many); emit example subject file key(names);"#,
        ),
        QueryOperator::Capture => described(
            "capture",
            "Match paths and append named nonempty captures.",
            vec![
                field(
                    "from",
                    "Rows containing a path.",
                    FieldRequirement::Required,
                ),
                field("path", "Path column to match.", FieldRequirement::Required),
                field(
                    ryo_values::domain::descriptions::constants::PATTERN,
                    "Anchored path pattern with named slots.",
                    FieldRequirement::Required,
                ),
                field(
                    "captures",
                    "Output capture columns in pattern order.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"let example = capture(from="function", path=file, pattern="docs/{page}.md", captures=[page]); emit example subject file key(page);"#,
        ),
        QueryOperator::Words => described(
            "words",
            "Split a text column into lower case words, one row for each, where letters and digits end and begin.",
            vec![
                field(
                    "from",
                    "Rows containing a text.",
                    FieldRequirement::Required,
                ),
                field("text", "Text column to split.", FieldRequirement::Required),
                field(
                    "word",
                    "Output column holding each word.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"let example = words(from="comment", text=text, word=each); emit example subject file key(each);"#,
        ),
        QueryOperator::Rank => described(
            "rank",
            "Append the place of the first of an ordered list of sets of text that holds the text of a column, the first set being 0. A row whose text no set holds is dropped.",
            vec![
                field(
                    "from",
                    "Rows containing a text.",
                    FieldRequirement::Required,
                ),
                field(COLUMN, "Text column to rank.", FieldRequirement::Required),
                field(
                    "sets",
                    "Parameter of type list<set<string>>: the sets, in order.",
                    FieldRequirement::Required,
                ),
                field(
                    ryo_values::domain::descriptions::constants::OUTPUT,
                    "Output rank column.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"param layers: list<set<string>>; let example = rank(from="defines", column=name, sets=$layers, output=layer); emit example subject file key(layer);"#,
        ),
        QueryOperator::Axis => described(
            "axis",
            "Navigate a typed hierarchy and append the related entity.",
            vec![
                field(
                    "from",
                    "Rows containing starting entities in id.",
                    FieldRequirement::Required,
                ),
                field(
                    RELATION,
                    "Typed parent-child edges, in parent and child.",
                    FieldRequirement::Required,
                ),
                field(
                    "direction",
                    "Hierarchy direction.",
                    FieldRequirement::Required,
                ),
                field(
                    "domain",
                    "Rows proving valid output entities in id.",
                    FieldRequirement::Required,
                ),
            ],
            &[
                "direction=parent",
                "direction=child",
                "direction=ancestor",
                "direction=descendant",
                "direction=nearest-ancestor",
                "direction=previous-sibling",
                "direction=next-sibling",
            ],
            r#"let example = axis(from="syntax-tree", relation="syntax-child", direction=ancestor, domain="syntax-tree"); emit example subject file key(related);"#,
        ),
        QueryOperator::Closure => described(
            "closure",
            "Transitively reachable pairs within optional exact key partitions.",
            vec![
                field("edges", "Directed edge rows.", FieldRequirement::Required),
                field(SOURCE, "Edge source.", FieldRequirement::Required),
                field(DESTINATION, "Edge destination.", FieldRequirement::Required),
                field(
                    "keys",
                    "Optional columns defining independent exact graph partitions, in output order.",
                    FieldRequirement::Optional,
                ),
            ],
            &[],
            r#"let example = closure(edges="source-dependency", source=source, destination=target); emit example subject source key(target);"#,
        ),
        QueryOperator::Rows => described(
            "rows",
            "The rows a `list<table>` parameter holds: one for each table, a text column for each key named.",
            vec![
                field(
                    "from",
                    "Parameter of type list<table>: the tables, each a row.",
                    FieldRequirement::Required,
                ),
                field(
                    COLUMNS,
                    "The keys whose values fill the columns, in order; every table has each.",
                    FieldRequirement::Required,
                ),
            ],
            &[],
            r#"param reexports: list<table>; let example = rows(from=$reexports, columns=["resource", "path"]); emit example subject path key(resource);"#,
        ),
        QueryOperator::Cycles => described(
            "cycles",
            "Directed graph cycles: one row for each entity that can reach itself, with the entity in both columns; two files that import each other give two rows.",
            vec![
                field("edges", "Directed edge rows.", FieldRequirement::Required),
                field(SOURCE, "Edge source.", FieldRequirement::Required),
                field(DESTINATION, "Edge destination.", FieldRequirement::Required),
            ],
            &[],
            r#"let example = cycles(edges="source-dependency", source=source, destination=target); emit example subject source key(target);"#,
        ),
    }
}
