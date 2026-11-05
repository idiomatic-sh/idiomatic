//! The public audit operation preserves the suite boundary's inventory and refusals.
use super::{interface::AuditUseCase as _, use_case::AuditUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::fixtures::fake::SuiteFake;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
use std::sync::Arc;
type Input = bool;
type Value = usize;
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    InvalidInventory,
    Unexpected,
}
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the boundary inventory remains unauthored"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("an empty default inventory refuses"),
        word: Word::Refuses,
        input: || false,
        expect: Expect::Error(Refusal::InvalidInventory),
    },
];
fn subject(nonempty: Input) -> Result<Value, Failure> {
    let tool = Tool {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::workloads::types::ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context: None,
        file_observation: None,
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec!["pin".into()],
        defaults: if nonempty {
            vec!["A".try_into().expect("rule")]
        } else {
            vec![]
        },
        deferred: std::collections::BTreeMap::default(),
        pending: std::collections::BTreeMap::default(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        project_search_roots: None,
    };
    AuditUseCaseImpl::new(Arc::new(SuiteFake::default()))
        .audit(std::path::Path::new("."), &tool)
        .map(|audit| audit.authored_count().0)
        .map_err(|error| match error {
            ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(message)
                if message == "empty or duplicated default rule inventory" =>
            {
                Refusal::InvalidInventory
            }
            ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(_)
            | ryo_benchmark_contracts::domain::reports::errors::ReportsError::Io(_)
            | ryo_benchmark_contracts::domain::reports::errors::ReportsError::Json(_)
            | ryo_benchmark_contracts::domain::reports::errors::ReportsError::Toml(_) => {
                Refusal::Unexpected
            }
        })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
