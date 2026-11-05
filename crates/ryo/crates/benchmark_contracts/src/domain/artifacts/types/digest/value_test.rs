//! Exact display and independently pinned canonical observation identities.
use super::value::ArtifactDigest;
use crate::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use ryo::cases::{Case, CaseName, Expect, Word, run};
use std::{borrow::Cow, collections::BTreeMap};
#[derive(Clone, Copy)]
enum Input {
    Display,
    Unspecified,
    Empty,
    KnownFalse,
    KnownTrue,
    Framing,
    Unordered,
    Duplicate,
}
type Value = Option<Cow<'static, str>>;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("unordered metadata is refused"),
        word: Word::Refuses,
        input: || Input::Unordered,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("duplicate metadata is refused"),
        word: Word::Refuses,
        input: || Input::Duplicate,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("digest display preserves exact identity"),
        word: Word::Succeeds,
        input: || Input::Display,
        expect: Expect::Value(Some(Cow::Borrowed("pin"))),
    },
    Case {
        name: CaseName("unspecified observations have no identity"),
        word: Word::Succeeds,
        input: || Input::Unspecified,
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("declared empty observations retain an exact identity"),
        word: Word::Succeeds,
        input: || Input::Empty,
        expect: Expect::Value(Some(Cow::Borrowed(
            "34603e92af5a7e5008c383e4521c7cef56f8a2d16e8f31435eced3558f138134",
        ))),
    },
    Case {
        name: CaseName("UTF8 path and known false are bound"),
        word: Word::Succeeds,
        input: || Input::KnownFalse,
        expect: Expect::Value(Some(Cow::Borrowed(
            "d56ce6c48ee649323396fbad9d896ba6fa793aef6fda082db766163850b62194",
        ))),
    },
    Case {
        name: CaseName("known true has a distinct exact identity"),
        word: Word::Succeeds,
        input: || Input::KnownTrue,
        expect: Expect::Value(Some(Cow::Borrowed(
            "f364429fc2978f01f5c9f0cbc97d0831b85617e6467001bc92c5bd119b997d04",
        ))),
    },
    Case {
        name: CaseName("path boundaries are framed independently"),
        word: Word::Succeeds,
        input: || Input::Framing,
        expect: Expect::Value(Some(Cow::Borrowed("framed"))),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    use ryo::linter::RelativePath;
    let metadata = match input {
        Input::Unordered | Input::Duplicate => {
            let first = RelativePath("a".into());
            let second = RelativePath("b".into());
            let paths = match input {
                Input::Unordered => [&second, &first],
                _ => [&first, &first],
            };
            return ArtifactDigest::file_metadata(Some(paths.map(|path| FileMetadataEntry {
                path,
                executable: FileExecutable::Unset,
            })))
            .map(|value| value.map(|value| value.0.into()))
            .map_err(|error| {
                assert_eq!(
                    error.to_string(),
                    "metadata paths must be unique and strictly ordered"
                );
                ()
            });
        }
        Input::Display => return Ok(Some(ArtifactDigest("pin".into()).to_string().into())),
        Input::Unspecified => {
            return ArtifactDigest::file_metadata(None::<std::iter::Empty<FileMetadataEntry<'_>>>)
                .map(|value| value.map(|value| value.0.into()))
                .map_err(|_| ());
        }
        Input::Empty => BTreeMap::new(),
        Input::KnownFalse => BTreeMap::from([(RelativePath("λ.py".into()), false)]),
        Input::KnownTrue => BTreeMap::from([(RelativePath("λ.py".into()), true)]),
        Input::Framing => {
            let first = BTreeMap::from([
                (RelativePath("ab".into()), true),
                (RelativePath("c".into()), false),
            ]);
            let second = BTreeMap::from([
                (RelativePath("a".into()), true),
                (RelativePath("bc".into()), false),
            ]);
            assert_ne!(
                ArtifactDigest::file_metadata((Some(&first)).map(|values| values.iter().map(
                    |(path, executable)| FileMetadataEntry {
                        path,
                        executable: if *executable {
                            FileExecutable::Set
                        } else {
                            FileExecutable::Unset
                        }
                    }
                )))
                .unwrap(),
                ArtifactDigest::file_metadata((Some(&second)).map(|values| values.iter().map(
                    |(path, executable)| FileMetadataEntry {
                        path,
                        executable: if *executable {
                            FileExecutable::Set
                        } else {
                            FileExecutable::Unset
                        }
                    }
                )))
                .unwrap()
            );
            return Ok(Some(Cow::Borrowed("framed")));
        }
    };
    ArtifactDigest::file_metadata((Some(&metadata)).map(|values| {
        values.iter().map(|(path, executable)| FileMetadataEntry {
            path,
            executable: if *executable {
                FileExecutable::Set
            } else {
                FileExecutable::Unset
            },
        })
    }))
    .map(|value| value.map(|value| value.0.into()))
    .map_err(|_| ())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
