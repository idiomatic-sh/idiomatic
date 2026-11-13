//! The raw invocation record binds every identity the registration and the receipt state.
use ryo_benchmark_contracts::domain::documents::types::text::value::{
    CanonicalJson, ExitStatus, PathText, ReasonText, YesNo,
};
use sha2::Digest as _;

use super::super::{
    errors::ProcessReceiptsError,
    fixtures::{Documents, edited, invocation, passing},
};
use super::{Outcome, sha};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::ArgumentText;
use ryo_benchmark_contracts::domain::documents::types::text::value::{Pin, Signal};

type Input = Documents;
type Value = Outcome;
type Failure = ProcessReceiptsError;

fn subject(input: Input) -> Result<Value, Failure> {
    super::subject(input)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a receipt whose raw invocation states every identity is accepted"),
        word: Word::Succeeds,
        input: passing,
        expect: Expect::Value(Outcome::Accepted),
    },
    Case {
        name: CaseName("an undecoded raw invocation is refused"),
        word: Word::Refuses,
        input: || edited(passing, |d| d.invocations.clear()),
        expect: Expect::Error(ProcessReceiptsError::MissingRawInvocation),
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
];

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
