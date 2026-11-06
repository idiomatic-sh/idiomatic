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
fn word(value: &str) -> syntax::ArgumentValue {
    syntax::ArgumentValue::Word(syntax::WordText(value.into()))
}
fn name() -> syntax::StepName {
    syntax::StepName(
        "result".into(),
        ryo_values::domain::program::types::StepIdentified::Unassigned,
    )
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
        name: CaseName("span_bounds_rejects_extra_arguments"),
        word: Word::Succeeds,
        input: || span_bounds_rejects_extra_arguments,
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
];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
