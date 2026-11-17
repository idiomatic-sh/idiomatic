//! Behavioral proof of `invocation_identity`: state, root and separators are normalized, the worker count is spelled as one
//! worker, and nothing else.
use super::errors::InvocationIdentityError;
use super::service::{boundaries, normalize, normalize_text};
use super::types::InvocationText;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::fixtures::case;
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use std::borrow::Cow;
use std::ffi::OsString;

/// What a row asks of the normalization.
#[derive(Clone, Copy, Debug)]
enum Operation {
    Spelling,
    Invocation,
    /// The arguments are the invocation's environment, each `KEY=VALUE`.
    Environment,
}

/// An invocation spelled against the case's state and root.
struct Request {
    operation: Operation,
    arguments: Vec<OsString>,
    directory: &'static str,
}

type Input = Request;
type Value = Cow<'static, str>;
type Failure = InvocationIdentityError;

fn asking(operation: Operation, arguments: Vec<OsString>, directory: &'static str) -> Request {
    Request {
        operation,
        arguments,
        directory,
    }
}

/// An argument no UTF8 text spells.
#[cfg(unix)]
fn non_utf8() -> OsString {
    use std::os::unix::ffi::OsStringExt as _;
    OsString::from_vec(vec![0xff])
}

/// An argument no UTF8 text spells: an unpaired surrogate, which Windows paths allow.
#[cfg(not(unix))]
fn non_utf8() -> OsString {
    use std::os::windows::ffi::OsStringExt as _;
    OsString::from_wide(&[0xD800])
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("state, root and backslashes are replaced in one spelling"),
        word: Word::Succeeds,
        input: || asking(Operation::Spelling, vec!["cache/a\\b".into()], "."),
        expect: Expect::Value(Cow::Borrowed("$STATE/a/b")),
    },
    Case {
        name: CaseName("every argument and the directory of an invocation are normalized"),
        word: Word::Succeeds,
        input: || {
            asking(
                Operation::Invocation,
                vec!["--flag".into(), "cache".into()],
                "cache",
            )
        },
        expect: Expect::Value(Cow::Borrowed("--flag $STATE|$STATE")),
    },
    Case {
        name: CaseName("the worker count is spelled as one worker, every other variable as it is"),
        word: Word::Succeeds,
        input: || {
            asking(
                Operation::Environment,
                vec!["RAYON_NUM_THREADS=4".into(), "OTHER=cache".into()],
                ".",
            )
        },
        expect: Expect::Value(Cow::Borrowed("OTHER=$STATE RAYON_NUM_THREADS=1")),
    },
    Case {
        name: CaseName("cargo's job count is spelled as one worker"),
        word: Word::Succeeds,
        input: || {
            asking(
                Operation::Environment,
                vec!["CARGO_BUILD_JOBS=4".into()],
                ".",
            )
        },
        expect: Expect::Value(Cow::Borrowed("CARGO_BUILD_JOBS=1")),
    },
    Case {
        name: CaseName(
            "the value after a thread-count flag is spelled as one worker, every other argument as it is",
        ),
        word: Word::Succeeds,
        input: || {
            asking(
                Operation::Invocation,
                vec![
                    "--threads".into(),
                    "4".into(),
                    "--format".into(),
                    "4".into(),
                ],
                ".",
            )
        },
        expect: Expect::Value(Cow::Borrowed("--threads 1 --format 4|$ROOT")),
    },
    Case {
        name: CaseName("a non UTF8 argument is refused"),
        word: Word::Refuses,
        input: || asking(Operation::Invocation, vec![non_utf8()], "."),
        expect: Expect::Error(InvocationIdentityError::NonUtf8),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    let case = case();
    match input.operation {
        Operation::Spelling => {
            let spelled = normalize_text(
                &InvocationText(input.arguments[0].to_string_lossy().into_owned()),
                &boundaries(&case),
            );
            Ok(Cow::Owned(spelled.0))
        }
        Operation::Environment => {
            let invocation = Invocation {
                executable: "tool".into(),
                arguments: Vec::new(),
                environment: input
                    .arguments
                    .iter()
                    .filter_map(|pair| {
                        let (key, value) = pair.to_str()?.split_once('=')?;
                        Some((key.into(), value.into()))
                    })
                    .collect(),
                directory: input.directory.into(),
            };
            let normalized = normalize(&invocation, &case)?;
            Ok(Cow::Owned(
                normalized
                    .environment
                    .iter()
                    .map(|(key, value)| {
                        format!("{}={}", key.to_string_lossy(), value.to_string_lossy())
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
            ))
        }
        Operation::Invocation => {
            let invocation = Invocation {
                executable: "tool".into(),
                arguments: input.arguments,
                environment: std::collections::BTreeMap::new(),
                directory: input.directory.into(),
            };
            let normalized = normalize(&invocation, &case)?;
            let arguments = normalized
                .arguments
                .iter()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" ");
            Ok(Cow::Owned(format!(
                "{arguments}|{}",
                normalized.directory.display()
            )))
        }
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
