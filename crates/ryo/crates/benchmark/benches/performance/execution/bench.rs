//! benchmark's execution workload: identical child invocations through both implementations.
#![expect(
    clippy::expect_used,
    reason = "missing benchmark prerequisites must fail visibly"
)]
use ryo_benchmark::domain::measurements::types::Invocation;
use ryo_benchmark::outbound::execution::{
    factory::{direct, process, timed},
    interface::ExecutionPort as _,
};
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
#[divan::bench]
/// Measure the isolated collector boundary using its actual built executable.
fn timed_child(bencher: divan::Bencher<'_, '_>) {
    use sha2::{Digest as _, Sha256};
    let bench = std::env::current_exe().expect("running benchmark executable");
    let executable = bench
        .parent()
        .expect("deps directory")
        .parent()
        .expect("profile directory")
        .join("ryo-benchmark-collector");
    let digest = ryo_benchmark::domain::suite::types::ArtifactDigest(format!(
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
                ryo_benchmark::domain::measurements::types::ResourceObservation::Collected(_)
            ),
            "instrumented workload must supply actual child resources"
        );
        divan::black_box(output)
    });
}

#[divan::bench]
/// Measure direct collection separately from the isolated collector boundary.
fn direct_child(bencher: divan::Bencher<'_, '_>) {
    let adapter = direct();
    let input = invocation();
    bencher.bench_local(|| adapter.run(input.clone()).expect("direct child starts"));
}
