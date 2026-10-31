//! Every linter prepares the requested workspace and refuses operational failure.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::measurements::types::ProcessOutput;
use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types::RunRequest};
use crate::outbound::linter::interface::LinterPort;
use std::{path::PathBuf, time::Duration};
#[derive(Clone, Copy, Debug)]
/// Operations covered by the shared linter contract.
pub enum Given {
    /// Prepare the requested working directory.
    Prepare,
    /// Decode an operational process failure.
    OperationalFailure,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Canonical contract failure classifications.
pub enum Refusal {
    /// The process reported an operational failure.
    Operational,
    /// The implementation returned an unexpected failure category.
    Unexpected,
}
/// Named observations every implementation must satisfy.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("prepare uses requested working directory"),
        word: Word::Succeeds,
        input: || Input::Prepare,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("operational failure is not findings"),
        word: Word::Refuses,
        input: || Input::OperationalFailure,
        expect: Expect::Error(Failure::Operational),
    },
];
/// Run the shared observations against the supplied implementation.
///
/// # Panics
///
/// Panics when fixture setup fails or the implementation violates a contract observation.
pub fn check(adapter: &impl LinterPort) {
    let request = RunRequest {
        root: PathBuf::new(),
        state: PathBuf::new(),
        files: vec![ryo::linter::RelativePath("input.txt".into())],
    };
    let subject = |input| match input {
        Input::Prepare => adapter
            .prepare(&request)
            .map(|call| call.directory == request.root)
            .map_err(|_not_text| Failure::Unexpected),
        Input::OperationalFailure => match adapter.decode(
            &request,
            ProcessOutput {
                status: crate::domain::measurements::types::ProcessExitCode(2),
                stdout: crate::domain::measurements::types::CapturedBytes(Vec::new()),
                stderr: crate::domain::measurements::types::CapturedBytes(Vec::new()),
                wall: Duration::ZERO,
                resources: crate::domain::measurements::types::ResourceObservation::WallOnly,
            },
        ) {
            Err(Error::Invalid(_)) => Err(Failure::Operational),
            Err(_) => Err(Failure::Unexpected),
            Ok(_) => Ok(false),
        },
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}

/// Classified output normalization refusal.
type Failure = Refusal;

/// Independently specified process output and request.
type Input = Given;

/// Whether normalization preserves the expected report.
type Value = bool;
