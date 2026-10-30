//! The `operating` service's cases, on values alone.

use super::errors::OperatingError;
use super::service::{family_of, first, second, third};
use super::types::Family;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_model::types::{Rows, Table};
use ryo_values::domain::plan_parts::types::Test;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// Which family a test belongs to.
    FamilyOfATest,
    /// The first of two inputs.
    FirstOfTwo,
    /// The third of two inputs.
    ThirdOfTwo,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// A count, or whether the answer was right.
type Value = usize;
type Failure = OperatingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_test_belongs_to_its_family"),
        word: Word::Succeeds,
        input: || Scenario::FamilyOfATest,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_first_and_second_of_two_inputs_are_there"),
        word: Word::Succeeds,
        input: || Scenario::FirstOfTwo,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_input_a_step_does_not_have_is_missing"),
        word: Word::Refuses,
        input: || Scenario::ThirdOfTwo,
        expect: Expect::Error(OperatingError::MissingInput { position: 3 }),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let (left, right) = (Table::default(), Table::default());
    let inputs = [Rows::Computed(&left), Rows::Computed(&right)];
    match scenario {
        Scenario::FamilyOfATest => Ok(usize::from(family_of(Test::NotGlobAny) == Family::Glob)),
        Scenario::FirstOfTwo => {
            let both = (first(&inputs)?, second(&inputs)?);
            Ok(usize::from(matches!(
                both,
                (Rows::Computed(_), Rows::Computed(_))
            )))
        }
        Scenario::ThirdOfTwo => third(&inputs).map(|_| 0),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
