//! Demand, representation, reset and unavailable-knowledge contracts.

use super::fixtures::{
    an_existing_hidden_embedded_success_is_reused_without_new_parsing,
    equal_decoded_text_keeps_representation_sensitive_parser_modes,
    literal_parsing_is_absent_without_demand_and_caches_both_outcomes,
    reset_discards_old_identities_results_and_key_bytes_but_keeps_capacity,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `literal_parsing_is_absent_without_demand_and_caches_both_outcomes` scenario.
    LiteralParsingIsAbsentWithoutDemandAndCachesBothOutcomes,
    /// The `reset_discards_old_identities_results_and_key_bytes_but_keeps_capacity` scenario.
    ResetDiscardsOldIdentitiesResultsAndKeyBytesButKeepsCapacity,
    /// The `equal_decoded_text_keeps_representation_sensitive_parser_modes` scenario.
    EqualDecodedTextKeepsRepresentationSensitiveParserModes,
    /// The `an_existing_hidden_embedded_success_is_reused_without_new_parsing` scenario.
    AnExistingHiddenEmbeddedSuccessIsReusedWithoutNewParsing,
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
        name: CaseName("literal_parsing_is_absent_without_demand_and_caches_both_outcomes"),
        word: Word::Succeeds,
        input: || Given::LiteralParsingIsAbsentWithoutDemandAndCachesBothOutcomes,
        expect: Expect::Value(
            "Fresh|0|Fresh|2|Complete|2|4|[Parsed, Parsed, Malformed, Malformed]|4|2",
        ),
    },
    Case {
        name: CaseName("reset_discards_old_identities_results_and_key_bytes_but_keeps_capacity"),
        word: Word::Succeeds,
        input: || Given::ResetDiscardsOldIdentitiesResultsAndKeyBytesButKeepsCapacity,
        expect: Expect::Value(
            "Fresh|true|true|true|true|true|Fresh|true|true|true|true|true|Fresh|true|true|true|true|true|Fresh|true|true|true|true|true|Fresh|true|true|true|true|true|Fresh|true|true|true|true|true",
        ),
    },
    Case {
        name: CaseName("equal_decoded_text_keeps_representation_sensitive_parser_modes"),
        word: Word::Succeeds,
        input: || Given::EqualDecodedTextKeepsRepresentationSensitiveParserModes,
        expect: Expect::Value("3|[Parsed, Malformed, Malformed]|3"),
    },
    Case {
        name: CaseName("an_existing_hidden_embedded_success_is_reused_without_new_parsing"),
        word: Word::Succeeds,
        input: || Given::AnExistingHiddenEmbeddedSuccessIsReusedWithoutNewParsing,
        expect: Expect::Value("true|1|true|0|true|1"),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::LiteralParsingIsAbsentWithoutDemandAndCachesBothOutcomes => {
            literal_parsing_is_absent_without_demand_and_caches_both_outcomes()
        }
        Given::ResetDiscardsOldIdentitiesResultsAndKeyBytesButKeepsCapacity => {
            reset_discards_old_identities_results_and_key_bytes_but_keeps_capacity()
        }
        Given::EqualDecodedTextKeepsRepresentationSensitiveParserModes => {
            equal_decoded_text_keeps_representation_sensitive_parser_modes()
        }
        Given::AnExistingHiddenEmbeddedSuccessIsReusedWithoutNewParsing => {
            an_existing_hidden_embedded_success_is_reused_without_new_parsing()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
