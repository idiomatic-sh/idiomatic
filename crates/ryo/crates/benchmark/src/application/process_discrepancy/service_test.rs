//! Behavioral proof of `process_discrepancy`: the actual reference process is the declared completed failure, byte for byte.
use super::errors::ProcessDiscrepancyError;
use super::fixtures::{Actual, Artifacts, complete, edited, passing};
use super::service::matches;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::types::{OptionalDigest, ReferenceKind};
use crate::domain::registered_inputs::types::RefusalText;
use crate::outbound::suite::fixtures::fake::SuiteFake;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::{
    FailedInvocation, InvocationDocument,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    ExitStatus, PathText, ReasonText, Sha256Text, YesNo,
};
use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Signal};
use ryo_benchmark_contracts::domain::measurements::types::ProcessExitCode;
use std::path::Path;

/// What an accepted capture answers.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Outcome {
    Matches,
}

type Input = Actual;
type Value = Outcome;
type Failure = ProcessDiscrepancyError;

fn failed() -> FailedInvocation {
    FailedInvocation {
        actual_request_root: PathText("/root".into()),
        files: vec![PathText("case.py".into())],
        arguments: Vec::new(),
        environment: std::collections::BTreeMap::new(),
        directory: PathText("/root".into()),
        exit: ExitStatus(2),
        signal: Signal::Absent,
        observer_no_deadline: YesNo::Yes,
        metadata_error: ReasonText("identity unavailable".into()),
        source_sha256: Pin::Absent,
        file_metadata_sha256: Pin::Absent,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the declared completed failure with exact bytes matches"),
        word: Word::Succeeds,
        input: passing,
        expect: Expect::Value(Outcome::Matches),
    },
    Case {
        name: CaseName("a case without a process expectation has nothing to match"),
        word: Word::Refuses,
        input: || {
            edited(|actual| {
                actual.case.reference_discrepancy = ryo_benchmark_contracts::domain::workloads::types::PreparedReferenceBehavior::Agrees
            })
        },
        expect: Expect::Error(ProcessDiscrepancyError::NoProcessExpectation),
    },
    Case {
        name: CaseName("another executable than the validated one is refused"),
        word: Word::Refuses,
        input: || edited(|actual| complete(actual).executable_sha256 = Sha256Text("b".repeat(64))),
        expect: Expect::Error(ProcessDiscrepancyError::IdentityNotValidatedGroup),
    },
    Case {
        name: CaseName("other settings than the validated ones are refused"),
        word: Word::Refuses,
        input: || edited(|actual| complete(actual).settings_sha256 = Sha256Text("b".repeat(64))),
        expect: Expect::Error(ProcessDiscrepancyError::IdentityNotValidatedGroup),
    },
    Case {
        name: CaseName("another profile than the validated one is refused"),
        word: Word::Refuses,
        input: || edited(|actual| complete(actual).profile_sha256 = Sha256Text("b".repeat(64))),
        expect: Expect::Error(ProcessDiscrepancyError::IdentityNotValidatedGroup),
    },
    Case {
        name: CaseName("file metadata the proof does not bind is refused"),
        word: Word::Refuses,
        input: || {
            edited(|actual| {
                complete(actual).file_metadata_sha256 = Pin::Sha256(Sha256Text("b".repeat(64)))
            })
        },
        expect: Expect::Error(ProcessDiscrepancyError::IdentityNotValidatedGroup),
    },
    Case {
        name: CaseName("file metadata only the proof binds is refused"),
        word: Word::Refuses,
        input: || {
            edited(|actual| {
                actual.proof.file_metadata = OptionalDigest::Pinned(ArtifactDigest("b".repeat(64)))
            })
        },
        expect: Expect::Error(ProcessDiscrepancyError::IdentityNotValidatedGroup),
    },
    Case {
        name: CaseName("a proof of diagnostics cannot validate a process failure"),
        word: Word::Refuses,
        input: || edited(|actual| actual.proof.kind = ReferenceKind::Diagnostics),
        expect: Expect::Error(ProcessDiscrepancyError::IdentityNotValidatedGroup),
    },
    Case {
        name: CaseName("a capture without a recorded identity is refused"),
        word: Word::Refuses,
        input: || edited(|actual| actual.invocation = InvocationDocument::Failed(failed())),
        expect: Expect::Error(ProcessDiscrepancyError::IdentityNotValidatedGroup),
    },
    Case {
        name: CaseName("a signalled process is not a completed failure"),
        word: Word::Refuses,
        input: || {
            edited(|actual| {
                actual.output.signal = ryo_benchmark_contracts::domain::measurements::types::TerminationSignal::Signalled(ryo_benchmark_contracts::domain::measurements::types::SignalNumber(15))
            })
        },
        expect: Expect::Error(ProcessDiscrepancyError::TerminationDiffers),
    },
    Case {
        name: CaseName("another exit status than declared is refused"),
        word: Word::Refuses,
        input: || edited(|actual| actual.output.status = ProcessExitCode(1)),
        expect: Expect::Error(ProcessDiscrepancyError::TerminationDiffers),
    },
    Case {
        name: CaseName("an observer that applied a deadline cannot attest the failure"),
        word: Word::Refuses,
        input: || edited(|actual| complete(actual).observer_no_deadline = YesNo::No),
        expect: Expect::Error(ProcessDiscrepancyError::TerminationDiffers),
    },
    Case {
        name: CaseName("a recorded signal is not a completed failure"),
        word: Word::Refuses,
        input: || edited(|actual| complete(actual).signal = Signal::Number(ExitStatus(15))),
        expect: Expect::Error(ProcessDiscrepancyError::TerminationDiffers),
    },
    Case {
        name: CaseName("an extra backtrace on stderr is not the declared bytes"),
        word: Word::Refuses,
        input: || {
            edited(|actual| {
                actual
                    .output
                    .stderr
                    .0
                    .extend_from_slice(b"\nstack backtrace\n")
            })
        },
        expect: Expect::Error(ProcessDiscrepancyError::BytesDiffer),
    },
    Case {
        name: CaseName("other stdout than declared is refused"),
        word: Word::Refuses,
        input: || edited(|actual| actual.output.stdout.0 = b"forged".to_vec()),
        expect: Expect::Error(ProcessDiscrepancyError::BytesDiffer),
    },
    Case {
        name: CaseName("stale independent artifacts are refused with their own words"),
        word: Word::Refuses,
        input: || edited(|actual| actual.artifacts = Artifacts::Invalid),
        expect: Expect::Error(ProcessDiscrepancyError::Refused(RefusalText(String::new()))),
    },
    Case {
        name: CaseName("rules without a profile directory name no profile"),
        word: Word::Refuses,
        input: || edited(|actual| actual.rules = ""),
        expect: Expect::Error(ProcessDiscrepancyError::MissingProfileBoundary),
    },
];

/// A refusal in a library's words is one kind of refusal, whatever the words.
fn words_dropped(error: Failure) -> Failure {
    match error {
        ProcessDiscrepancyError::Refused(_) => {
            ProcessDiscrepancyError::Refused(RefusalText(String::new()))
        }
        other @ (ProcessDiscrepancyError::NoProcessExpectation { .. }
        | ProcessDiscrepancyError::IdentityNotValidatedGroup { .. }
        | ProcessDiscrepancyError::TerminationDiffers { .. }
        | ProcessDiscrepancyError::BytesDiffer { .. }
        | ProcessDiscrepancyError::MissingProfileBoundary { .. }) => other,
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    let suite = SuiteFake {
        discrepancy_artifacts_valid: matches!(input.artifacts, Artifacts::Valid),
        ..SuiteFake::default()
    };
    matches(
        &input.case,
        &input.invocation,
        &input.output,
        &input.proof,
        &suite,
        Path::new(input.rules),
    )
    .map(|()| Outcome::Matches)
    .map_err(words_dropped)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
