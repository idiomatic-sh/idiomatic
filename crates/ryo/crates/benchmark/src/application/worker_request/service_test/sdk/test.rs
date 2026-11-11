//! A scenario of the `sdk` group: two cases of `worker_request` run through the real SDK: inputs declared missing are prepared once, and a prepared rule context
//! keeps its sibling rule's findings.

use super::fixtures::{
    actual_sdk_executes_missing_input_facts_but_refuses_unmodeled_provider_unread,
    prepared_context_counts_disabled_pairs_and_keeps_the_sibling_rule,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `actual_sdk_executes_missing_input_facts_but_refuses_unmodeled_provider_unread` scenario.
    ActualSdkExecutesMissingInputFactsButRefusesUnmodeledProviderUnread,
    /// The `prepared_context_counts_disabled_pairs_and_keeps_the_sibling_rule` scenario.
    PreparedContextCountsDisabledPairsAndKeepsTheSiblingRule,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `; `.
type Value = &'static str;
/// A scenario is never refused.
type Failure = ();

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "actual_sdk_executes_missing_input_facts_but_refuses_unmodeled_provider_unread",
        ),
        word: Word::Succeeds,
        input: || Given::ActualSdkExecutesMissingInputFactsButRefusesUnmodeledProviderUnread,
        expect: Expect::Value(
            "{RelativePath(\"missing.py\")}; 1; RelativePath(\"missing.py\"); 0; 0; true; true",
        ),
    },
    Case {
        name: CaseName("prepared_context_counts_disabled_pairs_and_keeps_the_sibling_rule"),
        word: Word::Succeeds,
        input: || Given::PreparedContextCountsDisabledPairsAndKeepsTheSiblingRule,
        expect: Expect::Value(
            "2; 2; true; 2; 1; true; true; 2; 1; true; true; 2; 0; true; 2; 2; true",
        ),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::ActualSdkExecutesMissingInputFactsButRefusesUnmodeledProviderUnread => {
            actual_sdk_executes_missing_input_facts_but_refuses_unmodeled_provider_unread()
        }
        Given::PreparedContextCountsDisabledPairsAndKeepsTheSiblingRule => {
            prepared_context_counts_disabled_pairs_and_keeps_the_sibling_rule()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
