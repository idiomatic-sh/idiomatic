//! A definition's fragment contracts are checked against the plan under construction: a contract with no input is
//! admitted; an input that is no relation, and a returned step the lowering did not make, are refused at the fragment.

use super::service::{check_fragment_inputs, check_fragment_output};
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use crate::domain::lowering::errors::LoweringError;
use crate::selects;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::program::types::{
    ExpandedDefinitions, FragmentContract, FragmentContractInput, RelationSignature,
};
use ryo_values::domain::query_syntax::types::ArgumentValue;
use ryo_values::domain::query_tokens::types::{
    ByteOffset, QueryOrigin, QuerySourceId, QuerySpan, StepIdentity, WrittenName,
};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A contract that binds no input.
    NoInput,
    /// A contract whose one input is bound to a number.
    NumberInput,
    /// A contract whose returned step the plan does not hold.
    MissingResult,
}

type Input = Scenario;
type Value = ();
type Failure = LoweringError;

const SPAN: QuerySpan = QuerySpan {
    start: ByteOffset(2),
    end: ByteOffset(5),
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_contract_with_no_input_is_admitted"),
        word: Word::Succeeds,
        input: || Scenario::NoInput,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_input_bound_to_a_number_is_refused_at_the_fragment"),
        word: Word::Refuses,
        input: || Scenario::NumberInput,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, LoweringError::FragmentContract { origin, .. } if origin.span == SPAN)
            )
        }),
    },
    Case {
        name: CaseName("a_returned_step_not_made_is_refused_at_the_fragment"),
        word: Word::Refuses,
        input: || Scenario::MissingResult,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, LoweringError::FragmentContract { origin, .. } if origin.source_id.0.as_ref() == "query/kinds")
            )
        }),
    },
];

/// A contract of `query/kinds` with `inputs`.
fn contract(inputs: Vec<FragmentContractInput>) -> FragmentContract {
    FragmentContract {
        first: StepIdentity(1),
        result: StepIdentity(1),
        inputs,
        output: RelationSignature {
            columns: Vec::new(),
            span: SPAN,
        },
        output_span: SPAN,
        origin: QueryOrigin {
            source_id: QuerySourceId("query/kinds".into()),
            span: SPAN,
            trace: [].into(),
        },
        statements: ExpandedDefinitions(1),
        parameters: Vec::new(),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let builder = lowering_builder();
    match scenario {
        Scenario::NoInput => check_fragment_inputs(&builder, &contract(Vec::new())),
        Scenario::NumberInput => check_fragment_inputs(
            &builder,
            &contract(vec![FragmentContractInput {
                name: WrittenName("rows".into()),
                value: ArgumentValue::Number(Integer(1)),
                relation: RelationSignature {
                    columns: Vec::new(),
                    span: SPAN,
                },
                span: SPAN,
            }]),
        ),
        Scenario::MissingResult => check_fragment_output(&builder, &contract(Vec::new())),
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
