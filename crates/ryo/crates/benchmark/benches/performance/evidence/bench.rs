//! benchmark's evidence workload: one rule's verification times added and the one document written, through each
//! construction of the BMF adapter (a stream, a new file, a file appended to).
#![expect(
    clippy::expect_used,
    reason = "missing benchmark prerequisites must fail visibly"
)]
use ryo_benchmark::outbound::evidence::{
    factory::{bmf, bmf_appending, bmf_document},
    interface::EvidencePort,
    types::{RuleTiming, RuleTimings},
};

/// One rule's verification times, as a verification adds them.
fn timings() -> RuleTimings {
    RuleTimings(std::collections::BTreeMap::from([(
        ryo::linter::RuleId::try_from("fixture").expect("fixture rule"),
        RuleTiming::default(),
    )]))
}

/// Add the times and write the document.
fn publish(evidence: &dyn EvidencePort) {
    evidence
        .verification(&"python".into(), &timings())
        .expect("times are added");
    evidence.finish().expect("the document is written");
}

#[divan::bench]
/// Measure a document written to a stream.
fn stream(bencher: divan::Bencher<'_, '_>) {
    bencher.bench_local(|| publish(&bmf(Box::new(std::io::sink()), Box::new(std::io::sink()))));
}

#[divan::bench]
/// Measure a document written to a new file.
fn created(bencher: divan::Bencher<'_, '_>) {
    let directory = tempfile::tempdir().expect("scratch directory");
    let file = directory.path().join("document.bmf");
    bencher.bench_local(|| publish(&bmf_document(file.clone(), Box::new(std::io::sink()))));
}

#[divan::bench]
/// Measure a document appended to a file.
fn appended(bencher: divan::Bencher<'_, '_>) {
    let directory = tempfile::tempdir().expect("scratch directory");
    let file = directory.path().join("times.bmf");
    bencher.bench_local(|| publish(&bmf_appending(file.clone())));
}
