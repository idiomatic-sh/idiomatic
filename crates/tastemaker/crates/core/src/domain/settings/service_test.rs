//! Proves the build that runs is told by its executable's bytes, not by its version alone.

use super::service::{build, unread};
use super::types::{Executable, ProcessId, ProcessStart, StartedAt};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::packages::types::VersionText;

/// Two executables, each its version and bytes; with no bytes, two processes whose executables were not read.
type Input = ((&'static str, &'static [u8]), (&'static str, &'static [u8]));
/// Whether the two are one build.
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("executables_of_one_size_whose_bytes_differ_are_other_builds"),
        word: Word::Succeeds,
        input: || (("0.1.0", b"build-a"), ("0.1.0", b"build-b")),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("the_same_bytes_are_the_same_build"),
        word: Word::Succeeds,
        input: || (("0.1.0", b"build-a"), ("0.1.0", b"build-a")),
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_same_bytes_of_another_version_are_another_build"),
        word: Word::Succeeds,
        input: || (("0.1.0", b"build-a"), ("0.2.0", b"build-a")),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("two_processes_whose_executables_were_not_read_are_other_builds"),
        word: Word::Succeeds,
        input: || (("0.1.0", b""), ("0.1.0", b"")),
        expect: Expect::Value(false),
    },
];

fn subject(((one_version, one), (other_version, other)): Input) -> Result<Value, std::io::Error> {
    if one.is_empty() {
        let version = VersionText(one_version.into());
        let start = |id| ProcessStart {
            id: ProcessId(id),
            started: StartedAt(5),
        };
        return Ok(unread(&version, start(1)) == unread(&version, start(2)));
    }
    Ok(build(
        &VersionText(one_version.into()),
        &mut Executable(&mut &*one),
    )? == build(
        &VersionText(other_version.into()),
        &mut Executable(&mut &*other),
    )?)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| subject(input).map_err(drop));
}
