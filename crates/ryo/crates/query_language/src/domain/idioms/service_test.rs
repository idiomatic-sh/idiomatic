//! Domain behavior from typed values, independent of parsing and adapters.
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

fn recognizer_requires_matching_function_columns() {
    use crate::domain::rewriting::types::SameSets;
    let mut builder = lowering_builder();
    builder.steps = vec![plans::PlanStep {
        scope: plans::StepScope::PerFile,
        operation: plans::Operation::Join {
            left: parts::Input::Step(parts::StepId(7)),
            right: parts::Input::Step(parts::StepId(7)),
            left_on: vec![parts::ColumnIndex(2)],
            right_on: vec![parts::ColumnIndex(2)],
            output: vec![
                parts::JoinColumn {
                    side: parts::Side::Left,
                    column: parts::ColumnIndex(0),
                },
                parts::JoinColumn {
                    side: parts::Side::Right,
                    column: parts::ColumnIndex(0),
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        },
    }];
    let idiom = SameSets {
        pairs: parts::StepId(0),
        function: parts::ColumnIndex(0),
        other: parts::ColumnIndex(1),
        texts: parts::StepId(8),
        owner: parts::ColumnIndex(0),
        text: parts::ColumnIndex(1),
    };
    let recognized = super::service::recognize_same_sets(&builder, idiom)
        .expect("same source and function columns");
    assert_eq!(recognized.left, parts::StepId(7));
    assert_eq!(recognized.first, parts::ColumnIndex(0));
    assert_eq!(recognized.left_on, vec![parts::ColumnIndex(2)]);
    let plans::Operation::Join { output, .. } = &mut builder.steps[0].operation else {
        unreachable!()
    };
    output[1].column = parts::ColumnIndex(1);
    assert!(super::service::recognize_same_sets(&builder, idiom).is_none());
}

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("recognizer_requires_matching_function_columns"),
    word: Word::Succeeds,
    input: || recognizer_requires_matching_function_columns,
    expect: Expect::Value(()),
}];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
