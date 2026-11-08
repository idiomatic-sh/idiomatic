//! Public preparation accepts agreeing requests and rejects structurally mismatched ones.
use super::{
    interface::PrepareUseCase,
    types::{PlanSource, PrepareRequest},
    use_case::PrepareUseCaseImpl,
};
use ryo_execution_runtime::application::file_dag::program::prepared_execution::interface::PreparedExecution;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::rule_intent::errors::RuleIntentError;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
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
            RuleIntentError::InvalidReference,
        )),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let prepare = PrepareUseCaseImpl::new(
        Arc::new(
            ryo_execution_runtime::outbound::operators::factory::use_case(ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(Arc::new(
                ryo_operators::outbound::matching::factory::use_case(),
            ))),
        ),
        Arc::new(ryo_execution_store::outbound::derived::factory::semantics()),
        Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    );
    prepare.prepare(input).map(|program| program.rule_count().0)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
