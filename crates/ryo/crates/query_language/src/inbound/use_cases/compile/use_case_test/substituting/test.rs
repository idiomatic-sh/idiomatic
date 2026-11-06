//! The compiler checks types and constants before either operator reads any rows.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::selects;
use crate::domain::lowering::errors::LoweringError;
use crate::outbound::syntax::fixtures::winnow::winnow;
use crate::outbound::syntax::interface::SyntaxPort as _;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::hosted::types::{ColumnLabel, HostedColumn, HostedName, HostedRelation};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::program::types::QueryView;

type Input = &'static str;
type Value = bool;
type Failure = LoweringError;

fn subject(text: Input) -> Result<Value, Failure> {
    let program = winnow()
        .read(QueryView(text))
        .expect("query fixture parses");
    let hosted = [HostedRelation {
        name: HostedName("global".into()),
        columns: vec![
            HostedColumn {
                name: ColumnLabel("id".into()),
                kind: CellKind::Node,
            },
            HostedColumn {
                name: ColumnLabel("file".into()),
                kind: CellKind::File,
            },
            HostedColumn {
                name: ColumnLabel("span".into()),
                kind: CellKind::Span,
            },
        ],
    }];
    let plan = crate::domain::assembling::service::lower_with_observations(
        &program,
        &[Relation::Function, Relation::SourceLocation],
        &hosted,
        &[],
    )?;
    Ok(plan.steps.iter().any(|step| {
        matches!(
            step.operation,
            Operation::Replace { .. }
                | Operation::OrderedMatch { .. }
                | Operation::OrderedDfs { .. }
        )
    }))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("semantic_call_rejects_extra_arguments"),
        word: Word::Refuses,
        input: || r#"let a = render-syntax(from="function", node="id", context="assignment-target", output="rendered", extra="id"); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::SemanticArgument { .. }))
        }),
    },
    Case {
        name: CaseName("semantic_call_rejects_project_input"),
        word: Word::Refuses,
        input: || r#"let a = render-syntax(from="global", node="id", context="assignment-target", output="rendered"); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::CrossFileSemantic { .. }))
        }),
    },
    Case {
        name: CaseName("semantic_call_rejects_unknown_context"),
        word: Word::Refuses,
        input: || r#"let a = render-syntax(from="function", node="id", context="not-a-context", output="rendered"); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::InvalidSemanticLiteral { .. }
            ))
        }),
    },
    Case {
        name: CaseName("span_bounds_rejects_extra_arguments"),
        word: Word::Refuses,
        input: || r#"let a = span-bounds(from="function", span="span", start-output="begin", end-output="finish", extra="span"); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::SpanBoundsArgument { .. }))
        }),
    },
    Case {
        name: CaseName("source_matches_rejects_extra_arguments"),
        word: Word::Refuses,
        input: || r#"let a = source-matches(from="function", span="span", pattern="fn", output="hit", extra="span"); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::SourceMatchesArgument { .. }))
        }),
    },
    Case {
        name: CaseName("source_matches_rejects_project_input"),
        word: Word::Refuses,
        input: || r#"let a = source-matches(from="global", span="span", pattern="fn", output="hit"); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::CrossFileSourceMatches { .. }
            ))
        }),
    },
    Case {
        name: CaseName("source_matches_requires_line_pair"),
        word: Word::Refuses,
        input: || r#"let a = source-matches(from="function", span="span", pattern="fn", output="hit", base-line="line"); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::SourceMatchesLinePair { .. }))
        }),
    },
    Case {
        name: CaseName("source_matches_requires_capture_pair"),
        word: Word::Refuses,
        input: || r#"let a = source-matches(from="function", span="span", pattern="fn", output="hit", capture="group"); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::SourceMatchesCapturePair { .. }
            ))
        }),
    },
    Case {
        name: CaseName("source_matches_requires_declared_capture"),
        word: Word::Refuses,
        input: || r#"let a = source-matches(from="function", span="span", pattern="fn", output="hit", capture="group", capture-output="captured"); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(
                error,
                LoweringError::SourceMatchesCaptureMissing { .. }
            ))
        }),
    },
    Case {
        name: CaseName("span_between_rejects_extra_arguments"),
        word: Word::Refuses,
        input: || r#"let a = span-between(from="function", start="span", end="span", output="hit", extra="span"); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::SpanArgument { .. }))
        }),
    },
    Case {
        name: CaseName("span_between_rejects_project_input"),
        word: Word::Refuses,
        input: || r#"let a = span-between(from="global", start="span", end="span", output="hit"); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::CrossFileSpan { .. }))
        }),
    },
    Case {
        name: CaseName("ambiguous_output_map_refused"),
        word: Word::Refuses,
        input: || r#"let a = ordered-match(left="source-location", right="source-location", left-on=[file], right-on=[file], left-order="start-byte", right-order="end-byte", output={file: [left,file], file: [right,file]}); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::DuplicateOutput { .. }))
        }),
    },
    Case {
        name: CaseName("replace_appends_text"),
        word: Word::Succeeds,
        input: || r#"let a = replace(from="function", text="name", pattern="_", replacement="-", output="label"); emit a subject file key(label);"#,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("empty_regex_and_replacement_are_constants"),
        word: Word::Succeeds,
        input: || r#"let a = replace(from="function", text="name", pattern="", replacement="", output="label"); emit a subject file key(label);"#,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("invalid_regex_refused_without_rows"),
        word: Word::Refuses,
        input: || r#"let a = replace(from="function", text="name", pattern="[", replacement="", output="label"); emit a subject file key(label);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::InvalidRegex { .. }))
        }),
    },
    Case {
        name: CaseName("numeric_text_refused"),
        word: Word::Refuses,
        input: || r#"let a = replace(from="source-location", text="start-byte", pattern="a", replacement="", output="label"); emit a subject file key(label);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
    Case {
        name: CaseName("output_collision_refused"),
        word: Word::Refuses,
        input: || r#"let a = replace(from="function", text="name", pattern="a", replacement="", output="name"); emit a subject file key(name);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::DuplicateOutput { .. }))
        }),
    },
    Case {
        name: CaseName("explicit_projection_and_numeric_order"),
        word: Word::Succeeds,
        input: || r#"let a = ordered-match(left="source-location", right="source-location", left-on=[file], right-on=[file], left-order="start-byte", right-order="end-byte", output={file: [left,file], span: [right,span]}); emit a subject file key(span);"#,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("text_order_refused"),
        word: Word::Refuses,
        input: || r#"let a = ordered-match(left="function", right="function", left-on=[file], right-on=[file], left-order="name", right-order="name", output={file: [left,file]}); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
    Case {
        name: CaseName("unlike_group_keys_refused"),
        word: Word::Refuses,
        input: || r#"let a = ordered-match(left="source-location", right="source-location", left-on=[file], right-on=["start-byte"], left-order="start-byte", right-order="end-byte", output={file: [left,file]}); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
    Case {
        name: CaseName("unequal_key_counts_refused"),
        word: Word::Refuses,
        input: || r#"let a = ordered-match(left="source-location", right="source-location", left-on=[file], right-on=[], left-order="start-byte", right-order="end-byte", output={file: [left,file]}); emit a subject file key(file);"#,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LoweringError::KindMismatch { .. }))
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
