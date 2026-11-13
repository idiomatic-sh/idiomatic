//! A call's arguments are read as the types their parameters declare, and bound parameters are substituted.

use super::service::{scalar_matches, substitute_value};
use super::types::{Bindings, FragmentBindingKey, ScalarMatch};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::query_syntax::types::ArgumentValue;
use ryo_values::domain::query_tokens::types::{TypeName, WrittenLiteral, WrittenParameter};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A number given for an `integer` parameter.
    NumberForInteger,
    /// A text given for an `integer` parameter.
    TextForInteger,
    /// A parameter the call binds.
    BoundParameter,
    /// A parameter the call does not bind.
    UnboundParameter,
}

/// Whether the answer is the one the row expects of the value.
type Value = bool;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_number_is_read_as_an_integer_parameter"),
        word: Word::Succeeds,
        input: || Scenario::NumberForInteger,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_text_is_not_read_as_an_integer_parameter"),
        word: Word::Succeeds,
        input: || Scenario::TextForInteger,
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("a_bound_parameter_is_replaced_by_its_value"),
        word: Word::Succeeds,
        input: || Scenario::BoundParameter,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_unbound_parameter_is_left_as_written"),
        word: Word::Succeeds,
        input: || Scenario::UnboundParameter,
        expect: Expect::Value(true),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let integer = TypeName("integer".into());
    let parameter = ArgumentValue::Parameter(WrittenParameter("limit".into()));
    let mut bindings = Bindings::default();
    bindings.0.insert(
        FragmentBindingKey("limit".into()),
        ArgumentValue::Number(Integer(3)),
    );
    Ok(match scenario {
        Scenario::NumberForInteger => {
            scalar_matches(&integer, &ArgumentValue::Number(Integer(1)), &[])
                == ScalarMatch::Matches
        }
        Scenario::TextForInteger => {
            scalar_matches(
                &integer,
                &ArgumentValue::Text(WrittenLiteral("x".into())),
                &[],
            ) == ScalarMatch::Matches
        }
        Scenario::BoundParameter => {
            substitute_value(&parameter, &bindings) == ArgumentValue::Number(Integer(3))
        }
        Scenario::UnboundParameter => {
            substitute_value(&parameter, &Bindings::default()) == parameter
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
