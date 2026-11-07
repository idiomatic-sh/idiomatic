//! The `globs` service's cases, on values alone: the fewest globs that cover a set of files.

use std::collections::BTreeSet;

use super::service::{compress, counted};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::paths::types::RepoPath;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// A directory of three files, two of them reached.
    Compressed,
}

type Input = Question;
/// What the row said, empty when it said what was expected.
type Value = String;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_directory_whose_files_are_all_reached_is_one_glob"),
    word: Word::Succeeds,
    input: || Question::Compressed,
    expect: Expect::Value(String::new()),
}];

fn path(text: &str) -> RepoPath {
    RepoPath(text.into())
}

fn subject(question: Input) -> Value {
    let (said, expected) = match question {
        Question::Compressed => {
            let files = [path("w/a/x.txt"), path("w/a/y.txt"), path("w/b/z.txt")];
            let reached = BTreeSet::from([path("w/a/x.txt"), path("w/a/y.txt")]);
            let globs = compress(&reached, &counted(&files), &[], &path("w"));
            (
                globs
                    .iter()
                    .map(|glob| &*glob.0)
                    .collect::<Vec<_>>()
                    .join(","),
                "a/**".to_owned(),
            )
        }
    };
    if said == expected {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
