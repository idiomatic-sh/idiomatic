//! The `carrying` service's cases, over the `check` fixture's workspace: its one rule handed whole.

use super::service::carried;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::applications::fixtures::{applications, matching, placement, workspace};
use crate::domain::applications::service::known;
use crate::domain::paths::types::RepoPath;

/// The fixture's tree.
const FILES: &[&str] = &[
    "tastemaker.toml",
    "src/a/errors.txt",
    "src/a/stray.txt",
    "src/b/lib.txt",
];

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The rules handed whole, and how many applications are matched here.
    Handed,
}

type Input = Question;
/// What the row said, empty when it said what was expected.
type Value = String;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_release_applied_once_is_handed_whole_with_its_globs"),
    word: Word::Succeeds,
    input: || Question::Handed,
    expect: Expect::Value(String::new()),
}];

fn subject(question: Input) -> Value {
    let workspace = workspace(FILES);
    let placement = placement(&workspace);
    let applied = applications(&workspace, &placement);
    let (said, expected) = match question {
        Question::Handed => {
            let said = match carried(
                &placement,
                &applied,
                &known(&workspace, &placement),
                matching().compile,
                &RepoPath::default(),
            ) {
                Ok(split) => format!(
                    "{} matched {}",
                    split
                        .handed
                        .iter()
                        .map(|rule| format!(
                            "{}+{}",
                            rule.include
                                .iter()
                                .map(|glob| &*glob.0)
                                .collect::<Vec<_>>()
                                .join(","),
                            rule.exclude
                                .iter()
                                .map(|glob| &*glob.0)
                                .collect::<Vec<_>>()
                                .join(",")
                        ))
                        .collect::<Vec<_>>()
                        .join("; "),
                    split.matched.len()
                ),
                Err(failure) => format!("{failure:?}"),
            };
            (said, "**,**/**+ matched 0".to_owned())
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
