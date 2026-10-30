//! What the schemas of this command accept and refuse, one row of `CASES` for each input.

use std::borrow::Cow;

use ryo_core::inbound::use_cases::list_operators::types::{ListOperatorsReport, OperatorSummary};

use ryo_values::domain::descriptions::types::{OperatorName, Purpose};

use super::AnswerSchema;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = ListOperatorsReport;
/// What the answer prints.
type Value = Cow<'static, str>;

/// No conversion of this command refuses: its report always becomes an answer.
type Failure = std::convert::Infallible;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_operator_prints_as_its_name_a_tab_and_its_summary"),
        word: Word::Succeeds,
        input: || ListOperatorsReport {
            operators: vec![OperatorSummary {
                name: OperatorName("join".into()),
                summary: Purpose("joins two streams".into()),
            }],
        },
        expect: Expect::Value(Cow::Borrowed("join\tjoins two streams\n")),
    },
    Case {
        name: CaseName("operators_print_one_to_a_line_in_the_order_given"),
        word: Word::Succeeds,
        input: || ListOperatorsReport {
            operators: vec![
                OperatorSummary {
                    name: OperatorName("join".into()),
                    summary: Purpose("joins".into()),
                },
                OperatorSummary {
                    name: OperatorName("select".into()),
                    summary: Purpose("picks".into()),
                },
            ],
        },
        expect: Expect::Value(Cow::Borrowed("join\tjoins\nselect\tpicks\n")),
    },
];

/// Converts the report into the answer and returns the text it prints.
fn subject(report: Input) -> Value {
    Cow::Owned(AnswerSchema::from(report).to_string())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
