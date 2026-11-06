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
fn name() -> syntax::StepName {
    syntax::StepName(
        "result".into(),
        ryo_values::domain::program::types::StepIdentified::Unassigned,
    )
}

fn all_set_operations_keep_both_typed_inputs() {
    for (operator, expected) in [
        (
            plans::QueryOperator::Union,
            plans::Operation::Union {
                left: parts::Input::Relation(Relation::Function),
                right: parts::Input::Relation(Relation::Function),
            },
        ),
        (
            plans::QueryOperator::Intersection,
            plans::Operation::Intersection {
                left: parts::Input::Relation(Relation::Function),
                right: parts::Input::Relation(Relation::Function),
            },
        ),
        (
            plans::QueryOperator::Difference,
            plans::Operation::Difference {
                left: parts::Input::Relation(Relation::Function),
                right: parts::Input::Relation(Relation::Function),
            },
        ),
    ] {
        let mut builder = lowering_builder();
        let call = call(
            "union",
            vec![("left", text("function")), ("right", text("function"))],
        );
        super::service::call_set(&mut builder, &name(), &call, operator).unwrap();
        assert_eq!(builder.steps[0].operation, expected);
    }
}
fn incompatible_inputs_keep_typed_refusal() {
    let mut builder = lowering_builder();
    let call = call(
        "union",
        vec![("left", text("function")), ("right", text("syntax-child"))],
    );
    assert_eq!(
        super::service::call_set(&mut builder, &name(), &call, plans::QueryOperator::Union),
        Err(LoweringError::BadArgument {
            operation: "union".into(),
            argument: "right".into(),
            reason: crate::domain::lowering::types::ArgumentFault::SameColumns,
            span: call.span,
        })
    );
    assert!(builder.steps.is_empty());
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("all_set_operations_keep_both_typed_inputs"),
        word: Word::Succeeds,
        input: || all_set_operations_keep_both_typed_inputs,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("incompatible_inputs_keep_typed_refusal"),
        word: Word::Succeeds,
        input: || incompatible_inputs_keep_typed_refusal,
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
