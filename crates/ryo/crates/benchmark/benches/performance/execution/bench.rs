//! benchmark's execution workload: identical child invocations through both implementations.
#![expect(
    clippy::expect_used,
    reason = "missing benchmark prerequisites must fail visibly"
)]
use ryo_benchmark::domain::measurements::types::Invocation;
use ryo_benchmark::outbound::execution::{
    factory::{process, timed},
    interface::ExecutionPort,
};
fn invocation() -> Invocation {
    Invocation {
        executable: "/bin/sh".into(),
        arguments: vec!["-c".into(), "printf result".into()],
        environment: std::collections::BTreeMap::new(),
        directory: std::env::temp_dir(),
    }
}
#[divan::bench]
fn process_child(bencher: divan::Bencher<'_, '_>) {
    let adapter = process();
    let input = invocation();
    bencher.bench_local(|| {
        let output = adapter.run(input.clone()).expect("child starts");
        assert_eq!(output.status, 0);
        assert_eq!(output.stdout, b"result");
        divan::black_box(output)
    });
}
#[divan::bench]
fn timed_child(bencher: divan::Bencher<'_, '_>) {
    let adapter = timed();
    let input = invocation();
    bencher.bench_local(|| {
        let output = adapter
            .run(input.clone())
            .expect("instrumented child starts");
        assert_eq!(output.status, 0);
        assert_eq!(output.stdout, b"result");
        assert!(output.resources.is_some());
        divan::black_box(output)
    });
}
