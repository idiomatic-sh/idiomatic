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

fn closure_keeps_endpoint_columns() {
    let mut builder = lowering_builder();
    let call = call(
        "closure",
        vec![
            ("edges", text("semantic-child")),
            ("source", word("parent")),
            ("destination", word("child")),
        ],
    );
    super::service::call_closure(&mut builder, &name(), &call, plans::QueryOperator::Closure)
        .unwrap();
    assert_eq!(
        builder.steps[0].operation,
        plans::Operation::Closure {
            edges: parts::Input::Relation(Relation::SemanticChild),
            keys: vec![],
            source: parts::ColumnIndex(0),
            destination: parts::ColumnIndex(1)
        }
    );
}
fn endpoint_cannot_also_be_a_partition_key() {
    let mut builder = lowering_builder();
    let call = call(
        "closure",
        vec![
            ("edges", text("semantic-child")),
            ("source", word("parent")),
            ("destination", word("child")),
            ("keys", syntax::ArgumentValue::List(vec![word("parent")])),
        ],
    );
    assert_eq!(
        super::service::call_closure(&mut builder, &name(), &call, plans::QueryOperator::Closure),
        Err(LoweringError::DuplicateOutput {
            name: "parent".into(),
            span: call.span
        })
    );
    assert!(builder.steps.is_empty());
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("closure_keeps_endpoint_columns"),
        word: Word::Succeeds,
        input: || closure_keeps_endpoint_columns,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("endpoint_cannot_also_be_a_partition_key"),
        word: Word::Succeeds,
        input: || endpoint_cannot_also_be_a_partition_key,
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
