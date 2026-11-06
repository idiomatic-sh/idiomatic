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
    syntax::StepName(
        "result".into(),
        ryo_values::domain::program::types::StepIdentified::Unassigned,
    )
}

fn group_count_keeps_key_and_count_column() {
    let mut builder = lowering_builder();
    let call = call(
        "group-count",
        vec![
            ("from", text("function")),
            ("keys", syntax::ArgumentValue::List(vec![word("file")])),
            ("count", word("n")),
        ],
    );
    super::service::call_group_count(&mut builder, &name(), &call).unwrap();
    assert_eq!(
        builder.steps[0].operation,
        plans::Operation::GroupCount {
            from: parts::Input::Relation(Relation::Function),
            keys: vec![parts::ColumnIndex(2)],
            distinct: None
        }
    );
    assert_eq!(
        builder.lowered[0].fields[1].name,
        syntax::ColumnName("n".into())
    );
    assert_eq!(
        builder.lowered[0].fields[1].kind,
        ryo_relations::domain::relations::types::CellKind::Number
    );
}
fn missing_key_names_exact_column() {
    let mut builder = lowering_builder();
    let call = call(
        "group-count",
        vec![
            ("from", text("function")),
            ("keys", syntax::ArgumentValue::List(vec![word("missing")])),
            ("count", word("n")),
        ],
    );
    assert_eq!(
        super::service::call_group_count(&mut builder, &name(), &call),
        Err(LoweringError::UnknownColumn {
            name: "missing".into(),
            span: call.span
        })
    );
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("group_count_keeps_key_and_count_column"),
        word: Word::Succeeds,
        input: || group_count_keeps_key_and_count_column,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("missing_key_names_exact_column"),
        word: Word::Succeeds,
        input: || missing_key_names_exact_column,
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
