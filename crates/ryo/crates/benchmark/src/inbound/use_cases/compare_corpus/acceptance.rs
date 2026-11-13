//! The steps of the `compare_corpus` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::compare_corpus::{
    fixtures::{Scenario, configured, corpus, syntax, use_case},
    interface::CompareCorpusUseCase as _,
    types::{AcceptedDifferencesFile, CorpusParity},
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::reports::types::ContestantName;

#[then("compare_corpus passes a corpus whose one difference is accepted")]
fn accepted(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(compare(true).0.is_ok());
}

#[then("compare_corpus refuses a request that selects other than the configured rules")]
fn unconfigured(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let (result, observations) = compare(false);
    assert!(result.is_err() && observations.is_empty());
}

/// Compare the fixture corpus, the request selecting the configured rule or another, and what was written.
fn compare(configured_rule: bool) -> (Result<(), ReportsError>, Vec<u8>) {
    let corpus = corpus(Scenario {
        configured: configured_rule,
        this_corpus: true,
        accepted_column: 3,
    });
    let mut observations = Vec::new();
    let result = use_case().compare_corpus(
        &CorpusParity {
            request: &corpus.request,
            configured: &configured(),
            accepted: AcceptedDifferencesFile::Declared(&corpus.accepted),
            reference: ContestantName("fixture"),
            syntax: &syntax(),
        },
        &mut observations,
    );
    (result, observations)
}
