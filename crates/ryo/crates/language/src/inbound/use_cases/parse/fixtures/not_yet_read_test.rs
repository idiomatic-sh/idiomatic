//! The list of relations not yet read: each relation answered `NotYetRead` is named, and no other answer is.

use super::{answers, listed, ships};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::answers::types::AbsenceCause;
use crate::domain::relations::types::Relation;

/// What the case gives: the answers of a language, as a function of the relation; or the command line's manifest a language
/// `go` is looked for in, as its text, or as a directory where the file should be.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The lines the run prints for these answers.
    Answers(fn(Relation) -> Option<AbsenceCause>),
    /// Whether `go` ships, by a manifest of this text.
    Ships(&'static str),
    /// Whether `go` ships, by a manifest that cannot be read.
    Unreadable,
}

type Input = Question;
/// The lines the run prints.
type Value = &'static str;
type Failure = ();

fn function_not_yet_read(relation: Relation) -> Option<AbsenceCause> {
    (relation == Relation::Function).then_some(AbsenceCause::NotYetRead)
}

fn every_other_reason(relation: Relation) -> Option<AbsenceCause> {
    (relation == Relation::Function).then_some(AbsenceCause::NoSuchConstruct)
}

fn everything_read(_relation: Relation) -> Option<AbsenceCause> {
    None
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_relation_not_yet_read_is_listed_by_name"),
        word: Word::Succeeds,
        input: || Question::Answers(function_not_yet_read),
        expect: Expect::Value("not yet read: function"),
    },
    Case {
        name: CaseName("a_relation_with_another_reason_is_not_listed"),
        word: Word::Succeeds,
        input: || Question::Answers(every_other_reason),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_language_that_reads_everything_lists_nothing"),
        word: Word::Succeeds,
        input: || Question::Answers(everything_read),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_language_among_the_default_features_written_over_lines_ships"),
        word: Word::Succeeds,
        input: || {
            Question::Ships(
                "[package]\nname = \"cli\"\n\n[features]\ndefault = [\n  \"css\",\n  \"go\",\n]\n",
            )
        },
        expect: Expect::Value("ships"),
    },
    Case {
        name: CaseName("a_language_left_out_of_the_default_features_does_not_ship"),
        word: Word::Succeeds,
        input: || Question::Ships("[features]\ndefault = [\"css\"]\ngo = []\n"),
        expect: Expect::Value("does not ship"),
    },
    Case {
        name: CaseName("a_manifest_that_does_not_parse_is_an_error"),
        word: Word::Succeeds,
        input: || Question::Ships("[features\ndefault = [\"go\"]\n"),
        expect: Expect::Value("error"),
    },
    Case {
        name: CaseName("a_manifest_that_cannot_be_read_is_an_error"),
        word: Word::Succeeds,
        input: || Question::Unreadable,
        expect: Expect::Value("error"),
    },
];

/// Whether `go` ships by the command line's manifest a language directory in a scratch tree finds, written as `manifest` says.
fn shipped(manifest: Option<&str>) -> &'static str {
    let scratch = std::env::temp_dir().join(format!("ryo-not-yet-read-{}", std::process::id()));
    let language = scratch.join("languages/go");
    let file = scratch.join("cli/Cargo.toml");
    drop(std::fs::remove_dir_all(&scratch));
    std::fs::create_dir_all(&language).expect("the language's directory");
    match manifest {
        Some(text) => {
            std::fs::create_dir_all(scratch.join("cli")).expect("the command line's directory");
            std::fs::write(&file, text).expect("the manifest");
        }
        None => std::fs::create_dir_all(&file).expect("a directory where the manifest should be"),
    }
    let answer = match ships("go", &language) {
        Ok(true) => "ships",
        Ok(false) => "does not ship",
        Err(_) => "error",
    };
    drop(std::fs::remove_dir_all(&scratch));
    answer
}

fn subject(given: Input) -> Result<Value, Failure> {
    Ok(match given {
        Question::Answers(answer) => {
            Box::leak(listed(&answers(answer)).join("\n").into_boxed_str())
        }
        Question::Ships(manifest) => shipped(Some(manifest)),
        Question::Unreadable => shipped(None),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
