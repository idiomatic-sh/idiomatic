//! Both contestants prepare the one neutral request a certificate binds their identities to.
use super::service::probe;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::linter::fixtures::fake::LinterFake;
use std::path::PathBuf;

/// What the probe made of the two contestants.
#[derive(Debug, PartialEq, Eq)]
enum Probed {
    /// Both contestants prepared the probe under the certified directory.
    BothUnderTheDirectory,
}

type Input = ();
type Value = Probed;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("both contestants prepare the neutral probe under the certified directory"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Probed::BothUnderTheDirectory),
}];

fn subject(_: Input) -> Result<Value, Failure> {
    let directory = PathBuf::from("profile");
    let probed = probe(
        &LinterFake::default(),
        &LinterFake::default(),
        &directory,
        std::path::Path::new("scratch"),
    )
    .map_err(|_error| ())?;
    assert_eq!(probed.probe.root, directory);
    assert!(
        probed
            .profiles
            .iter()
            .all(|profile| profile.directory == directory)
    );
    Ok(Probed::BothUnderTheDirectory)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
