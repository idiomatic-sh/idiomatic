//! Behavioral proof of `process_receipts`: a reference process receipt is rebuilt from registration and actual roots.
use std::collections::BTreeMap;

use ryo_benchmark_contracts::domain::documents::types::text::value::ExitStatus;
use ryo_benchmark_contracts::domain::documents::types::text::value::{FileMetadataDigest, PathText, ReasonText, Sha256Text, YesNo};
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{
    IntentMatch, ProcessArtifactsRecord, RecordedReason, ReferenceProcess, ReportedFindings, RequestRoot,
};
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::ProcessExpectation;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use sha2::Digest as _;

use super::{
    errors::ProcessReceiptsError,
    fixtures::{Documents, artifacts, discrepancy, edited, passing, process, stdout_path, validation},
    service::validate,
};
use crate::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        process_failure::errors::ProcessFailureError, process_output::errors::ProcessOutputError,
    },
};
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::{FailedInvocation, InvocationDocument};
use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Signal};
use ryo_benchmark_contracts::domain::workloads::types::CaseComparison;

/// What an accepted receipt answers.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Outcome {
    Accepted,
}

type Input = Documents;
type Value = Outcome;
type Failure = ProcessReceiptsError;

fn sha(letter: char) -> Sha256Text {
    Sha256Text(letter.to_string().repeat(64))
}

fn bytes_digest(bytes: &[u8]) -> Sha256Text {
    Sha256Text(format!("{:x}", sha2::Sha256::digest(bytes)))
}

/// A capture that could not record its process identity.
fn failed() -> FailedInvocation {
    FailedInvocation {
        actual_request_root: PathText("/root".into()),
        files: vec![PathText("case.py".into())],
        arguments: Vec::new(),
        environment: BTreeMap::new(),
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
        name: CaseName("a reference process that matches its registration is accepted"),
        word: Word::Succeeds,
        input: passing,
        expect: Expect::Value(Outcome::Accepted),
    },
    Case {
        name: CaseName("a case without a registered process expectation is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                discrepancy(d).process = ProcessExpectation::Absent
            })
        },
        expect: Expect::Error(ProcessReceiptsError::ProcessRegistrationMissing),
    },
    Case {
        name: CaseName("output templates that cannot expand from a relative root are refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                let relative = PathText("relative".into());
                d.row.actual_request_root = RequestRoot::Written(relative.clone());
                process(d).actual_request_root = relative;
            })
        },
        expect: Expect::Error(ProcessReceiptsError::Output(
            ProcessOutputError::RootNotAbsoluteUtf8,
        )),
    },
    Case {
        name: CaseName("a reference side without an observed process is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| d.row.process_observation = ReferenceProcess::NotObserved),
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("a rule projection cannot attest a process failure"),
        word: Word::Refuses,
        input: || edited(passing, |d| d.case.comparison = CaseComparison::Rule),
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("a capture that recorded a metadata error is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                process(d).metadata_error = RecordedReason::Stated(ReasonText("unavailable".into()))
            })
        },
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("an observer with a deadline is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| process(d).observer_no_deadline = YesNo::No),
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("another exit status than registered is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| process(d).exit = ExitStatus(3)),
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("a signalled process is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| process(d).signal = Signal::Number(ExitStatus(15))),
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("a reference side reporting diagnostics is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| d.row.diagnostics = ReportedFindings::Reported(Vec::new())),
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("a reference side matching the intended output is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                d.row.raw_parity_matches_intended = IntentMatch::Matched(YesNo::Yes)
            })
        },
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("another template digest is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| process(d).template_sha256 = Pin::Sha256(sha('9'))),
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("another archive group digest is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                validation(d).process_observation_group_sha256 = Pin::Sha256(sha('9'))
            })
        },
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("file metadata the certificate does not bind is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| d.case.file_metadata_sha256 = FileMetadataDigest::Declared(sha('9'))),
        expect: Expect::Error(ProcessReceiptsError::FileMetadataIdentityDiffers),
    },
    Case {
        name: CaseName("a receipt without a request root is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| d.row.actual_request_root = RequestRoot::Unwritten),
        expect: Expect::Error(ProcessReceiptsError::RequestRootMissing),
    },
    Case {
        name: CaseName("an observation under another root than the receipt is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                process(d).actual_request_root = PathText("/elsewhere".into())
            })
        },
        expect: Expect::Error(ProcessReceiptsError::ObservationRootDiffers),
    },
    Case {
        name: CaseName("an observation without raw artifact pointers is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| process(d).artifacts = ProcessArtifactsRecord::Unattached),
        expect: Expect::Error(ProcessReceiptsError::MissingRawProcessPath),
    },
    Case {
        name: CaseName("a raw artifact keyed for another case is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                artifacts(d).stdout.path =
                    PathText("attempt/other/stdout.bin".into())
            })
        },
        expect: Expect::Error(ProcessReceiptsError::ArtifactBelongsToAnotherCase),
    },
    Case {
        name: CaseName("absent captured bytes are refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                let path = stdout_path(d);
                d.artifacts.remove(&path);
            })
        },
        expect: Expect::Error(ProcessReceiptsError::MissingCapturedProcessBytes),
    },
    Case {
        name: CaseName("captured bytes that do not hash to their pointer are refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                let path = stdout_path(d);
                d.artifacts.insert(path, CapturedBytes(b"forged".to_vec()));
            })
        },
        expect: Expect::Error(ProcessReceiptsError::CapturedByteDigestDiffers),
    },
    Case {
        name: CaseName("forged bytes with a matching pointer differ from the templates"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                let path = stdout_path(d);
                d.artifacts.insert(path, CapturedBytes(b"forged".to_vec()));
                artifacts(d).stdout.sha256 = bytes_digest(b"forged");
            })
        },
        expect: Expect::Error(ProcessReceiptsError::CapturedBytesDifferFromTemplates),
    },
    Case {
        name: CaseName("a capture that failed to record its identity is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                let key = d.invocations.keys().next().unwrap().clone();
                d.invocations
                    .insert(key, InvocationDocument::Failed(failed()));
            })
        },
        expect: Expect::Error(ProcessReceiptsError::MetadataCaptureFailed),
    },
    Case {
        name: CaseName("archived captures sharing a raw artifact are refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                if let ProcessExpectation::Failure(process) = &mut discrepancy(d).process {
                    process.observations[1] = process.observations[0].clone();
                }
            })
        },
        expect: Expect::Error(ProcessReceiptsError::Archive(
            ProcessFailureError::GroupsShareCaptures,
        )),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    validate(
        &input.case,
        &input.row,
        &input.artifacts,
        &input.invocations,
    )
    .map(|()| Outcome::Accepted)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[cfg(test)]
mod invocation_test;
