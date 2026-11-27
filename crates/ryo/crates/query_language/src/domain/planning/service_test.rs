//! Domain behavior from typed values, independent of parsing and adapters.
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

fn projected_file_key_controls_scope() {
    let builder = lowering_builder();
    let input = parts::Input::Relation(Relation::Function);
    let scoped = super::service::scoped(&builder, input);
    let fields = scoped.fields.clone();
    for (column, expected) in [
        (2, ryo_values::domain::plan_parts::types::StepScope::PerFile),
        (
            1,
            ryo_values::domain::plan_parts::types::StepScope::CrossFile,
        ),
    ] {
        let operation = plans::Operation::Project {
            from: input,
            columns: vec![parts::ColumnIndex(column)],
            distinct: parts::RepeatedRows::Dropped,
        };
        assert_eq!(
            super::service::scope_of(
                &operation,
                std::slice::from_ref(&scoped),
                &[fields[usize::try_from(column).expect("a u32 fits a usize")].clone()]
            ),
            expected
        );
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("projected_file_key_controls_scope"),
        word: Word::Succeeds,
        input: || projected_file_key_controls_scope,
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
