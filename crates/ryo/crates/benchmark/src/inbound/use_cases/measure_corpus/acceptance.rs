//! The public corpus measurement over the fakes of its boundaries.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::measure_corpus::{
    fixtures::{use_case, with_corpus},
    interface::MeasureCorpusUseCase as _,
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
