//! The store's one operation: a scope opens its file generation's relation owner, which keeps the reuse the scope was
//! opened with.
use super::service::open_program_owner;
use crate::application::file_dag::relations::types::ReuseEligibility;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;

type Input = ReuseEligibility;
type Value = ReuseEligibility;
type Failure = OperatingError;

/// The reuse eligibility of the owner a scope opens over a compiled one-rule program with `reuse`.
fn subject(reuse: ReuseEligibility) -> Result<ReuseEligibility, Failure> {
    // A one-rule program of one region.
    let owner = open_program_owner(
        crate::application::file_dag::relations::types::ProducerCapacity(1),
        crate::application::file_dag::fixtures::program_derived_port(),
        reuse,
        crate::application::file_dag::route_counts::profile::service::RouteProfile::default(),
    );
    Ok(owner.stable())
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "an eligible scope opens an owner whose publications a later request may reuse",
        ),
        word: Word::Succeeds,
        input: || ReuseEligibility::Eligible,
        expect: Expect::Value(ReuseEligibility::Eligible),
    },
    Case {
        name: CaseName(
            "an ineligible scope opens an owner whose publications no later request reuses",
        ),
        word: Word::Succeeds,
        input: || ReuseEligibility::Ineligible,
        expect: Expect::Value(ReuseEligibility::Ineligible),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
