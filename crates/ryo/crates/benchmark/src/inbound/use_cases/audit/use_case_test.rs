//! The public audit operation preserves the suite boundary's inventory and refusals.
use super::{interface::AuditUseCase, use_case::AuditUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::types::Tool;
use crate::outbound::suite::fixtures::fake::SuiteFake;
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
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec!["pin".into()],
        defaults: if nonempty {
            vec!["A".try_into().expect("rule")]
        } else {
            vec![]
        },
        deferred: Default::default(),
        pending: Default::default(),
    };
    AuditUseCaseImpl::new(Arc::new(SuiteFake::default()))
        .audit(std::path::Path::new("."), &tool)
        .map(|audit| audit.authored_count().0)
        .map_err(|error| match error {
            crate::domain::diagnostics::errors::DiagnosticsError::Invalid(message)
                if message == "empty or duplicated default rule inventory" =>
            {
                Refusal::InvalidInventory
            }
            _ => Refusal::Unexpected,
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
