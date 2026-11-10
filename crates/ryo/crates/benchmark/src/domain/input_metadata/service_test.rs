//! Stable optional input identity, distinct truth states and required-observation admission.
use super::errors::InputMetadataError;
use super::service::require;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::linter::RelativePath;
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration;
use ryo_benchmark_contracts::domain::workloads::types::FileObservation;
use std::collections::BTreeMap;

/// What each scenario observes.
#[derive(Clone, Copy)]
enum Scenario {
    Unspecified,
    DeclaredEmpty,
    ModeDiffers,
    PathFramingDiffers,
    RequiredWithoutMetadata,
    RequiredWithEmptyMetadata,
    RequiredWithKnownFalse,
    NamesUndeclaredSource,
}

/// What a scenario answers.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// No digest: the identity stays legacy.
    Legacy,
    /// A digest exists.
    Bound,
    /// Two declarations digest differently.
    Distinct,
    /// The required observation is admitted.
    Admitted,
}

type Input = Scenario;
type Value = Verdict;
type Failure = InputMetadataError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an unspecified identity stays legacy"),
        word: Word::Succeeds,
        input: || Scenario::Unspecified,
        expect: Expect::Value(Verdict::Legacy),
    },
    Case {
        name: CaseName("a declared empty mode set is bound to a digest"),
        word: Word::Succeeds,
        input: || Scenario::DeclaredEmpty,
        expect: Expect::Value(Verdict::Bound),
    },
    Case {
        name: CaseName("a declared mode changes the digest"),
        word: Word::Succeeds,
        input: || Scenario::ModeDiffers,
        expect: Expect::Value(Verdict::Distinct),
    },
    Case {
        name: CaseName("path boundaries are framed in the digest"),
        word: Word::Succeeds,
        input: || Scenario::PathFramingDiffers,
        expect: Expect::Value(Verdict::Distinct),
    },
    Case {
        name: CaseName("a known false is a complete observation"),
        word: Word::Succeeds,
        input: || Scenario::RequiredWithKnownFalse,
        expect: Expect::Value(Verdict::Admitted),
    },
    Case {
        name: CaseName("metadata naming a source the case never declared is refused"),
        word: Word::Refuses,
        input: || Scenario::NamesUndeclaredSource,
        expect: Expect::Error(InputMetadataError::UndeclaredSource),
    },
    Case {
        name: CaseName("a required observation without metadata is refused"),
        word: Word::Refuses,
        input: || Scenario::RequiredWithoutMetadata,
        expect: Expect::Error(InputMetadataError::RequiredObservationsMissing),
    },
    Case {
        name: CaseName("a required observation with empty metadata is refused"),
        word: Word::Refuses,
        input: || Scenario::RequiredWithEmptyMetadata,
        expect: Expect::Error(InputMetadataError::RequiredObservationsMissing),
    },
];

fn digest(metadata: &BTreeMap<RelativePath, bool>) -> Result<Option<ArtifactDigest>, ReportsError> {
    ArtifactDigest::file_metadata(Some(metadata).map(|values| {
        values.iter().map(|(path, executable)| FileMetadataEntry {
            path,
            executable: if *executable {
                FileExecutable::Set
            } else {
                FileExecutable::Unset
            },
        })
    }))
    .map_err(ReportsError::from)
}

fn distinct(
    left: &BTreeMap<RelativePath, bool>,
    right: &BTreeMap<RelativePath, bool>,
) -> Result<Verdict, ReportsError> {
    if digest(left)? == digest(right)? {
        return Ok(Verdict::Bound);
    }
    Ok(Verdict::Distinct)
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let file = RelativePath("λ.py".into());
    let empty = BTreeMap::new();
    let no = BTreeMap::from([(file.clone(), false)]);
    let required = |metadata: &FileMetadataDeclaration| {
        require(
            FileObservation::PosixExecutable,
            std::slice::from_ref(&file),
            metadata,
        )
        .map(|()| Verdict::Admitted)
    };
    match scenario {
        Scenario::Unspecified => {
            ArtifactDigest::file_metadata(None::<std::iter::Empty<FileMetadataEntry<'_>>>)
                .map_err(ReportsError::from)
                .map(|value| value.map_or(Verdict::Legacy, |_| Verdict::Bound))
        }
        Scenario::DeclaredEmpty => {
            digest(&empty).map(|value| value.map_or(Verdict::Legacy, |_| Verdict::Bound))
        }
        Scenario::ModeDiffers => distinct(&no, &BTreeMap::from([(file.clone(), true)])),
        Scenario::PathFramingDiffers => distinct(
            &BTreeMap::from([
                (RelativePath("ab".into()), true),
                (RelativePath("c".into()), false),
            ]),
            &BTreeMap::from([
                (RelativePath("a".into()), true),
                (RelativePath("bc".into()), false),
            ]),
        ),
        Scenario::NamesUndeclaredSource => require(
            FileObservation::Undeclared,
            std::slice::from_ref(&file),
            &FileMetadataDeclaration::Declared(BTreeMap::from([(
                RelativePath("other.py".into()),
                FileExecutable::Set,
            )])),
        )
        .map(|()| Verdict::Admitted),
        Scenario::RequiredWithoutMetadata => required(&FileMetadataDeclaration::Undeclared),
        Scenario::RequiredWithEmptyMetadata => {
            required(&FileMetadataDeclaration::Declared(BTreeMap::new()))
        }
        Scenario::RequiredWithKnownFalse => required(&FileMetadataDeclaration::Declared(
            BTreeMap::from([(file.clone(), FileExecutable::Unset)]),
        )),
    }
    .map_err(|error| {
        let text = error.to_string();
        [
            InputMetadataError::UndeclaredSource,
            InputMetadataError::RequiredObservationsMissing,
        ]
        .into_iter()
        .find(|refusal| refusal.to_string() == text)
        .expect("a refusal of this part")
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
