//! A refused source observation stops staging before any snapshot writes.
use super::service;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::fixtures::fake::SuiteFake;
const CASES: &[Case<(), (), ()>] = &[Case {
    name: CaseName("missing profile stops staging"),
    word: Word::Refuses,
    input: || (),
    expect: Expect::Error(()),
}];
fn subject(_: ()) -> Result<(), ()> {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("snapshot");
    let result = service::stage(
        &SuiteFake::default(),
        root.path(),
        &destination,
        &[],
        ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Certification,
    );
    assert_eq!(
        std::fs::symlink_metadata(&destination).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    result.map_err(|error| { assert!(matches!(error, ryo_benchmark_contracts::domain::reports::errors::ReportsError::Io(ref error) if error.kind() == std::io::ErrorKind::NotFound)); () })
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
