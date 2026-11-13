//! What every executables adapter keeps: the target answers, a sealed copy carries the digest of its bytes, and an
//! absent reference tool, an absent bench and an absent executable are refused.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::executables::interface::ExecutablesPort;
use crate::outbound::executables::types::{BenchPackage, RunArguments};
use ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName;
use sha2::{Digest as _, Sha256};

/// What a contract row asks of the adapter.
#[derive(Clone, Copy, Debug)]
pub enum Request {
    /// Answer the build target directory.
    Target,
    /// Seal the running executable and compare the digest with the sealed bytes.
    SealCurrent,
    /// Look up a tool no search path holds.
    AbsentReference,
    /// Build a bench that does not exist.
    AbsentBench,
    /// Run an executable that does not exist.
    AbsentExecutable,
}

/// How a contract row ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The adapter answered.
    Answered,
    /// The sealed copy's digest is the digest of its bytes.
    DigestMatches,
    /// The adapter refused the request.
    Refused,
}

/// What a row asks.
type Input = Request;
/// What a row answers.
type Value = Outcome;
/// What a row refuses with.
type Failure = Outcome;

/// Named observations every implementation must satisfy.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the build target directory is answered"),
        word: Word::Succeeds,
        input: || Request::Target,
        expect: Expect::Value(Outcome::Answered),
    },
    Case {
        name: CaseName("a sealed copy carries the digest of its own bytes"),
        word: Word::Succeeds,
        input: || Request::SealCurrent,
        expect: Expect::Value(Outcome::DigestMatches),
    },
    Case {
        name: CaseName("a reference tool the search path does not hold is refused"),
        word: Word::Refuses,
        input: || Request::AbsentReference,
        expect: Expect::Error(Outcome::Refused),
    },
    Case {
        name: CaseName("a bench that does not exist is refused before anything is built"),
        word: Word::Refuses,
        input: || Request::AbsentBench,
        expect: Expect::Error(Outcome::Refused),
    },
    Case {
        name: CaseName("an executable that does not exist is refused"),
        word: Word::Refuses,
        input: || Request::AbsentExecutable,
        expect: Expect::Error(Outcome::Refused),
    },
];

/// Run the shared observations against the supplied implementation.
///
/// # Panics
///
/// Panics when fixture setup fails or the implementation violates a contract observation.
#[expect(
    clippy::expect_used,
    reason = "fixture setup failures must fail the contract test"
)]
pub fn check(adapter: &impl ExecutablesPort) {
    let root = tempfile::tempdir().expect("seal fixture");
    let refused = |_error| Outcome::Refused;
    let subject = |request: Input| -> Result<Value, Failure> {
        match request {
            Request::Target => adapter
                .target()
                .map(|_target| Outcome::Answered)
                .map_err(refused),
            Request::SealCurrent => {
                let sealed = adapter
                    .seal_current(&root.path().join("collector"))
                    .map_err(refused)?;
                let bytes = std::fs::read(&sealed.path).expect("sealed bytes");
                Ok(
                    if format!("{:x}", Sha256::digest(&bytes)) == sealed.sha256.0 {
                        Outcome::DigestMatches
                    } else {
                        Outcome::Refused
                    },
                )
            }
            Request::AbsentReference => adapter
                .reference(&ToolName::from("ryo-absent-reference-tool"))
                .map(|_path| Outcome::Answered)
                .map_err(refused),
            Request::AbsentBench => adapter
                .seal_worker(
                    &BenchPackage(root.path().join("absent-bench")),
                    &root.path().join("out/worker"),
                )
                .map(|_sealed| Outcome::Answered)
                .map_err(refused),
            Request::AbsentExecutable => adapter
                .run_sealed(
                    &root.path().join("absent-executable"),
                    &RunArguments(Vec::new()),
                    &mut Vec::new(),
                )
                .map(|_verdict| Outcome::Answered)
                .map_err(refused),
        }
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
