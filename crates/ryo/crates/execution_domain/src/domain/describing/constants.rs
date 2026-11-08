//! The names of the `describing` concept: the description of an operator and of its fields.

use ryo_values::domain::descriptions::constants::OUTPUT;
use ryo_values::domain::descriptions::types::FieldRequirement::{Optional, Required};
use ryo_values::domain::descriptions::types::{
    Example, FieldDescription, FieldName, FieldRequirement, OperatorDescription, OperatorName,
    OptionLine, Purpose,
};

/// The field naming the column that holds the far end of an edge.
pub(crate) const DESTINATION: &str = "destination";
/// The field naming the relation an operator reads its edges from.
pub(crate) const RELATION: &str = "relation";

/// A field of an operator's description: its name, what it holds and whether it must be given.
pub(crate) fn field(name: &str, summary: &str, required: FieldRequirement) -> FieldDescription {
    FieldDescription {
        name: FieldName(name.into()),
        summary: Purpose(summary.into()),
        required,
    }
}

/// An operator's description: its name, what it does, its fields, its options and an example of it.
pub(crate) fn described(
    name: &str,
    summary: &str,
    fields: Vec<FieldDescription>,
    options: &[&str],
    example: &str,
) -> OperatorDescription {
    OperatorDescription {
        name: OperatorName(name.into()),
        summary: Purpose(summary.into()),
        fields,
        options: options
            .iter()
            .map(|option| OptionLine((*option).into()))
            .collect(),
        example: Example(example.into()),
    }
}

/// The description of `span-between`.
pub(crate) fn span_between() -> OperatorDescription {
    described(
        "span-between",
        "Append [start.start, end.end) in the current file, preserving every input row. Absent spans propagate; invalid byte ranges fail.",
        vec![
            field("from", "Per-file input rows.", Required),
            field("start", "Span column supplying the start offset.", Required),
            field("end", "Span column supplying the end offset.", Required),
            field(OUTPUT, "New span column.", Required),
        ],
        &[],
        r#"let example = span-between(from="function", start="span", end="span", output="diagnostic_span"); emit example subject file key(diagnostic_span);"#,
    )
}

/// The description of `span-bounds`.
pub(crate) fn span_bounds() -> OperatorDescription {
    described(
        "span-bounds",
        "Append zero-based start and exclusive end byte offsets of each present ordered span. Input order and duplicates are preserved; absent, reversed, or out-of-file spans fail before their row emits.",
        vec![
            field("from", "Input rows.", Required),
            field("span", "Physical span column to inspect.", Required),
            field(
                "start-output",
                "New zero-based start byte column.",
                Required,
            ),
            field("end-output", "New exclusive end byte column.", Required),
        ],
        &[],
        r#"let example = span-bounds(from="function", span="span", start-output="begin", end-output="finish"); emit example subject file key(span);"#,
    )
}

/// The description of `source-matches`.
pub(crate) fn source_matches() -> OperatorDescription {
    described(
        "source-matches",
        "For each per-file input row, append one absolute physical span for each non-overlapping regex occurrence in its checked source span. Input order and duplicates are preserved; zero-width matches use the regex engine's ordered match positions. Absent spans emit no rows; invalid ranges fail before that row emits.",
        vec![
            field("from", "Per-file input rows.", Required),
            field("span", "Physical source span column to search.", Required),
            field("pattern", "Literal regular expression.", Required),
            field(OUTPUT, "New absolute occurrence span column.", Required),
            field(
                "base-line",
                "Optional positive one-based line at the input span's start; requires line-output.",
                Optional,
            ),
            field(
                "line-output",
                "Optional output line at each match start; requires base-line.",
                Optional,
            ),
        ],
        &[],
        r#"let example = source-matches(from="syntax-tree", span="span", pattern="TODO", output="match_span"); emit example subject file key(match_span);"#,
    )
}

/// The description of `render-syntax`.
pub(crate) fn render_syntax() -> OperatorDescription {
    described(
        "render-syntax",
        "Append exact provider-rendered syntax for an already selected node in a declared embedding context. Unsupported valid syntax makes the consuming rule incomplete.",
        vec![
            field("from", "Candidate rows from this file.", Required),
            field("node", "Node column to render.", Required),
            field(
                "context",
                "Embedding role: assignment-target, attribute-receiver, or positional-argument.",
                Required,
            ),
            field(OUTPUT, "New exact text column.", Required),
        ],
        &[],
        r#"let example = render-syntax(from="syntax-tree", node="id", context="assignment-target", output="rendered"); emit example subject file key(id);"#,
    )
}

/// The description of `fits-rendered`.
pub(crate) fn fits_rendered() -> OperatorDescription {
    described(
        "fits-rendered",
        "Keep rendered candidates whose physical display lines fit a declared maximum, including the original source prefix on the first line and tab stops.",
        vec![
            field("from", "Rendered candidate rows from this file.", Required),
            field("texts", "Ordered exact rendered text columns.", Required),
            field(
                "literals",
                "Literal separators, one more than the text columns.",
                Required,
            ),
            field("span", "Original physical statement span.", Required),
            field("max-line", "Maximum display columns.", Required),
            field("tab-width", "Physical tab stop width.", Required),
        ],
        &[],
        r#"let example = fits-rendered(from="function", texts=["name"], literals=["", ""], span="span", max-line=88, tab-width=8); emit example subject file key(id);"#,
    )
}

/// The description of `replace`.
pub(crate) fn replace() -> OperatorDescription {
    described(
        "replace",
        "Append text with literal replacement of regex matches.",
        vec![
            field("from", "Input rows.", Required),
            field("text", "Text column.", Required),
            field(
                ryo_values::domain::descriptions::constants::PATTERN,
                "Constant regular expression.",
                Required,
            ),
            field(
                ryo_values::domain::descriptions::constants::REPLACEMENT,
                "Literal replacement text.",
                Required,
            ),
            field(
                ryo_values::domain::descriptions::constants::OUTPUT,
                "New output column.",
                Required,
            ),
        ],
        &[],
        r#"let example = replace(from="function", text="name", pattern="_", replacement="-", output="label"); emit example subject file key(label);"#,
    )
}

/// The description of `ordered-match`.
pub(crate) fn ordered_match() -> OperatorDescription {
    described(
        "ordered-match",
        "Consume the earliest later right row for each ordered left row in an equal-key group.",
        vec![
            field("left", "Left input.", Required),
            field("right", "Right input.", Required),
            field(
                ryo_values::domain::descriptions::constants::LEFT_ON,
                "Left equality keys.",
                Required,
            ),
            field(
                ryo_values::domain::descriptions::constants::RIGHT_ON,
                "Right equality keys.",
                Required,
            ),
            field("left-order", "Left numeric order.", Required),
            field("right-order", "Right numeric order.", Required),
            field(
                ryo_values::domain::descriptions::constants::OUTPUT,
                "Named columns with explicit input sources.",
                Required,
            ),
        ],
        &[],
        r#"let requests = extend(from="function", values={sequence: 1}); let replies = extend(from="function", values={sequence: 2}); let example = ordered-match(left=requests, right=replies, left-on=[file,name], right-on=[file,name], left-order="sequence", right-order="sequence", output={file: [left,file], name: [right,name]}); emit example subject file key(name); "#,
    )
}

/// The description of `ordered-dfs`.
pub(crate) fn ordered_dfs() -> OperatorDescription {
    described(
        "ordered-dfs",
        "Visit outgoing edges in numeric order, recursively depth first for each distinct partition/root.",
        vec![
            field("from", "Root rows.", Required),
            field("root", "Root entity column.", Required),
            field("edges", "Directed edge rows.", Required),
            field("source", "Edge source column.", Required),
            field(DESTINATION, "Edge destination column.", Required),
            field(
                "order",
                "Numeric outgoing-edge order; ties are rejected.",
                Required,
            ),
            field(
                "edge",
                "Edge identity column retained on every visit.",
                Required,
            ),
            field(
                "keys",
                "Matching partition columns of roots and edges, in output order; omitted means one graph.",
                Optional,
            ),
            field(
                OUTPUT,
                "Name of the zero-based visit ordinal column.",
                Required,
            ),
        ],
        &[],
        r#"let roots = select id as root, file from "function"; let edges = select parent as source, child as destination, ordinal as position, child as base, file from "syntax-child"; let example = ordered-dfs(from=roots,root=root,edges=edges,source=source,destination=destination,order=position,edge=base,keys=[file],output=visit); emit example subject file key(base,visit);"#,
    )
}
