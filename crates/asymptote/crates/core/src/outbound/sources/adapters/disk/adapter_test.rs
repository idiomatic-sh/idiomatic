//! The disk adapter of the sources port: the contract over a temporary product, a link out of it refused and located
//! at its target, a byte that is no UTF-8 read lossily, and a stamp that follows content.

use super::adapter::SourcesDiskAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use std::time::{Duration, SystemTime};

use crate::domain::manifest::types::ProductRoot;
use crate::domain::reports::service::{recorded, unread};
use crate::domain::reports::types::Recorded;
use crate::domain::violations::types::{Located, RepoPath};
use crate::outbound::sources::errors::SourcesError;
use crate::outbound::sources::fixtures::contract::check;
use crate::outbound::sources::interface::SourcesPort as _;

/// Which question is asked of a temporary product.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The contract.
    Contract,
    /// The path read.
    Read(&'static str),
    /// Whether a link out of the product is located at its target.
    LinkLocated,
    /// Whether a file with a byte that is no UTF-8 is read lossily, so its lines keep their keys.
    Lossy,
    /// Whether overwriting a file with as many bytes in its record's timestamp tick moves its stamp.
    Overwritten,
    /// Whether a file whose size and time match a record taken long after it was modified keeps its stamp unread.
    Settled,
}

type Input = Question;
/// Whether the answer is the expected one.
type Value = bool;
type Failure = SourcesError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_adapter_keeps_the_contract"),
        word: Word::Succeeds,
        input: || Question::Contract,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_link_out_of_the_product_is_located_at_its_target"),
        word: Word::Succeeds,
        input: || Question::LinkLocated,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_line_beside_a_byte_that_is_no_utf8_keeps_the_first_asymptotes_key"),
        word: Word::Succeeds,
        input: || Question::Lossy,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_overwrite_of_as_many_bytes_in_the_records_tick_moves_the_stamp"),
        word: Word::Succeeds,
        input: || Question::Overwritten,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_file_whose_size_and_time_match_a_settled_record_keeps_its_stamp_unread"),
        word: Word::Succeeds,
        input: || Question::Settled,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_link_out_of_the_product_is_refused"),
        word: Word::Refuses,
        input: || Question::Read("out"),
        expect: Expect::Refusal(|actual| {
            if actual == &(SourcesError::Outside { path: "out".into() }) {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_missing_file_is_unreadable"),
        word: Word::Fails,
        input: || Question::Read("src/missing.rs"),
        expect: Expect::Refusal(|error| {
            if matches!(error, SourcesError::Unreadable { path, reason } if path == "src/missing.rs" && !reason.is_empty())
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// Answers one row's question about the disk adapter over a fresh product beside a file outside it.
fn subject(question: Input) -> Result<Value, Failure> {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let product = directory.path().join("product");
    std::fs::create_dir_all(product.join("src")).expect("src");
    std::fs::write(product.join("src/a.rs"), "fn a() {}\n").expect("a file");
    std::fs::write(directory.path().join("secret"), "outside\n").expect("a file outside");
    #[cfg(unix)]
    std::os::unix::fs::symlink(directory.path().join("secret"), product.join("out"))
        .expect("a link");
    // The Windows runner holds the privilege a symbolic link needs.
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(directory.path().join("secret"), product.join("out"))
        .expect("a link");
    let root = ProductRoot(product.display().to_string().into());
    let source = RepoPath("src/a.rs".into());
    Ok(match question {
        Question::Contract => {
            check(&SourcesDiskAdapter::new(root.clone()));
            true
        }
        Question::Read(path) => SourcesDiskAdapter::new(root.clone())
            .read(&RepoPath(path.into()))
            .map(|_| true)?,
        Question::LinkLocated => {
            let target = directory
                .path()
                .canonicalize()
                .expect("the directory")
                .join("secret");
            SourcesDiskAdapter::new(root.clone()).locate(&RepoPath("out".into()))
                == Located::Outside(RepoPath(target.display().to_string().into()))
                && SourcesDiskAdapter::new(root.clone()).locate(&source) == Located::Inside
                || cfg!(not(unix))
        }
        Question::Lossy => {
            std::fs::write(product.join("src/a.rs"), b"fn main() {}\n// calf\xe9\n")
                .expect("a file that is no UTF-8");
            // `String::from_utf8_lossy`: the digest adapter keys the known line from this text.
            SourcesDiskAdapter::new(root.clone())
                .read(&source)?
                .0
                .as_ref()
                == "fn main() {}\n// calf\u{fffd}\n"
        }
        Question::Overwritten | Question::Settled => {
            let file = product.join("src/a.rs");
            let set = |time| {
                std::fs::File::options()
                    .write(true)
                    .open(&file)
                    .and_then(|opened| opened.set_modified(time))
                    .expect("its time set");
            };
            if matches!(question, Question::Settled) {
                set(SystemTime::now() - Duration::from_hours(1));
            }
            // What the check does: each file's state, read only where the record cannot vouch for it.
            let stamped = |known: &Recorded| {
                let paths = std::slice::from_ref(&source);
                let stats = SourcesDiskAdapter::new(root.clone()).stats(paths);
                let read = SourcesDiskAdapter::new(root.clone()).digests(&unread(known, &stats));
                recorded(known, &stats, &read.0)
            };
            let before = stamped(&Recorded::default());
            let time = std::fs::metadata(&file)
                .and_then(|metadata| metadata.modified())
                .expect("its time");
            std::fs::write(&file, "fn b() {}\n").expect("as many bytes");
            set(time);
            let after = stamped(&before);
            // Overwritten in the tick its record was taken in, the record is racy, so the file is read again; with
            // its record taken long after, the same size and time keep the recorded stamp unread, as git and cargo
            // keep theirs.
            if matches!(question, Question::Overwritten) {
                before.files != after.files
            } else {
                before.files == after.files
            }
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

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
