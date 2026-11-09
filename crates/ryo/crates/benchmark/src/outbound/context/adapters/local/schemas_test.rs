//! A receipt is spelled exactly as an observation document carries it, and a path JSON cannot spell is refused.
use super::receipt_text;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Which receipt is spelled.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// A listed-disk receipt with one present and one unobserved dependency.
    ListedDisk,
    /// An isolated receipt with no dependencies.
    Isolated,
    /// A receipt whose actual root is not UTF-8.
    NonUtf8Root,
}

/// Why a receipt is not spelled.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    /// The encoding refused the receipt as invalid.
    Invalid,
    /// Any other failure.
    Unexpected,
}

type Input = Scenario;
type Value = Cow<'static, str>;
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a listed-disk receipt spells its mode, roots, hash and every dependency"),
        word: Word::Succeeds,
        input: || Scenario::ListedDisk,
        expect: Expect::Value(Cow::Borrowed(
            r#"{"mode":"listed-disk","logical_root":"pkg","actual_root":"/snapshot/pkg","manifest_hash":"context-hash","dependencies":{"../__init__.py":null,"__init__.py":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]}}"#,
        )),
    },
    Case {
        name: CaseName("an isolated receipt spells its mode and an empty dependency map"),
        word: Word::Succeeds,
        input: || Scenario::Isolated,
        expect: Expect::Value(Cow::Borrowed(
            r#"{"mode":"isolated","logical_root":"pkg","actual_root":"/snapshot/pkg","manifest_hash":"context-hash","dependencies":{}}"#,
        )),
    },
    Case {
        name: CaseName("a root JSON cannot spell is refused as invalid"),
        word: Word::Refuses,
        input: || Scenario::NonUtf8Root,
        expect: Expect::Error(Refusal::Invalid),
    },
];

/// The receipt a scenario names.
fn receipt(scenario: Scenario) -> ProjectContextReceipt {
    let actual_root = match scenario {
        Scenario::NonUtf8Root => non_utf8_root(),
        Scenario::ListedDisk | Scenario::Isolated => PathBuf::from("/snapshot/pkg"),
    };
    ProjectContextReceipt {
        mode: match scenario {
            Scenario::Isolated => ProjectContextMode::Isolated,
            Scenario::ListedDisk | Scenario::NonUtf8Root => ProjectContextMode::ListedDisk,
        },
        logical_root: "pkg".into(),
        actual_root,
        manifest_hash: ArtifactDigest("context-hash".into()),
        dependencies: match scenario {
            Scenario::ListedDisk => BTreeMap::from([
                ("../__init__.py".into(), None),
                (
                    "__init__.py".into(),
                    Some(ryo::linter::DependencyFingerprint([1; 16])),
                ),
            ]),
            Scenario::Isolated | Scenario::NonUtf8Root => BTreeMap::new(),
        },
    }
}

/// A root path whose bytes are not UTF-8.
#[cfg(unix)]
fn non_utf8_root() -> PathBuf {
    use std::os::unix::ffi::OsStrExt as _;
    PathBuf::from(std::ffi::OsStr::from_bytes(b"/snapshot/\xff"))
}
/// A root path holding an unpaired surrogate, which is not UTF-8.
#[cfg(windows)]
fn non_utf8_root() -> PathBuf {
    use std::os::windows::ffi::OsStringExt as _;
    PathBuf::from(std::ffi::OsString::from_wide(&[0x005c, 0xd800]))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    receipt_text(&receipt(scenario))
        .map(|text| Cow::Owned(text.0))
        .map_err(|error| match error {
            ReportsError::Invalid(_) => Refusal::Invalid,
            ReportsError::Io(_) | ReportsError::Json(_) | ReportsError::Toml(_) => {
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
