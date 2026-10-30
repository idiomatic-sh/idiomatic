//! The `listing` service's cases, on values alone.

use super::service::every_file;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::workers::types::Fresh;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusEntry, FileSize, Nanoseconds};

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Lists two files that no rule governs, one of which could not be read.
    Ungoverned,
}

/// What a row of the cases does.
type Input = Scenario;
/// How many records were made.
type Value = usize;
type Failure = ();

fn entry(path: &str) -> CorpusEntry {
    CorpusEntry {
        path: FilePath(path.into()),
        size: FileSize(1),
        modified: Nanoseconds(2),
    }
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::Ungoverned => {
            let files = [entry("a.txt"), entry("b.txt")];
            every_file(&files, Fresh::default(), &[FilePath("b.txt".into())]).len()
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_file_no_rule_governs_has_a_record_unless_its_bytes_could_not_be_read"),
    word: Word::Succeeds,
    input: || Scenario::Ungoverned,
    expect: Expect::Value(1),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
