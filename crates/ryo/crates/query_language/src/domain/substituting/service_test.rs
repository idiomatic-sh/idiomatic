//! Domain behavior from typed values, independent of parsing and adapters.
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use crate::domain::lowering::errors::LoweringError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::program::types as syntax;
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

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
fn name() -> syntax::StepName {
    syntax::StepName(
        "result".into(),
        ryo_values::domain::program::types::StepIdentified::Unassigned,
    )
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
