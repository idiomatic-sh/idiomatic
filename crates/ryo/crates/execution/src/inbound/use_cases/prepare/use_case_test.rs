//! Public preparation accepts agreeing requests and rejects structurally mismatched ones.
use super::{
    interface::PrepareUseCase,
    types::{PlanSource, PrepareRequest},
    use_case::PrepareUseCaseImpl,
};
use crate::application::file_dag::program::prepared_execution::interface::PreparedExecution;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::file_program::errors::FileProgramError;
use crate::domain::operating::errors::OperatingError;
use crate::domain::prepared_request::types::ObservationIdentity;
use std::sync::Arc;

type Input = PrepareRequest;
type Value = usize;
type Failure = OperatingError;

fn request(bindings: Vec<Vec<ryo_values::domain::query_bindings::types::ParameterBinding>>) -> Input {
    PrepareRequest {
        plans: PlanSource::Authored {
            plans: vec![],
            bindings,
        },
        rules: vec![],
        digests: vec![],
        hosted: vec![],
        observations: ObservationIdentity::Independent,
    }
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(an_empty_request_prepares_no_rule)),
        word: Word::Succeeds,
        input: || request(vec![]),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName(stringify!(a_binding_population_without_a_rule_is_refused)),
        word: Word::Refuses,
        input: || request(vec![vec![]]),
        expect: Expect::Error(OperatingError::ObservationConfiguration(
            FileProgramError::InvalidReference,
        )),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let prepare = PrepareUseCaseImpl::new(
        Arc::new(
            crate::outbound::operators::factory::use_case(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(Arc::new(
                ryo_operators::outbound::matching::factory::use_case(),
            ))),
        ),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    );
    prepare.prepare(input).map(|program| program.rule_count())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
