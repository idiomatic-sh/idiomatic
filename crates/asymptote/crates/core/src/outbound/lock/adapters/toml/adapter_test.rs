//! The TOML adapter of the lock port: the contract over a temporary product, two operations of ryo's lock read
//! and verified unchanged and written again byte for byte, appends interrupted or at once, and a stray file, an
//! operation edited by hand, one that is no TOML and a lock beneath a file refused.

use super::adapter::LockTomlAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::manifest::types::ProductRoot;
use crate::domain::operations::service::fold;
use crate::domain::operations::types::{Operation, OperationId, OperationText};
use crate::outbound::lock::errors::LockError;
use crate::outbound::lock::fixtures::contract::check;
use crate::outbound::lock::interface::LockPort as _;

/// Two operations of ryo's lock, copied unchanged.
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/lock");

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The contract.
    Contract,
    /// the fixture operations, read, verified and folded.
    FirstAsymptotes,
    /// One of them checked out with CRLF.
    Crlf,
    /// One of them with a byte changed.
    Edited,
    /// A file named as an operation that is no TOML.
    NoToml,
    /// The lock's directory is a file.
    Unreadable,
    /// An operation appended beneath a file.
    Unwritable,
    /// Each of the fixture operations parsed and appended again: the same name and the same bytes.
    Rewritten,
    /// the fixture operations beside what an interrupted append leaves: its temporary file, outside the lock's directory.
    Interrupted,
    /// The same operation appended from many threads at once.
    Concurrent,
    /// A file in the lock's directory that is no operation.
    Stray,
    /// An appended operation's mode, beside a file `std::fs::write` makes there.
    Mode,
}

type Input = Question;
/// Whether the answer is the expected one.
type Value = bool;
type Failure = LockError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_appended_operation_has_the_mode_any_written_file_has"),
        word: Word::Succeeds,
        input: || Question::Mode,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_adapter_keeps_the_contract"),
        word: Word::Succeeds,
        input: || Question::Contract,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_first_asymptotes_operations_read_unchanged_and_fold"),
        word: Word::Succeeds,
        input: || Question::FirstAsymptotes,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_crlf_checkout_reads_as_its_lf_text"),
        word: Word::Succeeds,
        input: || Question::Crlf,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "an_operation_appended_again_is_written_byte_for_byte_as_the_first_asymptote_wrote_it",
        ),
        word: Word::Succeeds,
        input: || Question::Rewritten,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_interrupted_append_leaves_the_lock_readable_by_either_asymptote"),
        word: Word::Succeeds,
        input: || Question::Interrupted,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_same_operation_appended_at_once_from_many_threads_is_one_whole_file"),
        word: Word::Succeeds,
        input: || Question::Concurrent,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_in_the_lock_that_is_no_operation_is_refused"),
        word: Word::Refuses,
        input: || Question::Stray,
        expect: Expect::Refusal(|error| {
            if matches!(error, LockError::Malformed {operation,reason} if operation.ends_with("/.asymptote/operations/.0123.lock.writing") && reason == "its name is not a lowercase BLAKE3 digest")
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_operation_edited_by_hand_is_refused"),
        word: Word::Refuses,
        input: || Question::Edited,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(LockError::Address {
                    expected: "3079da431dc5201c8635a6fe9ae4774f1e9cf4fd07179f0fb2968656c05d5114"
                        .into(),
                    actual: "e0d14bf78c1dc620f5d0a355a75b7acf725c271fb490d5c27d919256e0fb8821"
                        .into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_operation_that_is_no_toml_is_refused"),
        word: Word::Refuses,
        input: || Question::NoToml,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(LockError::Malformed {
                    operation: "e670a9f8f35e4dc95c89b0427066f03b78763fb48cc0e3e3ca15c3b9572454cf"
                        .into(),
                    reason: "unclosed array, expected `]`".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_lock_that_cannot_be_listed_fails"),
        word: Word::Fails,
        input: || Question::Unreadable,
        expect: Expect::Refusal(|error| {
            if matches!(error, LockError::Unreadable {path,reason} if path.ends_with("/.asymptote/operations") && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_lock_beneath_a_file_cannot_be_written"),
        word: Word::Fails,
        input: || Question::Unwritable,
        expect: Expect::Refusal(|error| {
            if matches!(error, LockError::Unwritable {path,reason} if path.contains("/.asymptote/") && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// The permission bits of `path`; on a system with no modes, whether it is read-only.
fn mode(path: &std::path::Path) -> u32 {
    let permissions = std::fs::metadata(path).expect("its metadata").permissions();
    #[cfg(unix)]
    let bits = std::os::unix::fs::PermissionsExt::mode(&permissions) & 0o777;
    #[cfg(not(unix))]
    let bits = u32::from(permissions.readonly());
    bits
}

/// Whether an appended operation has the mode of a file `std::fs::write` makes beside it.
fn modes(root: &ProductRoot, operations: &std::path::Path) -> Result<Value, Failure> {
    let (id, text) = fixture();
    let operation = adapter(root).parse(&id, &OperationText(text.as_str().into()))?;
    adapter(root).append(&operation)?;
    let written = operations.join("written");
    std::fs::write(&written, "").expect("a file written as any is");
    Ok(mode(&operations.join(format!("{}.lock", id.0))) == mode(&written))
}

/// The adapter over the lock of the product at `root`.
fn adapter(root: &ProductRoot) -> LockTomlAdapter {
    LockTomlAdapter::new(root.clone())
}

fn product(directory: &std::path::Path) -> ProductRoot {
    ProductRoot(directory.display().to_string().into())
}

/// The smaller of the two fixtures: its id and its text.
fn fixture() -> (OperationId, String) {
    let path = std::fs::read_dir(FIXTURES)
        .expect("the fixtures")
        .map(|entry| entry.expect("a fixture").path())
        .min_by_key(|path| std::fs::metadata(path).map_or(u64::MAX, |meta| meta.len()))
        .expect("one fixture");
    let id = OperationId(
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .expect("a stem")
            .into(),
    );
    (id, std::fs::read_to_string(&path).expect("its text"))
}

/// Whether each of the fixture operations, parsed and appended again, keeps its name and its bytes.
fn rewritten(root: &ProductRoot, operations: &std::path::Path) -> Result<Value, Failure> {
    let mut same = true;
    for entry in std::fs::read_dir(FIXTURES).expect("the fixtures") {
        let path = entry.expect("a fixture").path();
        let id = OperationId(
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .expect("a stem")
                .into(),
        );
        let text = std::fs::read_to_string(&path).expect("its text");
        let operation = adapter(root).parse(&id, &OperationText(text.as_str().into()))?;
        adapter(root).append(&operation)?;
        same &= std::fs::read_to_string(operations.join(format!("{}.lock", id.0)))
            .is_ok_and(|again| again == text);
    }
    Ok(same)
}

/// Whether the same operation appended from many threads at once is one whole file.
fn concurrent(root: &ProductRoot, operations: &std::path::Path) -> Result<Value, Failure> {
    let (id, text) = fixture();
    let operation = adapter(root).parse(&id, &OperationText(text.as_str().into()))?;
    let written: Vec<Result<(), LockError>> = std::thread::scope(|scope| {
        let appends: Vec<_> =
            std::iter::repeat_with(|| scope.spawn(|| adapter(root).append(&operation)))
                .take(8)
                .collect();
        appends
            .into_iter()
            .map(|append| append.join().expect("an append"))
            .collect()
    });
    let names: Vec<_> = std::fs::read_dir(operations)
        .expect("the lock")
        .map(|entry| entry.expect("a file").file_name())
        .collect();
    Ok(written.iter().all(|append| *append == Ok(()))
        && names.len() == 1
        && std::fs::read_to_string(operations.join(format!("{}.lock", id.0)))
            .is_ok_and(|again| again == text))
}

/// Answers one row's question about the TOML lock adapter over a fresh temporary product.
fn subject(question: Input) -> Result<Value, Failure> {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let root = product(directory.path());
    let operations = directory.path().join(".asymptote/operations");
    let copied = || {
        std::fs::create_dir_all(&operations).expect("the lock's directory");
        for entry in std::fs::read_dir(FIXTURES).expect("the fixtures") {
            let path = entry.expect("a fixture").path();
            let _: u64 = std::fs::copy(&path, operations.join(path.file_name().expect("a name")))
                .expect("a copy");
        }
    };
    let answer = match question {
        Question::Rewritten => rewritten(&root, &operations),
        Question::Concurrent => concurrent(&root, &operations),
        Question::Interrupted => {
            copied();
            std::fs::write(
                directory.path().join(".asymptote/.0123.lock.tmp"),
                "version = 1\n",
            )
            .expect("an interrupted append's temporary file");
            let read = adapter(&root).read()?;
            Ok(read.0.len() == 2
                && matches!(fold(&read.0), Ok(folded) if !folded.records.is_empty()))
        }
        Question::Mode => modes(&root, &operations),
        Question::Stray => {
            copied();
            std::fs::write(operations.join(".0123.lock.writing"), "").expect("a stray file");
            adapter(&root).read().map(|_| true)
        }
        Question::Contract => {
            check(&adapter(&root));
            Ok(true)
        }
        Question::FirstAsymptotes => {
            copied();
            let read = adapter(&root).read()?;
            let folded = fold(&read.0).expect("they fold");
            Ok(read.0.len() == 2 && folded.heads.len() == 1 && !folded.records.is_empty())
        }
        Question::Crlf => {
            let (id, text) = fixture();
            adapter(&root)
                .parse(&id, &OperationText(text.replace('\n', "\r\n").into()))
                .map(|_| true)
        }
        Question::Edited => {
            let (id, text) = fixture();
            adapter(&root)
                .parse(
                    &id,
                    &OperationText(text.replacen("version = 1", "version = 1\n", 1).into()),
                )
                .map(|_| true)
        }
        Question::NoToml => {
            let text = "not = [toml";
            let id = OperationId(blake3::hash(text.as_bytes()).to_hex().as_str().into());
            adapter(&root)
                .parse(&id, &OperationText(text.into()))
                .map(|_| true)
        }
        Question::Unreadable => {
            std::fs::create_dir_all(directory.path().join(".asymptote"))
                .expect("the lock's parent");
            std::fs::write(&operations, "").expect("a file where the lock's directory is");
            adapter(&root).listing().map(|_| true)
        }
        Question::Unwritable => {
            let file = directory.path().join("file");
            std::fs::write(&file, "").expect("a file");
            let operation = Operation {
                parents: Vec::new(),
                covers: Vec::new(),
                accepted: Vec::new(),
                removed: Vec::new(),
                perf: Vec::new(),
                perf_removed: Vec::new(),
            };
            adapter(&product(&file.join("product")))
                .append(&operation)
                .map(|()| true)
        }
    };
    if let Err(error) = &answer {
        assert!(matches!(
            error,
            LockError::Address { .. }
                | LockError::Malformed { .. }
                | LockError::Unreadable { .. }
                | LockError::Unwritable { .. }
        ));
    }
    answer
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
