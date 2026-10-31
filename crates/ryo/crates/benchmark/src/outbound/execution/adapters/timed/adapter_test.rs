//! Resource instrumentation preserves the execution port's process observations.
use super::{adapter::ExecutionTimedAdapter, settings::ExecutionTimedSettings};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::execution::fixtures::contract::check;
use sha2::{Digest, Sha256};
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("timed execution honors process contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
fn subject(_: Input) -> Result<Value, Failure> {
    let executable = std::path::PathBuf::from("/usr/bin/time");
    let bytes = std::fs::read(&executable)
        .expect("GNU time is required by the instrumentation integration proof");
    let sha256 = format!("{:x}", Sha256::digest(bytes));
    check(&ExecutionTimedAdapter::new(ExecutionTimedSettings {
        executable,
        sha256: crate::domain::measurements::types::ExecutableDigest(sha256),
        path: Some(std::ffi::OsString::from("/usr/bin:/bin")),
    }));
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
