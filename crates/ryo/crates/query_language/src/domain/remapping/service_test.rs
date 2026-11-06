//! A fragment's parameters are replaced by what its call binds them to, and a parameter nothing binds is refused.

use super::service::remap_operand;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::binding::types::{Bindings, FragmentBindingKey};
use crate::domain::linking::errors::LinkingError;
use crate::selects;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::program::types::{
    ArgumentValue, ByteOffset, QuerySourceId, QuerySpan, WrittenOperand, WrittenParameter,
};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// An operand reads a parameter the call binds.
    Bound,
    /// An operand reads a parameter the call does not bind.
    Unbound,
}

type Value = ();
type Failure = LinkingError;
/// What a row asks of the service.
type Input = Scenario;

const SPAN: QuerySpan = QuerySpan {
    start: ByteOffset(3),
    end: ByteOffset(11),
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_bound_parameter_operand_is_remapped"),
        word: Word::Succeeds,
        input: || Scenario::Bound,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_unbound_parameter_operand_is_refused_at_its_filter"),
        word: Word::Refuses,
        input: || Scenario::Unbound,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LinkingError::Located { origin, .. } if origin.span == SPAN))
        }),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut bindings = Bindings::default();
    if matches!(scenario, Scenario::Bound) {
        bindings.0.insert(
            FragmentBindingKey("limit".into()),
            ArgumentValue::Number(Integer(3)),
        );
    }
    let mut operand = WrittenOperand::Parameter(WrittenParameter("limit".into()));
    remap_operand(
        &mut operand,
        &bindings,
        SPAN,
        &QuerySourceId(Box::from("module")),
        &[],
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
