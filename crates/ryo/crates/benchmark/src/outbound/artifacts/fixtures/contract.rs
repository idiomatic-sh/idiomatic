//! What every artifacts adapter keeps: an absent reference tool is refused, and a sealed copy carries the digest of its bytes.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::artifacts::interface::ArtifactsPort;
use ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName;
use sha2::{Digest as _, Sha256};

/// What a contract row asks of the adapter.
#[derive(Clone, Copy, Debug)]
pub enum Request {
    /// Seal the running executable and compare the digest with the sealed bytes.
    SealCurrent,
    /// Look up a tool no search path holds.
    AbsentReference,
}

/// How a contract row ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The sealed copy's digest is the digest of its bytes.
    DigestMatches,
    /// The adapter refused the request.
    Refused,
}

/// Named observations every implementation must satisfy.
pub const CASES: &[Case<Request, Outcome, Outcome>] = &[
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
pub fn check(adapter: &impl ArtifactsPort) {
    let root = tempfile::tempdir().expect("seal fixture");
    let subject = |request| match request {
        Request::SealCurrent => {
            let sealed = adapter
                .seal_current(&root.path().join("collector"))
                .map_err(|_error| Outcome::Refused)?;
            let bytes = std::fs::read(&sealed.path).expect("sealed bytes");
            Ok(if format!("{:x}", Sha256::digest(&bytes)) == sealed.sha256.0 {
                Outcome::DigestMatches
            } else {
                Outcome::Refused
            })
        }
        Request::AbsentReference => adapter
            .reference(&ToolName::from("ryo-absent-reference-tool"))
            .map(|_path| Outcome::DigestMatches)
            .map_err(|_error| Outcome::Refused),
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
