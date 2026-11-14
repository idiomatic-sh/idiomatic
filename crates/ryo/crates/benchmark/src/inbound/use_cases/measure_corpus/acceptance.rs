//! The public corpus measurement over the fakes of its boundaries.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::measure_corpus::{
    fixtures::{discovering, use_case, with_corpus, with_suffixes},
    interface::MeasureCorpusUseCase as _,
    types::SourceSuffix,
};
use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;

#[then("measure_corpus says a diagnostic corpus publishes no metric")]
fn diagnostic(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let (succeeded, observations) = measure(BenchmarkCoverage::Interim);
    assert!(succeeded);
    assert!(observations.contains("not benchmark metrics"));
}

#[then("measure_corpus refuses to publish an accounted corpus without samples")]
fn refuses(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(!measure(BenchmarkCoverage::Accounted).0);
}

#[then("measure_corpus counts a.ts and b.mjs in the corpus shape and not c.vue")]
fn suffixed(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    let suffixes = [".ts", ".mts", ".cts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"].map(SourceSuffix);
    let mut observations = Vec::new();
    let _measured = with_suffixes(&suffixes, |corpus| {
        discovering(&["a.ts", "b.mjs", "c.vue"]).measure_corpus(corpus, &mut observations)
    });
    let observations = String::from_utf8_lossy(&observations);
    assert!(
        observations.contains(
            "project-0: matched Fixture source scope 2 files; excluded default-discovered formats 1 files (source-kind capability gap): c.vue\n"
        ),
        "{observations}"
    );
}

/// Whether an empty corpus run succeeded, and what it wrote.
fn measure(scope: BenchmarkCoverage) -> (bool, String) {
    let mut observations = Vec::new();
    let result = with_corpus(scope, 0, |corpus| {
        use_case().measure_corpus(corpus, &mut observations)
    });
    (
        result.is_ok(),
        String::from_utf8_lossy(&observations).into_owned(),
    )
}
