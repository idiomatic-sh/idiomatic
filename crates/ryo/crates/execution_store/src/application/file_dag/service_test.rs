//! The store's one operation: a scope opens its file generation's relation owner, which keeps the reuse the scope was
//! opened with.
use super::relations::types::ReuseEligibility;
use super::service::open_program_owner;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;

type Failure = OperatingError;

/// The reuse eligibility of the owner a scope opens over a compiled one-rule program with `reuse`.
fn opened(reuse: ReuseEligibility) -> Result<ReuseEligibility, Failure> {
    // A one-rule program of one region.
    let owner = open_program_owner(
        super::relations::types::ProducerCapacity(1),
        super::fixtures::program_derived_port(),
        reuse,
        super::route_counts::types::RouteProfile::default(),
    );
    Ok(owner.stable())
}

const CASES: &[Case<ReuseEligibility, ReuseEligibility, Failure>] = &[
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
    run(CASES, Word::Succeeds, opened);
}
