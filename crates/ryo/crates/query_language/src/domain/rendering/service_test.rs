//! Domain behavior from typed values, independent of parsing and adapters.
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use crate::domain::lowering::errors::LoweringError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::program::types as syntax;
use ryo_values::domain::query_syntax::types as ast;
use ryo_values::domain::query_tokens::types as tokens;
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

type Lowering = fn(
    &mut crate::domain::lowering::types::Builder<'_>,
    &tokens::StepName,
    &ast::Call,
) -> Result<(), LoweringError>;

/// The refusal a call to `lower` meets, over the relations of one file and the manifests of the project.
fn refusal_of(lower: Lowering, call: &ast::Call) -> LoweringError {
    let mut builder = lowering_builder();
    builder.catalog = &[
        Relation::Function,
        Relation::SyntaxChild,
        Relation::SemanticChild,
        Relation::PackageDependency,
    ];
    lower(&mut builder, &name(), call).expect_err("the call is refused")
}

fn call(name: &str, arguments: Vec<(&str, ast::ArgumentValue)>) -> ast::Call {
    ast::Call {
        name: tokens::OperationName(name.into()),
        span: lowering_builder().span,
        arguments: arguments
            .into_iter()
            .map(|(name, value)| ast::Argument {
                name: tokens::ArgumentName(name.into()),
                value,
            })
            .collect(),
    }
}
fn text(value: &str) -> ast::ArgumentValue {
    ast::ArgumentValue::Text(tokens::WrittenLiteral(value.into()))
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

const CASES: &[Case<Input, Value, Failure>] = &[
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
];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
