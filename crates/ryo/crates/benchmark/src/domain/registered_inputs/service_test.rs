//! Behavioral proof of `registered_inputs`: what a case may declare about its files and project scope.
use super::errors::RegisteredInputsError;
use super::service::{metadata_digest, project_inputs_digest};
use super::types::{ObservationDeclaration, RegisteredInputs};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::linter::{GivenProjectInventory, RelativePath};
use ryo_benchmark_contracts::domain::artifacts::types::{
    FileExecutable, digest::value::ArtifactDigest,
};
use std::collections::BTreeMap;

/// Which identity of the registered case is recomputed.
#[derive(Clone, Copy, Debug)]
enum Identity {
    FileMetadata,
    ProjectInputs,
}

type Input = (Identity, RegisteredInputs);
type Value = Option<ArtifactDigest>;
type Failure = RegisteredInputsError;

fn path(text: &str) -> RelativePath {
    RelativePath(text.into())
}

fn declaring(
    files: &[&str],
    observation: ObservationDeclaration,
    metadata: Option<&[(&str, FileExecutable)]>,
) -> RegisteredInputs {
    RegisteredInputs {
        files: files.iter().map(|file| path(file)).collect(),
        observation,
        metadata: metadata.map(|values| {
            values
                .iter()
                .map(|(file, executable)| (path(file), *executable))
                .collect::<BTreeMap<_, _>>()
        }),
        search_roots: None,
        inventory: GivenProjectInventory::default(),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a case without file declarations has no metadata identity"),
        word: Word::Succeeds,
        input: || {
            (
                Identity::FileMetadata,
                declaring(&["input.txt"], ObservationDeclaration::Absent, None),
            )
        },
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("a case without a project scope has no project identity"),
        word: Word::Succeeds,
        input: || {
            (
                Identity::ProjectInputs,
                declaring(&["input.txt"], ObservationDeclaration::Absent, None),
            )
        },
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("metadata naming a source the case never declared is refused"),
        word: Word::Refuses,
        input: || {
            (
                Identity::FileMetadata,
                declaring(
                    &["a"],
                    ObservationDeclaration::Absent,
                    Some(&[("b", FileExecutable::Set)]),
                ),
            )
        },
        expect: Expect::Error(RegisteredInputsError::UndeclaredMetadataSource),
    },
    Case {
        name: CaseName("a required observation without its metadata is refused"),
        word: Word::Refuses,
        input: || {
            (
                Identity::FileMetadata,
                declaring(&["a"], ObservationDeclaration::Required, None),
            )
        },
        expect: Expect::Error(RegisteredInputsError::InputObservationsMissing),
    },
    Case {
        name: CaseName("a required observation that covers only some files is refused"),
        word: Word::Refuses,
        input: || {
            (
                Identity::FileMetadata,
                declaring(
                    &["a", "b"],
                    ObservationDeclaration::Required,
                    Some(&[("a", FileExecutable::Unset)]),
                ),
            )
        },
        expect: Expect::Error(RegisteredInputsError::InputObservationsMissing),
    },
];

fn subject((identity, inputs): Input) -> Result<Value, Failure> {
    match identity {
        Identity::FileMetadata => metadata_digest(&inputs),
        Identity::ProjectInputs => project_inputs_digest(&inputs),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
