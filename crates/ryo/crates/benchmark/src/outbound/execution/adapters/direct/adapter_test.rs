//! Resource instrumentation preserves the execution port's process observations.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::execution::fixtures::contract::check;
type Input = ();
type Value = ();
type Failure = crate::outbound::execution::fixtures::contract::LaunchFailure;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("timed execution honors process contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared case runner requires a Result-returning subject for every outcome word"
)]
fn subject(_: Input) -> Result<Value, Failure> {
    check(&crate::outbound::execution::factory::direct());
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// A real signaled child still has native observations and streams.
#[cfg(target_os = "linux")]
#[test]
fn signaled_child_retains_native_resources() -> Result<(), Box<dyn std::error::Error>> {
    use crate::domain::measurements::types::{Invocation, ResourceObservation};
    use crate::outbound::execution::interface::ExecutionPort as _;
    let output = crate::outbound::execution::factory::direct().run(Invocation {
        executable: "/bin/sh".into(),
        arguments: vec!["-c".into(), "printf partial; kill -TERM $$".into()],
        environment: Default::default(),
        directory: ".".into(),
    })?;
    assert_eq!(output.signal, Some(15));
    assert_eq!(output.status.0, -15);
    assert_eq!(output.stdout.0, b"partial");
    assert!(!output.wall.is_zero());
    let ResourceObservation::Collected(resources) = output.resources else {
        return Err("native resources absent".into());
    };
    assert!(resources.peak_rss_bytes.0 > 0);
    assert!(resources.cpu <= output.wall);
    Ok(())
}
