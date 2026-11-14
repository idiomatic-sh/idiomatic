//! The names of the `operator` concept: what each operator is called and how it is described,
//! text that leaves the engine in `operator list` and `operator describe`.

use crate::domain::describing::constants::{DESTINATION, RELATION, described, field};
use ryo_relations::domain::relations::constants::SOURCE;
use ryo_values::domain::descriptions::constants::{
    COLUMN, COLUMNS, DIGEST, OUTPUT, PROJECT, SELECT,
};
use ryo_values::domain::descriptions::types::FieldRequirement::{Optional, Required};
use ryo_values::domain::descriptions::types::OperatorDescription;
use ryo_values::domain::query_operators::types::QueryOperator;

/// Every operator, in the order the documentation lists them.
pub const OPERATORS: [QueryOperator; 25] = [
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
pub fn describe(operator: QueryOperator) -> OperatorDescription {
    match operator {
        QueryOperator::SpanBetween => crate::domain::describing::constants::span_between(),
        QueryOperator::SpanBounds => crate::domain::describing::constants::span_bounds(),
        QueryOperator::SourceMatches => crate::domain::describing::constants::source_matches(),
        QueryOperator::RenderSyntax => crate::domain::describing::constants::render_syntax(),
        QueryOperator::FitsRendered => crate::domain::describing::constants::fits_rendered(),
        QueryOperator::Replace => crate::domain::describing::constants::replace(),
        QueryOperator::OrderedDfs => crate::domain::describing::constants::ordered_dfs(),
        QueryOperator::OrderedMatch => crate::domain::describing::constants::ordered_match(),
        QueryOperator::Select => described(
            SELECT,
            "Keep rows for which every comparison is true.",
            vec![
                field("from", "Input to filter.", Required),
                field(
                    "where",
                    "Typed comparisons of columns, literals, parameters or functions of them.",
                    Required,
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
                field("from", "Input to project.", Required),
                field(COLUMNS, "Columns retained in output.", Required),
            ],
            &[],
            r#"let example = select file, name from "function"; emit example subject file key(name);"#,
        ),
        QueryOperator::Extend => described(
            "extend",
            "Append numeric or flag columns.",
            vec![
                field("from", "Input to extend.", Required),
                field(
                    "values",
                    "New columns from numbers or parameters.",
                    Required,
                ),
            ],
            &[],
            r#"let example = extend(from="function", values={layer: 1}); emit example subject file key(layer);"#,
        ),
        QueryOperator::Join => described(
            "join",
            "Combine rows with equal typed keys.",
            vec![
                field("left", "Left input.", Required),
                field("right", "Right input.", Required),
                field(
                    ryo_values::domain::descriptions::constants::LEFT_ON,
                    "Left equality keys; empty for Cartesian product.",
                    Required,
                ),
                field(
                    ryo_values::domain::descriptions::constants::RIGHT_ON,
                    "Right equality keys.",
                    Required,
                ),
                field(
                    ryo_values::domain::descriptions::constants::OUTPUT,
                    "Output names and their explicit input sources.",
                    Required,
                ),
            ],
            &[],
            r#"let example = select l.file, r.name from "function" as l join "function" as r on l.file = r.file; emit example subject file key(name);"#,
        ),
        QueryOperator::Union => described(
            "union",
            "Rows present in either same-schema input.",
            vec![
                field("left", "Left input.", Required),
                field("right", "Right input.", Required),
            ],
            &[],
            r#"let example = union(left="function", right="function"); emit example subject file key(span);"#,
        ),
        QueryOperator::Intersection => described(
            "intersection",
            "Rows present in both same-schema inputs.",
            vec![
                field("left", "Left input.", Required),
                field("right", "Right input.", Required),
            ],
            &[],
            r#"let example = intersection(left="function", right="function"); emit example subject file key(span);"#,
        ),
        QueryOperator::Difference => described(
            "difference",
            "Left rows absent from a complete same-schema right input.",
            vec![
                field("left", "Rows to retain.", Required),
                field("right", "Rows to subtract.", Required),
            ],
            &[],
            r#"let example = difference(left="function", right="function"); emit example subject file key(span);"#,
        ),
        QueryOperator::Antijoin => described(
            "antijoin",
            "Left rows without a complete matching right key.",
            vec![
                field("left", "Rows to retain when unmatched.", Required),
                field("right", "Rows consulted for matches.", Required),
                field(
                    ryo_values::domain::descriptions::constants::LEFT_ON,
                    "Nonempty left match keys.",
                    Required,
                ),
                field(
                    ryo_values::domain::descriptions::constants::RIGHT_ON,
                    "Nonempty right match keys.",
                    Required,
                ),
            ],
            &[],
            r#"let example = antijoin(left="function", right="function", left-on=[file], right-on=[file]); emit example subject file key(span);"#,
        ),
        QueryOperator::GroupCount => described(
            "group-count",
            "Group rows and append a count.",
            vec![
                field("from", "Rows to aggregate.", Required),
                field("keys", "Columns identifying one group.", Required),
                field(
                    "distinct",
                    "Count distinct values of this column.",
                    Optional,
                ),
                field("count", "Output count column.", Required),
            ],
            &[],
            r#"let example = group-count(from="function", keys=[file], distinct=name, count=functions); emit example subject file key(functions);"#,
        ),
        QueryOperator::GroupDigest => described(
            "group-digest",
            "Group rows and append a digest of the set of distinct values of a column, and how many there are.",
            vec![
                field("from", "Rows to aggregate.", Required),
                field("keys", "Columns identifying one group.", Required),
                field(
                    "distinct",
                    "The column whose distinct values are digested.",
                    Required,
                ),
                field(
                    DIGEST,
                    "Output digest column: equal sets give equal digests, whatever the order or repeats.",
                    Required,
                ),
                field(
                    "count",
                    "Output column holding how many distinct values the group has.",
                    Required,
                ),
            ],
            &[],
            r#"let example = group-digest(from="function", keys=[file], distinct=name, digest=names, count=how-many); emit example subject file key(names);"#,
        ),
        QueryOperator::Capture => described(
            "capture",
            "Match paths and append named nonempty captures.",
            vec![
                field("from", "Rows containing a path.", Required),
                field("path", "Path column to match.", Required),
                field(
                    ryo_values::domain::descriptions::constants::PATTERN,
                    "Anchored path pattern with named slots.",
                    Required,
                ),
                field(
                    "captures",
                    "Output capture columns in pattern order.",
                    Required,
                ),
            ],
            &[],
            r#"let example = capture(from="function", path=file, pattern="docs/{page}.md", captures=[page]); emit example subject file key(page);"#,
        ),
        QueryOperator::Words => described(
            "words",
            "Split a text column into lower case words, one row for each, where letters and digits end and begin.",
            vec![
                field("from", "Rows containing a text.", Required),
                field("text", "Text column to split.", Required),
                field("word", "Output column holding each word.", Required),
            ],
            &[],
            r#"let example = words(from="comment", text=text, word=each); emit example subject file key(each);"#,
        ),
        QueryOperator::Rank => described(
            "rank",
            "Append the place of the first of an ordered list of sets of text that holds the text of a column, the first set being 0. A row whose text no set holds is dropped.",
            vec![
                field("from", "Rows containing a text.", Required),
                field(COLUMN, "Text column to rank.", Required),
                field(
                    "sets",
                    "Parameter of type list<set<string>>: the sets, in order.",
                    Required,
                ),
                field(
                    ryo_values::domain::descriptions::constants::OUTPUT,
                    "Output rank column.",
                    Required,
                ),
            ],
            &[],
            r#"param layers: list<set<string>>; let example = rank(from="defines", column=name, sets=$layers, output=layer); emit example subject file key(layer);"#,
        ),
        QueryOperator::Axis => described(
            "axis",
            "Navigate a typed hierarchy and append the related entity.",
            vec![
                field("from", "Rows containing starting entities in id.", Required),
                field(
                    RELATION,
                    "Typed parent-child edges, in parent and child.",
                    Required,
                ),
                field("direction", "Hierarchy direction.", Required),
                field(
                    "domain",
                    "Rows proving valid output entities in id.",
                    Required,
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
                field("edges", "Directed edge rows.", Required),
                field(SOURCE, "Edge source.", Required),
                field(DESTINATION, "Edge destination.", Required),
                field(
                    "keys",
                    "Optional columns defining independent exact graph partitions, in output order.",
                    Optional,
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
                    Required,
                ),
                field(
                    COLUMNS,
                    "The keys whose values fill the columns, in order; every table has each.",
                    Required,
                ),
            ],
            &[],
            r#"param reexports: list<table>; let example = rows(from=$reexports, columns=["resource", "path"]); emit example subject path key(resource);"#,
        ),
        QueryOperator::Cycles => described(
            "cycles",
            "Directed graph cycles: one row for each entity that can reach itself, with the entity in both columns; two files that import each other give two rows.",
            vec![
                field("edges", "Directed edge rows.", Required),
                field(SOURCE, "Edge source.", Required),
                field(DESTINATION, "Edge destination.", Required),
            ],
            &[],
            r#"let example = cycles(edges="source-dependency", source=source, destination=target); emit example subject source key(target);"#,
        ),
    }
}
