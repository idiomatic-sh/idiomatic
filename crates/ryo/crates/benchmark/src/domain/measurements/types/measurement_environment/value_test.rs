//! The environment line names the collection semantics and their limits, with this machine's OS and architecture.
use super::value::MeasurementEnvironment;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The line, its machine named `<os>` and `<arch>`.
type Value = &'static str;

const CASES: &[Case<(), Value, ()>] = &[Case {
    name: CaseName("the environment statement names the collection semantics and their limits"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(
        "environment: os=<os> arch=<arch> release=true workers=1; isolated collector Linux wait4 CPU=user+system, RSS=child peak bytes; child wall includes spawn/wait and pipe collection; collector launch, normalization and certificate/source hashing excluded; OS page cache uncontrolled; retained cache hits unverified",
    ),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| {
        Ok(&*Box::leak(
            MeasurementEnvironment
                .to_string()
                .replace(std::env::consts::OS, "<os>")
                .replace(std::env::consts::ARCH, "<arch>")
                .into_boxed_str(),
        ))
    });
}
