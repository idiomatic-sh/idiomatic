//! A profile that requires file observations refuses a case that omits them before anything executes.
use super::constants;
use super::service::admit;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::fixtures::{case, profile};
use crate::outbound::suite::fixtures::fake::SuiteFake;
use ryo_benchmark_contracts::domain::artifacts::types::FileExecutable;
use ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration;
use ryo_benchmark_contracts::domain::workloads::types::{FileObservation, PreparedCase};
use std::path::Path;

/// What the profile requires and what the case declares.
#[derive(Clone, Copy)]
enum Scenario {
    /// The profile requires nothing.
    NothingRequired,
    /// The profile requires observations and the case declares its file's.
    Declared,
    /// The profile requires observations and the case omits them.
    Omitted,
    /// The rules directory has no parent that could hold a profile.
    NoProfileBoundary,
}

/// What a row established once admission answered.
#[derive(Debug, PartialEq, Eq)]
enum Admitted {
    Cases,
}

/// What a refused admission states.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    ObservationsMissing,
    MissingProfileBoundary,
    Unexpected,
}

type Input = Scenario;
type Value = Admitted;
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a profile without file observations admits a case without metadata"),
        word: Word::Succeeds,
        input: || Scenario::NothingRequired,
        expect: Expect::Value(Admitted::Cases),
    },
    Case {
        name: CaseName("a profile that requires observations admits a case that declares them"),
        word: Word::Succeeds,
        input: || Scenario::Declared,
        expect: Expect::Value(Admitted::Cases),
    },
    Case {
        name: CaseName("a profile that requires observations refuses a case that omits them"),
        word: Word::Refuses,
        input: || Scenario::Omitted,
        expect: Expect::Error(Refusal::ObservationsMissing),
    },
    Case {
        name: CaseName("a rules directory without a profile boundary is refused"),
        word: Word::Refuses,
        input: || Scenario::NoProfileBoundary,
        expect: Expect::Error(Refusal::MissingProfileBoundary),
    },
];

fn prepared(declared: bool) -> PreparedCase {
    let mut prepared = case();
    if declared {
        prepared.request.file_metadata = FileMetadataDeclaration::Declared(
            std::collections::BTreeMap::from([(prepared.request.files[0].clone(), FileExecutable::Unset)]),
        );
    }
    prepared
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut tool = profile();
    let (rules, cases) = match scenario {
        Scenario::NothingRequired => (Path::new("profile/rules"), prepared(false)),
        Scenario::Declared => {
            tool.file_observation = FileObservation::PosixExecutable;
            (Path::new("profile/rules"), prepared(true))
        }
        Scenario::Omitted => {
            tool.file_observation = FileObservation::PosixExecutable;
            (Path::new("profile/rules"), prepared(false))
        }
        Scenario::NoProfileBoundary => (Path::new(""), prepared(false)),
    };
    let suite = SuiteFake {
        profile: Some(tool),
        ..Default::default()
    };
    admit(&suite, rules, &[cases])
        .map(|()| Admitted::Cases)
        .map_err(|error| {
            let message = error.to_string();
            if message == constants::MISSING_CASE_PROFILE_BOUNDARY {
                Refusal::MissingProfileBoundary
            } else if message.contains("observation") {
                Refusal::ObservationsMissing
            } else {
                Refusal::Unexpected
            }
        })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
