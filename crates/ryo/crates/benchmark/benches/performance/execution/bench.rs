//! benchmark's execution workload: identical child invocations through both implementations.
//! The child is `/bin/sh`, so the module is compiled on unix alone; `timed` observes through `direct`,
//! whose platform contract refuses every launch off Linux, so their two workloads are registered on Linux alone.
#![cfg(unix)]
#![expect(
    clippy::expect_used,
    reason = "missing benchmark prerequisites must fail visibly"
)]
#[cfg(target_os = "linux")]
use ryo_benchmark::outbound::execution::factory::{direct, timed};
use ryo_benchmark::outbound::execution::{
    factory::{bounded, in_process, process},
    interface::ExecutionPort as _,
    types::{InProcessWorker, RuleCompileTimes, StoredRequest, WorkerResponse},
};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
/// Construct the same explicit child workload for each execution adapter.
fn invocation() -> Invocation {
    Invocation {
        executable: "/bin/sh".into(),
        arguments: vec!["-c".into(), "printf result".into()],
        environment: std::collections::BTreeMap::new(),
        directory: std::env::temp_dir(),
    }
}
#[divan::bench]
/// Measure child startup and collection without resource instrumentation.
fn process_child(bencher: divan::Bencher<'_, '_>) {
    let adapter = process();
    let input = invocation();
    bencher.bench_local(|| {
        let output = adapter.run(input.clone()).expect("child starts");
        assert_eq!(output.status.0, 0_i32, "workload must exit successfully");
        assert_eq!(
            output.stdout.0, b"result",
            "workload output must survive capture"
        );
        divan::black_box(output)
    });
}
#[cfg(target_os = "linux")]
#[divan::bench]
/// Measure the isolated collector boundary using its actual built executable.
fn timed_child(bencher: divan::Bencher<'_, '_>) {
    use sha2::{Digest as _, Sha256};
    // Cargo builds the package's binaries for its benches and names each one's path: never a guess from where this
    // bench's own executable lies, which differs between Cargo's build layouts.
    let executable = std::path::PathBuf::from(env!("CARGO_BIN_EXE_ryo-benchmark-collector"));
    let digest =
        ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest(format!(
            "{:x}",
            Sha256::digest(std::fs::read(&executable).expect("collector bytes"))
        ));
    let adapter = timed(executable, digest);
    let input = invocation();
    bencher.bench_local(|| {
        let output = adapter
            .run(input.clone())
            .expect("instrumented child starts");
        assert_eq!(output.status.0, 0_i32, "workload must exit successfully");
        assert_eq!(
            output.stdout.0, b"result",
            "workload output must survive capture"
        );
        assert!(
            matches!(
                output.resources,
                ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::Collected(_)
            ),
            "instrumented workload must supply actual child resources"
        );
        divan::black_box(output)
    });
}

#[cfg(target_os = "linux")]
#[divan::bench]
/// Measure direct collection separately from the isolated collector boundary.
fn direct_child(bencher: divan::Bencher<'_, '_>) {
    let adapter = direct();
    let input = invocation();
    bencher.bench_local(|| {
        let output = adapter.run(input.clone()).expect("direct child starts");
        assert_eq!(output.status.0, 0_i32, "workload must exit successfully");
        assert_eq!(
            output.stdout.0, b"result",
            "workload output must survive capture"
        );
        divan::black_box(output)
    });
}

#[divan::bench]
/// Measure the deadline-bounded wall execution that file-backed capture adds to process launch.
fn bounded_child(bencher: divan::Bencher<'_, '_>) {
    let adapter = bounded(std::time::Duration::from_secs(60));
    let input = invocation();
    bencher.bench_local(|| {
        let output = adapter.run(input.clone()).expect("bounded child starts");
        assert_eq!(output.status.0, 0_i32, "workload must exit successfully");
        assert_eq!(
            output.stdout.0, b"result",
            "workload output must survive capture"
        );
        divan::black_box(output)
    });
}
/// A worker that answers every stored request with the same response, as the language's worker answers one.
#[derive(Debug)]
struct Answering;
impl InProcessWorker for Answering {
    fn run(
        &self,
        _request: &StoredRequest,
    ) -> Result<WorkerResponse, ryo_benchmark_contracts::domain::reports::errors::ReportsError>
    {
        Ok(WorkerResponse(b"result".to_vec()))
    }
    fn compile(
        &self,
        _request: &StoredRequest,
    ) -> Result<
        Option<RuleCompileTimes>,
        ryo_benchmark_contracts::domain::reports::errors::ReportsError,
    > {
        Ok(Some(RuleCompileTimes::default()))
    }
}
#[divan::bench]
/// Measure one worker request run in this process, its response captured as a child's stdout.
fn in_process_worker(bencher: divan::Bencher<'_, '_>) {
    let adapter = in_process(&Answering);
    let input = Invocation {
        executable: "/bin/sh".into(),
        arguments: vec![
            ryo_benchmark_contracts::domain::requests::constants::RUN_RYO.into(),
            ryo_benchmark_contracts::domain::requests::constants::REQUEST_FILE.into(),
            "request.json".into(),
        ],
        environment: std::collections::BTreeMap::new(),
        directory: std::env::temp_dir(),
    };
    bencher.bench_local(|| {
        let output = adapter.run(input.clone()).expect("worker runs");
        assert_eq!(output.status.0, 0_i32, "workload must complete");
        assert_eq!(
            output.stdout.0, b"result",
            "workload response must survive capture"
        );
        divan::black_box(output)
    });
}
