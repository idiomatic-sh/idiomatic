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

fn project_keeps_exact_file_column() {
    let mut builder = lowering_builder();
    let call = call(
        "project",
        vec![
            ("from", text("function")),
            ("columns", syntax::ArgumentValue::List(vec![word("file")])),
        ],
    );
    super::service::lower_call(&mut builder, &name(), &call).unwrap();
    assert_eq!(
        builder.steps[0].operation,
        plans::Operation::Project {
            from: parts::Input::Relation(Relation::Function),
            columns: vec![parts::ColumnIndex(1)],
            distinct: parts::RepeatedRows::Dropped,
        }
    );
    assert_eq!(
        builder.lowered[0].fields[0].name,
        syntax::ColumnName("file".into())
    );
}
fn unknown_call_keeps_operation_and_span() {
    let mut builder = lowering_builder();
    let call = call("unknown", vec![]);
    assert_eq!(
        super::service::lower_call(&mut builder, &name(), &call),
        Err(LoweringError::BadArgument {
            operation: "unknown".into(),
            argument: String::new(),
            reason: crate::domain::lowering::types::ArgumentFault::NoSuchOperation,
            span: call.span,
        })
    );
    assert!(builder.steps.is_empty());
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("project_keeps_exact_file_column"),
        word: Word::Succeeds,
        input: || project_keeps_exact_file_column,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unknown_call_keeps_operation_and_span"),
        word: Word::Succeeds,
        input: || unknown_call_keeps_operation_and_span,
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
