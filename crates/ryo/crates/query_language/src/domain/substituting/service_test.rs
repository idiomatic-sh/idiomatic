//! Domain behavior from typed values, independent of parsing and adapters.
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use crate::domain::lowering::errors::LoweringError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::program::types as syntax;
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

fn call(name: &str, arguments: Vec<(&str, syntax::ArgumentValue)>) -> syntax::Call {
    syntax::Call {
        name: syntax::OperationName(name.into()),
        span: lowering_builder().span,
        arguments: arguments
            .into_iter()
            .map(|(name, value)| syntax::Argument {
                name: syntax::ArgumentName(name.into()),
                value,
            })
            .collect(),
    }
}
fn text(value: &str) -> syntax::ArgumentValue {
    syntax::ArgumentValue::Text(syntax::WrittenLiteral(value.into()))
}
fn word(value: &str) -> syntax::ArgumentValue {
    syntax::ArgumentValue::Word(syntax::WordText(value.into()))
}
fn name() -> syntax::StepName {
    syntax::StepName("result".into(), None)
}

fn span_bounds_preserves_operand_and_appends_numbers() {
    let mut builder = lowering_builder();
    let call = call(
        "span-bounds",
        vec![
            ("from", text("function")),
            ("span", word("span")),
            ("start-output", word("start")),
            ("end-output", word("end")),
        ],
    );
    super::service::call_span_bounds(&mut builder, &name(), &call).unwrap();
    assert_eq!(
        builder.steps[0].operation,
        plans::Operation::SpanBounds {
            from: parts::Input::Relation(Relation::Function),
            span: parts::ColumnIndex(3)
        }
    );
    let fields = &builder.lowered[0].fields;
    assert_eq!(
        fields[fields.len() - 2].name,
        syntax::ColumnName("start".into())
    );
    assert_eq!(
        fields[fields.len() - 1].name,
        syntax::ColumnName("end".into())
    );
}
fn non_span_operand_preserves_refusal_location() {
    let mut builder = lowering_builder();
    let call = call(
        "span-bounds",
        vec![
            ("from", text("function")),
            ("span", word("name")),
            ("start-output", word("start")),
            ("end-output", word("end")),
        ],
    );
    assert_eq!(
        super::service::call_span_bounds(&mut builder, &name(), &call),
        Err(LoweringError::KindMismatch { span: call.span })
    );
    assert!(builder.steps.is_empty());
}

type Lowering = fn(
    &mut crate::domain::lowering::types::Builder<'_>,
    &syntax::StepName,
    &syntax::Call,
) -> Result<(), LoweringError>;

/// The refusal a call to `lower` meets, over the relations of one file and the manifests of the project.
fn refusal_of(lower: Lowering, call: &syntax::Call) -> LoweringError {
    let mut builder = lowering_builder();
    builder.catalog = &[
        Relation::Function,
        Relation::SyntaxChild,
        Relation::SemanticChild,
        Relation::PackageDependency,
    ];
    lower(&mut builder, &name(), call).expect_err("the call is refused")
}

fn semantic_call_rejects_extra_arguments() {
    let call = call(
        "render-syntax",
        vec![
            ("from", text("function")),
            ("node", text("id")),
            ("context", text("assignment-target")),
            ("output", text("rendered")),
            ("extra", text("id")),
        ],
    );
    assert!(matches!(
        refusal_of(super::service::call_render_syntax, &call),
        LoweringError::SemanticArgument { .. }
    ));
}
fn semantic_call_rejects_project_input() {
    let call = call(
        "render-syntax",
        vec![
            ("from", text("package-dependency")),
            ("node", text("id")),
            ("context", text("assignment-target")),
            ("output", text("rendered")),
        ],
    );
    assert!(matches!(
        refusal_of(super::service::call_render_syntax, &call),
        LoweringError::CrossFileSemantic { .. }
    ));
}
fn semantic_call_rejects_unknown_context() {
    let call = call(
        "render-syntax",
        vec![
            ("from", text("function")),
            ("node", text("id")),
            ("context", text("not-a-context")),
            ("output", text("rendered")),
        ],
    );
    assert!(matches!(
        refusal_of(super::service::call_render_syntax, &call),
        LoweringError::InvalidSemanticLiteral { .. }
    ));
}
fn span_bounds_rejects_extra_arguments() {
    let call = call(
        "span-bounds",
        vec![
            ("from", text("function")),
            ("span", text("span")),
            ("start-output", text("begin")),
            ("end-output", text("finish")),
            ("extra", text("span")),
        ],
    );
    assert!(matches!(
        refusal_of(super::service::call_span_bounds, &call),
        LoweringError::SpanBoundsArgument { .. }
    ));
}
fn source_matches_call(extra: Vec<(&str, syntax::ArgumentValue)>, from: &str) -> syntax::Call {
    let mut arguments = vec![
        ("from", text(from)),
        ("span", text("span")),
        ("pattern", text("fn")),
        ("output", text("hit")),
    ];
    arguments.extend(extra);
    call("source-matches", arguments)
}
fn source_matches_rejects_extra_arguments() {
    let call = source_matches_call(vec![("extra", text("span"))], "function");
    assert!(matches!(
        refusal_of(super::service::call_source_matches, &call),
        LoweringError::SourceMatchesArgument { .. }
    ));
}
fn source_matches_rejects_project_input() {
    let call = source_matches_call(vec![], "package-dependency");
    assert!(matches!(
        refusal_of(super::service::call_source_matches, &call),
        LoweringError::CrossFileSourceMatches { .. }
    ));
}
fn source_matches_requires_both_line_arguments_or_neither() {
    let call = source_matches_call(vec![("base-line", text("line"))], "function");
    assert!(matches!(
        refusal_of(super::service::call_source_matches, &call),
        LoweringError::SourceMatchesLinePair { .. }
    ));
}
fn source_matches_capture_needs_its_output() {
    let call = source_matches_call(vec![("capture", text("group"))], "function");
    assert!(matches!(
        refusal_of(super::service::call_source_matches, &call),
        LoweringError::SourceMatchesCapturePair { .. }
    ));
}
fn source_matches_capture_must_be_named_by_the_pattern() {
    let call = source_matches_call(
        vec![
            ("capture", text("group")),
            ("capture-output", text("captured")),
        ],
        "function",
    );
    assert!(matches!(
        refusal_of(super::service::call_source_matches, &call),
        LoweringError::SourceMatchesCaptureMissing { .. }
    ));
}
fn span_between_rejects_extra_arguments() {
    let call = call(
        "span-between",
        vec![
            ("from", text("function")),
            ("start", text("span")),
            ("end", text("span")),
            ("output", text("hit")),
            ("extra", text("span")),
        ],
    );
    assert!(matches!(
        refusal_of(super::service::call_span_between, &call),
        LoweringError::SpanArgument { .. }
    ));
}
fn span_between_rejects_project_input() {
    let call = call(
        "span-between",
        vec![
            ("from", text("package-dependency")),
            ("start", text("span")),
            ("end", text("span")),
            ("output", text("hit")),
        ],
    );
    assert!(matches!(
        refusal_of(super::service::call_span_between, &call),
        LoweringError::CrossFileSpan { .. }
    ));
}
fn replace_rejects_a_pattern_that_is_not_a_regex() {
    let call = call(
        "replace",
        vec![
            ("from", text("function")),
            ("text", text("name")),
            ("pattern", text("[")),
            ("replacement", text("")),
            ("output", text("label")),
        ],
    );
    assert!(matches!(
        refusal_of(super::service::call_replace, &call),
        LoweringError::InvalidRegex { .. }
    ));
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("span_bounds_preserves_operand_and_appends_numbers"),
        word: Word::Succeeds,
        input: || span_bounds_preserves_operand_and_appends_numbers,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("non_span_operand_preserves_refusal_location"),
        word: Word::Succeeds,
        input: || non_span_operand_preserves_refusal_location,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("semantic_call_rejects_extra_arguments"),
        word: Word::Succeeds,
        input: || semantic_call_rejects_extra_arguments,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("semantic_call_rejects_project_input"),
        word: Word::Succeeds,
        input: || semantic_call_rejects_project_input,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("semantic_call_rejects_unknown_context"),
        word: Word::Succeeds,
        input: || semantic_call_rejects_unknown_context,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("span_bounds_rejects_extra_arguments"),
        word: Word::Succeeds,
        input: || span_bounds_rejects_extra_arguments,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("source_matches_rejects_extra_arguments"),
        word: Word::Succeeds,
        input: || source_matches_rejects_extra_arguments,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("source_matches_rejects_project_input"),
        word: Word::Succeeds,
        input: || source_matches_rejects_project_input,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("source_matches_requires_both_line_arguments_or_neither"),
        word: Word::Succeeds,
        input: || source_matches_requires_both_line_arguments_or_neither,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("source_matches_capture_needs_its_output"),
        word: Word::Succeeds,
        input: || source_matches_capture_needs_its_output,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("source_matches_capture_must_be_named_by_the_pattern"),
        word: Word::Succeeds,
        input: || source_matches_capture_must_be_named_by_the_pattern,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("span_between_rejects_extra_arguments"),
        word: Word::Succeeds,
        input: || span_between_rejects_extra_arguments,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("span_between_rejects_project_input"),
        word: Word::Succeeds,
        input: || span_between_rejects_project_input,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("replace_rejects_a_pattern_that_is_not_a_regex"),
        word: Word::Succeeds,
        input: || replace_rejects_a_pattern_that_is_not_a_regex,
        expect: Expect::Value(()),
    },
];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
