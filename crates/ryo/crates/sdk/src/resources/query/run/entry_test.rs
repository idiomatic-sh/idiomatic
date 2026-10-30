//! What `query run` promises its caller, one row of `CASES` for each case it has.

use super::entry::run as run_query;
use super::errors::RunError;
use super::types::{RowLimit, RunRequest};
#[cfg(any(unix, windows))]
use crate::cases::Selection;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::capacity::types::MemoryBudget;
use crate::domain::language::types::Languages;
use crate::domain::query::types::RyoQlQuery;

/// What a row gives the function: a query over a directory holding one file, or a root that is not text.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The query text, run over a directory holding one file.
    Query(&'static str),
    /// A root whose directory is absent.
    MissingRoot,
    /// The total remains available when zero result rows are requested.
    ZeroLimit,
    /// A root that is not text. A Unix path from raw bytes and a Windows path from a lone surrogate can be other than
    /// text, so the row and this variant exist there.
    #[cfg(any(unix, windows))]
    RootNotText,
}

/// What a row gives the function.
type Input = Scenario;
/// How many rows the query answers, and how many files nothing read.
type Value = (usize, usize);
/// Why the function ends in error.
type Failure = RunError;

const FUNCTIONS: &str =
    r#"let functions = select * from "function"; emit functions subject "file" key("span");"#;
const MISSPELLED: &str =
    r#"let functions = selekt * from "function"; emit functions subject "file" key("span");"#;

/// Whether a selector picked the error it was asked about.
#[cfg(any(unix, windows))]
const fn selection(matched: bool) -> Selection {
    if matched {
        Selection::Matches
    } else {
        Selection::Differs
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_absent_root_is_a_corpus_error"),
        word: Word::Refuses,
        input: || Scenario::MissingRoot,
        expect: Expect::Refusal(|error| selects(matches!(error, RunError::Corpus { .. }))),
    },
    Case {
        name: CaseName("a_zero_limit_keeps_the_total_but_shows_no_rows"),
        word: Word::Succeeds,
        input: || Scenario::ZeroLimit,
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("returns_rows"),
        word: Word::Succeeds,
        input: || Scenario::Query(FUNCTIONS),
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("a_wrong_query_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Query(MISSPELLED),
        expect: Expect::Refusal(|error| selects(matches!(error, RunError::Query { .. }))),
    },
    #[cfg(any(unix, windows))]
    Case {
        name: CaseName("a_root_that_is_not_text_is_refused"),
        word: Word::Refuses,
        input: || Scenario::RootNotText,
        expect: Expect::Refusal(|error| selection(matches!(error, RunError::PathNotText { .. }))),
    },
];

/// A path that is not text.
#[cfg(unix)]
fn not_text() -> std::path::PathBuf {
    use std::os::unix::ffi::OsStringExt as _;
    std::ffi::OsString::from_vec(vec![b'/', 0xff, 0xfe]).into()
}

/// A path that is not text: a lone surrogate, which no UTF-8 holds.
#[cfg(windows)]
fn not_text() -> std::path::PathBuf {
    use std::os::windows::ffi::OsStringExt as _;
    std::ffi::OsString::from_wide(&[u16::from(b'/'), 0xD800]).into()
}

/// Runs the query over a directory holding one file, with no language linked: the file is unread.
fn subject(scenario: Input) -> Result<Value, Failure> {
    let root = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(root.path().join("notes.txt"), "text").expect("a file");
    let (directory, text) = match scenario {
        Scenario::MissingRoot => (root.path().join("absent"), FUNCTIONS),
        Scenario::ZeroLimit => (
            root.path().to_path_buf(),
            r#"let f = select * from "filesystem.file"; emit f subject path key(path);"#,
        ),
        Scenario::Query(text) => (root.path().to_path_buf(), text),
        #[cfg(any(unix, windows))]
        Scenario::RootNotText => (not_text(), FUNCTIONS),
    };
    let request = RunRequest {
        root: directory,
        query: RyoQlQuery::try_from(text).expect("a query"),
        limit: RowLimit(if matches!(scenario, Scenario::ZeroLimit) {
            0
        } else {
            20
        }),
    };
    let report = run_query(
        Languages::default(),
        &crate::domain::capacity::types::WorkerPool::from(
            crate::resources::linter::lint::types::Workers(std::num::NonZeroUsize::MIN),
        ),
        MemoryBudget::Machine,
        request,
    )?;
    if matches!(scenario, Scenario::ZeroLimit) {
        assert_eq!(report.total.0, 1);
        assert_eq!(report.shown.0, 0);
    }
    Ok((report.rows.len(), report.unread.len()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
