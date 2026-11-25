//! Legitimate path substitution and independent byte-preservation negatives.
use super::errors::ProcessOutputError;
use super::service::{expand, observation_key};
use super::types::ReceiptKeyParts;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::canonical_text::types::Label;
use benchmark_contracts::domain::process::types::OutputPart as Part;
use benchmark_contracts::domain::process::types::PathEncoding as Encoding;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// What each scenario expands or frames.
#[derive(Clone, Copy)]
enum Scenario {
    UnicodeJsonPath,
    RelativeRoot,
    EmptyRoot,
    UnlistedSlot,
    EscapingSlot,
    AbsoluteSlot,
    EmptySlot,
    BoundaryShift,
    RoleShift,
    KeyLength,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = ProcessOutputError;

/// What the subject answers when the expansion is the independently built document.
const FRAMED_JSON_PATH: &str =
    "the file is the root joined with the path, escaped as JSON, around the bytes it was framed in";

/// An absolute directory on every platform: `/tmp/root` has no drive on Windows and is not absolute there.
fn root(name: &str) -> PathBuf {
    std::env::temp_dir().join(name)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a JSON path is escaped without touching unrelated bytes"),
        word: Word::Succeeds,
        input: || Scenario::UnicodeJsonPath,
        expect: Expect::Value(Cow::Borrowed(FRAMED_JSON_PATH)),
    },
    Case {
        name: CaseName("moving a byte between rule and case changes the key"),
        word: Word::Succeeds,
        input: || Scenario::BoundaryShift,
        expect: Expect::Value(Cow::Borrowed("distinct")),
    },
    Case {
        name: CaseName("the reference and candidate roles frame differently"),
        word: Word::Succeeds,
        input: || Scenario::RoleShift,
        expect: Expect::Value(Cow::Borrowed("distinct")),
    },
    Case {
        name: CaseName("a case name never becomes a path component of the key"),
        word: Word::Succeeds,
        input: || Scenario::KeyLength,
        expect: Expect::Value(Cow::Borrowed("64")),
    },
    Case {
        name: CaseName("a relative root is refused"),
        word: Word::Refuses,
        input: || Scenario::RelativeRoot,
        expect: Expect::Error(ProcessOutputError::RootNotAbsoluteUtf8),
    },
    Case {
        name: CaseName("an empty root is refused"),
        word: Word::Refuses,
        input: || Scenario::EmptyRoot,
        expect: Expect::Error(ProcessOutputError::RootNotAbsoluteUtf8),
    },
    Case {
        name: CaseName("an undeclared slot is refused"),
        word: Word::Refuses,
        input: || Scenario::UnlistedSlot,
        expect: Expect::Error(ProcessOutputError::SlotNotDeclared),
    },
    Case {
        name: CaseName("an escaping slot is refused"),
        word: Word::Refuses,
        input: || Scenario::EscapingSlot,
        expect: Expect::Error(ProcessOutputError::SlotNotDeclared),
    },
    Case {
        name: CaseName("an absolute slot is refused"),
        word: Word::Refuses,
        input: || Scenario::AbsoluteSlot,
        expect: Expect::Error(ProcessOutputError::SlotNotDeclared),
    },
    Case {
        name: CaseName("an empty slot is refused"),
        word: Word::Refuses,
        input: || Scenario::EmptySlot,
        expect: Expect::Error(ProcessOutputError::SlotNotDeclared),
    },
];

fn slot(file: &str) -> Result<Value, Failure> {
    let parts = [Part::InputPath {
        file: file.into(),
        encoding: Encoding::Utf8Bytes,
    }];
    expand(&parts, &root("root"), &[PathBuf::from("case.py")])
        .map(|bytes| String::from_utf8_lossy(&bytes.0).into_owned().into())
}

fn key(rule: &str, name: &str, side: &str) -> String {
    observation_key(&ReceiptKeyParts {
        rule: Label::from(rule),
        name: Label::from(name),
        side: Label::from(side),
    })
    .0
}

fn distinct(left: &str, right: &str) -> Result<Value, Failure> {
    Ok(Cow::Borrowed(if left == right {
        "same"
    } else {
        "distinct"
    }))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::UnicodeJsonPath => {
            let parts = [
                Part::Literal(benchmark_contracts::domain::process::types::OutputText(
                    "{\"file\":\"".into(),
                )),
                Part::InputPath {
                    file: "é\".py".into(),
                    encoding: Encoding::JsonStringContent,
                },
                Part::Literal(benchmark_contracts::domain::process::types::OutputText(
                    "\",\"assertion\":\"x < y\"}".into(),
                )),
            ];
            let root = root("日本");
            let expanded = expand(&parts, &root, &[PathBuf::from("é\".py")])
                .map(|bytes| String::from_utf8_lossy(&bytes.0).into_owned())?;
            // The expectation is the platform's own join of the root and the file, escaped by an independent JSON encoder.
            let path = serde_json::to_string(&root.join("é\".py").to_string_lossy())
                .expect("a path is a JSON string");
            let expected = format!("{{\"file\":{},\"assertion\":\"x < y\"}}", path);
            if expanded == expected {
                Ok(Cow::Borrowed(FRAMED_JSON_PATH))
            } else {
                Ok(Cow::Owned(expanded))
            }
        }
        Scenario::RelativeRoot => expand(&[], Path::new("relative"), &[])
            .map(|bytes| String::from_utf8_lossy(&bytes.0).into_owned().into()),
        Scenario::EmptyRoot => expand(&[], Path::new(""), &[])
            .map(|bytes| String::from_utf8_lossy(&bytes.0).into_owned().into()),
        Scenario::UnlistedSlot => slot("other.py"),
        Scenario::EscapingSlot => slot("../case.py"),
        Scenario::AbsoluteSlot => slot("/case.py"),
        Scenario::EmptySlot => slot(""),
        Scenario::BoundaryShift => {
            distinct(&key("a", "bc", "reference"), &key("ab", "c", "reference"))
        }
        Scenario::RoleShift => distinct(&key("a", "bc", "reference"), &key("a", "bc", "candidate")),
        Scenario::KeyLength => Ok(key("a", "../name", "reference").len().to_string().into()),
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
