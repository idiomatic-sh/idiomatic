//! benchmark's engine workload: one rule directory loaded through every construction of the local SDK adapter.
#![expect(
    clippy::expect_used,
    reason = "missing benchmark prerequisites must fail visibly"
)]
use ryo_benchmark::outbound::engine::{
    factory::{
        local, local_with_diagnostic_events, local_with_frontend_diagnostic, local_with_routes,
    },
    interface::EnginePort,
    types::FrontendDiagnosticProjection,
};
use ryo_benchmark::outbound::worker::fixtures::data::request;
use ryo_benchmark_contracts::domain::requests::types::{WorkerRequest, WorkerState};
use std::path::Path;

/// The one authored query every variant loads.
const QUERY: &str = "let missing = select path as file, span from \"filesystem.read-failure\"; emit missing subject file key(file);";

/// Write the rule directory and bind the fixture request to it.
fn workload(root: &Path) -> WorkerRequest {
    let rules = root.join("rules");
    std::fs::create_dir_all(rules.join("R1")).expect("rule directory");
    std::fs::write(rules.join("R1/query.ryoql"), QUERY).expect("rule query");
    let mut request = request();
    request.rules = rules;
    request.root = root.to_path_buf();
    request.state = WorkerState::Fresh;
    request
}
/// The explicit frontend diagnostic query the projecting variants load beside the rule.
fn projection() -> FrontendDiagnosticProjection {
    FrontendDiagnosticProjection {
        rule: ryo::linter::RuleId::try_from("frontend").expect("projection rule"),
        query: ryo::query::RyoQlQuery::try_from(QUERY).expect("projection query"),
    }
}
/// Load the workload through one adapter and keep the session alive for the measurement.
fn load(adapter: &impl EnginePort, request: &WorkerRequest) {
    divan::black_box(adapter.load(request).expect("rules load"));
}

#[divan::bench]
/// Measure loading with no frontend diagnostic projection.
fn plain(bencher: divan::Bencher<'_, '_>) {
    let root = tempfile::tempdir().expect("workload directory");
    let request = workload(root.path());
    let adapter = local(ryo::linter::Languages::default(), None);
    bencher.bench_local(|| load(&adapter, &request));
}
#[divan::bench]
/// Measure loading with the frontend diagnostic projection.
fn projected(bencher: divan::Bencher<'_, '_>) {
    let root = tempfile::tempdir().expect("workload directory");
    let request = workload(root.path());
    let adapter =
        local_with_frontend_diagnostic(ryo::linter::Languages::default(), None, projection());
    bencher.bench_local(|| load(&adapter, &request));
}
#[divan::bench]
/// Measure loading with every execution strategy wired explicitly.
fn routed(bencher: divan::Bencher<'_, '_>) {
    let root = tempfile::tempdir().expect("workload directory");
    let request = workload(root.path());
    let adapter = local_with_routes(
        ryo::linter::Languages::default(),
        projection(),
        ryo::linter::RoutePolicy::default(),
    );
    bencher.bench_local(|| load(&adapter, &request));
}
#[divan::bench(sample_count = 1, sample_size = 1)]
/// Measure one load with the diagnostic event sink; the process-wide subscriber installs once.
fn diagnostic_events(bencher: divan::Bencher<'_, '_>) {
    let root = tempfile::tempdir().expect("workload directory");
    let request = workload(root.path());
    let adapter = local_with_diagnostic_events(
        ryo::linter::Languages::default(),
        root.path().join("events.jsonl"),
        projection(),
    );
    bencher.bench_local(|| load(&adapter, &request));
}
