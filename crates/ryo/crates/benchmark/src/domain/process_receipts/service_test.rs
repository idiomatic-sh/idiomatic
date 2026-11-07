//! Behavioral proof of `process_receipts`: a reference process receipt is rebuilt from registration and actual roots.
use super::errors::ProcessReceiptsError;
use super::fixtures::{Documents, edited, invocation, passing, process, stdout_path};
use super::service::validate;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::process_failure::errors::ProcessFailureError;
use crate::domain::registered_inputs::types::RefusalText;
use crate::outbound::inventory::types::{
    ArgumentText, FailedInvocation, InvocationDocument, Pin, Signal, case_document::Comparison,
};
use crate::outbound::suite::types::{
    observation_document::ExitStatus,
    text::{CanonicalJson, PathText, ReasonText, Sha256Text, YesNo},
};
use crate::outbound::suite::types::process::ProcessFailureWire;
use sha2::Digest as _;
use std::collections::BTreeMap;

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
                d.case.reference_discrepancy.as_mut().unwrap().process = None
            })
        },
        expect: Expect::Error(ProcessReceiptsError::ProcessRegistrationMissing),
    },
    Case {
        name: CaseName(
            "a registered failure without a positive exit is refused with its own words",
        ),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                if let Some(ProcessFailureWire::ProcessFailure { exit, .. }) = d
                    .case
                    .reference_discrepancy
                    .as_mut()
                    .unwrap()
                    .process
                    .as_mut()
                {
                    *exit = 0;
                }
            })
        },
        expect: Expect::Error(ProcessReceiptsError::Refused(RefusalText(String::new()))),
    },
    Case {
        name: CaseName("a reference side without an observed process is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| d.row.process_observation = None),
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("a rule projection cannot attest a process failure"),
        word: Word::Refuses,
        input: || edited(passing, |d| d.case.comparison = Comparison::Rule),
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("a capture that recorded a metadata error is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                process(d).metadata_error = Some(ReasonText("unavailable".into()))
            })
        },
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("an observer with a deadline is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                process(d).observer_no_deadline = YesNo::No
            })
        },
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
        input: || edited(passing, |d| process(d).signal = Some(ExitStatus(15))),
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("a reference side reporting diagnostics is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| d.row.diagnostics = Some(Vec::new())),
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("a reference side matching the intended output is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                d.row.raw_parity_matches_intended = Some(YesNo::Yes)
            })
        },
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("another template digest is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| process(d).template_sha256 = Some(sha('9'))),
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("another archive group digest is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                d.row
                    .discrepancy_validation
                    .as_mut()
                    .unwrap()
                    .process_observation_group_sha256 = Some(sha('9'))
            })
        },
        expect: Expect::Error(ProcessReceiptsError::ReferenceVariantDiffers),
    },
    Case {
        name: CaseName("file metadata the certificate does not bind is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| d.case.file_metadata_sha256 = Some(sha('9'))),
        expect: Expect::Error(ProcessReceiptsError::FileMetadataIdentityDiffers),
    },
    Case {
        name: CaseName("a receipt without a request root is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| d.row.actual_request_root = None),
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
        input: || edited(passing, |d| process(d).artifacts = None),
        expect: Expect::Error(ProcessReceiptsError::MissingRawProcessPath),
    },
    Case {
        name: CaseName("a raw artifact keyed for another case is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                process(d).artifacts.as_mut().unwrap().stdout.path =
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
                d.artifacts.insert(path, b"forged".to_vec());
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
                d.artifacts.insert(path, b"forged".to_vec());
                process(d).artifacts.as_mut().unwrap().stdout.sha256 = bytes_digest(b"forged");
            })
        },
        expect: Expect::Error(ProcessReceiptsError::CapturedBytesDifferFromTemplates),
    },
    Case {
        name: CaseName("an undecoded raw invocation is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| d.invocations.clear()),
        expect: Expect::Error(ProcessReceiptsError::MissingRawInvocation),
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
        name: CaseName("file metadata the invocation binds alone is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                invocation(d).file_metadata_sha256 = Pin::Sha256(sha('9'))
            })
        },
        expect: Expect::Error(ProcessReceiptsError::InvocationFileMetadataDiffers),
    },
    Case {
        name: CaseName("another tool in the invocation is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| invocation(d).tool = ReasonText("other".into())),
        expect: Expect::Error(ProcessReceiptsError::InvocationToolDiffers),
    },
    Case {
        name: CaseName("another version in the invocation is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| invocation(d).version = ReasonText("2".into())),
        expect: Expect::Error(ProcessReceiptsError::InvocationVersionDiffers),
    },
    Case {
        name: CaseName("another request root in the invocation is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                invocation(d).actual_request_root = PathText("/elsewhere".into())
            })
        },
        expect: Expect::Error(ProcessReceiptsError::InvocationRootDiffers),
    },
    Case {
        name: CaseName("other files in the invocation are refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                invocation(d).files = vec![PathText("other.py".into())]
            })
        },
        expect: Expect::Error(ProcessReceiptsError::InvocationFilesDiffer),
    },
    Case {
        name: CaseName("another source identity in the invocation is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| invocation(d).source_sha256 = Pin::Absent),
        expect: Expect::Error(ProcessReceiptsError::InvocationSourceDiffers),
    },
    Case {
        name: CaseName("another executable in the invocation is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| invocation(d).executable_sha256 = sha('b')),
        expect: Expect::Error(ProcessReceiptsError::InvocationExecutableDiffers),
    },
    Case {
        name: CaseName("another settings identity in the invocation is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| invocation(d).settings_sha256 = sha('9')),
        expect: Expect::Error(ProcessReceiptsError::InvocationSettingsDiffer),
    },
    Case {
        name: CaseName("another profile identity in the invocation is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| invocation(d).profile_sha256 = sha('9')),
        expect: Expect::Error(ProcessReceiptsError::InvocationProfileDiffers),
    },
    Case {
        name: CaseName("another exit status in the invocation is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| invocation(d).exit = ExitStatus(3)),
        expect: Expect::Error(ProcessReceiptsError::InvocationExitDiffers),
    },
    Case {
        name: CaseName("a signal in the invocation is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                invocation(d).signal = Signal::Number(ExitStatus(15))
            })
        },
        expect: Expect::Error(ProcessReceiptsError::InvocationSignalDiffers),
    },
    Case {
        name: CaseName("a deadline in the invocation is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| invocation(d).observer_no_deadline = YesNo::No),
        expect: Expect::Error(ProcessReceiptsError::InvocationObserverDiffers),
    },
    Case {
        name: CaseName("normalized settings that do not hash to the pinned identity are refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                invocation(d).normalized_settings.canonical = CanonicalJson("{}".into())
            })
        },
        expect: Expect::Error(ProcessReceiptsError::MissingPinnedNormalizedInvocation),
    },
    Case {
        name: CaseName("an invocation without a state boundary is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| invocation(d).state = PathText(String::new())),
        expect: Expect::Error(ProcessReceiptsError::RawStateMissing),
    },
    Case {
        name: CaseName("raw argv that does not establish the pinned settings is refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                invocation(d).arguments = vec![ArgumentText("--changed-flag".into())]
            })
        },
        expect: Expect::Error(ProcessReceiptsError::RawSettingsNotEstablished),
    },
    Case {
        name: CaseName("archived captures sharing a raw artifact are refused"),
        word: Word::Refuses,
        input: || {
            edited(passing, |d| {
                if let Some(ProcessFailureWire::ProcessFailure { observations, .. }) = d
                    .case
                    .reference_discrepancy
                    .as_mut()
                    .unwrap()
                    .process
                    .as_mut()
                {
                    observations[1] = observations[0].clone();
                }
            })
        },
        expect: Expect::Error(ProcessReceiptsError::Archive(
            ProcessFailureError::GroupsShareCaptures,
        )),
    },
];

/// A refusal in a library's words is one kind of refusal, whatever the words.
fn words_dropped(error: Failure) -> Failure {
    match error {
        ProcessReceiptsError::Refused(_) => {
            ProcessReceiptsError::Refused(RefusalText(String::new()))
        }
        other => other,
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    validate(
        &input.case,
        &input.row,
        &input.artifacts,
        &input.invocations,
    )
    .map(|()| Outcome::Accepted)
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
