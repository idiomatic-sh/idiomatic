//! Behavioral proof of `registered_inputs`: what a case may declare about its files and project scope.
use super::errors::RegisteredInputsError;
use super::service::{metadata_digest, project_inputs_digest};
use crate::cases::{Case, CaseName, Expect, Word, run};
use serde_json::{Value as Json, json};

/// Which identity of the registered case is recomputed.
#[derive(Clone, Copy, Debug)]
enum Identity {
    FileMetadata,
    ProjectInputs,
}

type Input = (Identity, Json);
type Value = Json;
type Failure = RegisteredInputsError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a case without file declarations has no metadata identity"),
        word: Word::Succeeds,
        input: || (Identity::FileMetadata, json!({"files": ["input.txt"]})),
        expect: Expect::Value(Json::Null),
    },
    Case {
        name: CaseName("a case without a project scope has no project identity"),
        word: Word::Succeeds,
        input: || (Identity::ProjectInputs, json!({})),
        expect: Expect::Value(Json::Null),
    },
    Case {
        name: CaseName("a case without its files is refused"),
        word: Word::Refuses,
        input: || (Identity::FileMetadata, json!({})),
        expect: Expect::Error(RegisteredInputsError::RegisteredFilesMissing),
    },
    Case {
        name: CaseName("an unknown file observation is refused"),
        word: Word::Refuses,
        input: || {
            (
                Identity::FileMetadata,
                json!({"files": ["a"], "file_observation": "other"}),
            )
        },
        expect: Expect::Error(RegisteredInputsError::UnsupportedFileObservation),
    },
    Case {
        name: CaseName("file metadata that is not an object is refused"),
        word: Word::Refuses,
        input: || {
            (
                Identity::FileMetadata,
                json!({"files": ["a"], "file_metadata": 5}),
            )
        },
        expect: Expect::Error(RegisteredInputsError::FileMetadataNotObject),
    },
    Case {
        name: CaseName("metadata naming a source the case never declared is refused"),
        word: Word::Refuses,
        input: || {
            (
                Identity::FileMetadata,
                json!({"files": ["a"], "file_metadata": {"b": true}}),
            )
        },
        expect: Expect::Error(RegisteredInputsError::UndeclaredMetadataSource),
    },
    Case {
        name: CaseName("an executable observation that is not a yes or no is refused"),
        word: Word::Refuses,
        input: || {
            (
                Identity::FileMetadata,
                json!({"files": ["a"], "file_metadata": {"a": "yes"}}),
            )
        },
        expect: Expect::Error(RegisteredInputsError::ExecutableNotBoolean),
    },
    Case {
        name: CaseName("a required observation without its metadata is refused"),
        word: Word::Refuses,
        input: || {
            (
                Identity::FileMetadata,
                json!({"files": ["a"], "file_observation": "posix-executable"}),
            )
        },
        expect: Expect::Error(RegisteredInputsError::InputObservationsMissing),
    },
    Case {
        name: CaseName("search roots that are not an array are refused"),
        word: Word::Refuses,
        input: || {
            (
                Identity::ProjectInputs,
                json!({"project_search_roots": "src"}),
            )
        },
        expect: Expect::Error(RegisteredInputsError::SearchRootsNotArray),
    },
    Case {
        name: CaseName("a search root that is not text is refused"),
        word: Word::Refuses,
        input: || {
            (
                Identity::ProjectInputs,
                json!({"project_search_roots": [1]}),
            )
        },
        expect: Expect::Error(RegisteredInputsError::SearchRootNotText),
    },
];

fn subject((identity, case): Input) -> Result<Value, Failure> {
    match identity {
        Identity::FileMetadata => metadata_digest(&case),
        Identity::ProjectInputs => project_inputs_digest(&case),
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
