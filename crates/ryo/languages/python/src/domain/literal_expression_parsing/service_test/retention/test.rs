//! Unavailable carriers, retained storage and independence from the other parser caches.

use super::fixtures::{
    literal_demand_does_not_initialize_other_parser_caches,
    missing_logical_carriers_are_unavailable_and_partial_not_malformed,
    retained_storage_is_outcomes_occurrences_and_complete_carrier_keys,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `missing_logical_carriers_are_unavailable_and_partial_not_malformed` scenario.
    MissingLogicalCarriersAreUnavailableAndPartialNotMalformed,
    /// The `retained_storage_is_outcomes_occurrences_and_complete_carrier_keys` scenario.
    RetainedStorageIsOutcomesOccurrencesAndCompleteCarrierKeys,
    /// The `literal_demand_does_not_initialize_other_parser_caches` scenario.
    LiteralDemandDoesNotInitializeOtherParserCaches,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// A scenario is never refused.
type Failure = ();

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("missing_logical_carriers_are_unavailable_and_partial_not_malformed"),
        word: Word::Succeeds,
        input: || Given::MissingLogicalCarriersAreUnavailableAndPartialNotMalformed,
        expect: Expect::Value("Partial|1|true|0"),
    },
    Case {
        name: CaseName("retained_storage_is_outcomes_occurrences_and_complete_carrier_keys"),
        word: Word::Succeeds,
        input: || Given::RetainedStorageIsOutcomesOccurrencesAndCompleteCarrierKeys,
        expect: Expect::Value("256|256|256|1|1|true|257"),
    },
    Case {
        name: CaseName("literal_demand_does_not_initialize_other_parser_caches"),
        word: Word::Succeeds,
        input: || Given::LiteralDemandDoesNotInitializeOtherParserCaches,
        expect: Expect::Value("Fresh|Fresh|Fresh|1"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::MissingLogicalCarriersAreUnavailableAndPartialNotMalformed => {
            missing_logical_carriers_are_unavailable_and_partial_not_malformed()
        }
        Given::RetainedStorageIsOutcomesOccurrencesAndCompleteCarrierKeys => {
            retained_storage_is_outcomes_occurrences_and_complete_carrier_keys()
        }
        Given::LiteralDemandDoesNotInitializeOtherParserCaches => {
            literal_demand_does_not_initialize_other_parser_caches()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
