//! What the `paths` service answers, one row of `CASES` for each path.

use super::service::{ancestors, beneath, directories, join, name, normalize, parent, relative};
use super::types::PathText;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::paths::types::RepoPath;

/// A path, and which question is asked of it.
type Input = (&'static str, Question);
/// The answer, as text.
type Value = String;
type Failure = ();

/// A question asked of a path.
#[derive(Clone, Copy, Debug)]
enum Question {
    Normalize,
    Parent,
    Name,
    Ancestors,
    JoinedUnder(&'static str),
    RelativeTo(&'static str),
    Directories,
    Beneath(&'static str),
}

fn path(text: &str) -> RepoPath {
    RepoPath(text.into())
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("dot_and_empty_segments_are_dropped"),
        word: Word::Succeeds,
        input: || ("./a//b/./c", Question::Normalize),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_parent_is_the_path_before_the_last_segment"),
        word: Word::Succeeds,
        input: || ("a/b/c.txt", Question::Parent),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_name_is_the_last_segment"),
        word: Word::Succeeds,
        input: || ("a/b/c.txt", Question::Name),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("ancestors_run_from_the_nearest_to_the_root"),
        word: Word::Succeeds,
        input: || ("a/b/c.txt", Question::Ancestors),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("joining_folds_a_parent_segment"),
        word: Word::Succeeds,
        input: || ("../d.txt", Question::JoinedUnder("a/b")),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_relative_to_its_base_drops_the_base"),
        word: Word::Succeeds,
        input: || ("a/b/c.txt", Question::RelativeTo("a")),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("directories_hold_every_ancestor_once"),
        word: Word::Succeeds,
        input: || ("a/b/c.txt", Question::Directories),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("beneath_a_directory_never_takes_a_sibling_with_its_prefix"),
        word: Word::Succeeds,
        input: || ("a/b", Question::Beneath("a")),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_climbing_above_the_root_is_refused"),
        word: Word::Refuses,
        input: || ("../a", Question::Normalize),
        expect: Expect::error(()),
    },
];

/// The expected answer of each question, beside the row so the table stays a table of inputs.
fn expected(input: Input) -> &'static str {
    match input.1 {
        Question::Normalize => "a/b/c",
        Question::Parent => "a/b",
        Question::Name => "c.txt",
        Question::Ancestors => "a/b|a|",
        Question::JoinedUnder(_) => "a/d.txt",
        Question::RelativeTo(_) => "b/c.txt",
        Question::Directories => "|a|a/b",
        Question::Beneath(_) => "a/b|a/b/c.txt",
    }
}

fn subject((text, question): Input) -> Result<Value, Failure> {
    let given = path(text);
    let answer = match question {
        Question::Normalize => normalize(PathText(text)).ok_or(())?.0.into_string(),
        Question::Parent => parent(&given).ok_or(())?.0.into_string(),
        Question::Name => name(&given).0.into_string(),
        Question::Ancestors => ancestors(&given)
            .into_iter()
            .map(|path| path.0.into_string())
            .collect::<Vec<_>>()
            .join("|"),
        Question::JoinedUnder(base) => join(&path(base), PathText(text)).ok_or(())?.0.into_string(),
        Question::RelativeTo(base) => relative(&given, &path(base))
            .map(|relative| relative.0)
            .ok_or(())?
            .to_owned(),
        Question::Directories => directories(&[given])
            .into_iter()
            .map(|path| path.0.into_string())
            .collect::<Vec<_>>()
            .join("|"),
        Question::Beneath(directory) => {
            let files = [path("a/b"), path("a/b/c.txt"), path("ab/c.txt")];
            beneath(&files, &path(directory))
                .iter()
                .filter(|file| file.0.starts_with("a/"))
                .map(|file| file.0.to_string())
                .collect::<Vec<_>>()
                .join("|")
        }
    };
    Ok(if answer == expected((text, question)) {
        String::new()
    } else {
        answer
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
