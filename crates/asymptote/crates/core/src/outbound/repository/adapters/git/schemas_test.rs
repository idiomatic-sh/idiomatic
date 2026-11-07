//! What the git schemas read of git's `-z` output.

use super::{history, ignored, introductions, renames};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which output is read.
#[derive(Clone, Copy, Debug)]
enum Question {
    Renames(&'static str),
    History(&'static str),
    Introductions(&'static str),
    Ignored(&'static str),
}

type Input = Question;
type Value = std::borrow::Cow<'static, str>;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rename_among_other_statuses_is_read_in_step"),
        word: Word::Succeeds,
        input: || Question::Renames("M\0x.rs\0R087\0a.rs\0b.rs\0A\0y.rs\0"),
        expect: Expect::Value(std::borrow::Cow::Borrowed("a.rs>b.rs")),
    },
    Case {
        name: CaseName("a_commit_with_its_renames_is_read"),
        word: Word::Succeeds,
        input: || {
            Question::History(
                "\u{1e}0123456789abcdef0123456789abcdef01234567\0\nR100\0a.rs\0b.rs\0",
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "0123456789abcdef0123456789abcdef01234567:1",
        )),
    },
    Case {
        name: CaseName("a_root_commit_adds_a_file"),
        word: Word::Succeeds,
        input: || {
            Question::Introductions(
                "\u{1e}0123456789abcdef0123456789abcdef01234567\0\0\n.asymptote/operations/x.lock\0",
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            ".asymptote/operations/x.lock:Root",
        )),
    },
    Case {
        name: CaseName("only_a_path_a_pattern_ignores_is_ignored"),
        word: Word::Succeeds,
        input: || {
            Question::Ignored(
                "::\t.\n.gitignore:1:target/\ttarget\n.gitignore:2:!keep.sarif\tkeep.sarif\n",
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("target")),
    },
    Case {
        name: CaseName("a_commit_that_is_no_object_id_is_refused"),
        word: Word::Refuses,
        input: || Question::History("\u{1e}HEAD\0"),
        expect: Expect::Refusal(|actual| {
            if actual == "git failed: git log printed \"HEAD\", which is no object id" {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

fn told(question: Input) -> Result<String, Failure> {
    Ok(match question {
        Question::Renames(raw) => renames(raw)
            .iter()
            .map(|rename| format!("{}>{}", rename.from.0, rename.to.0))
            .collect::<Vec<_>>()
            .join(" "),
        Question::History(raw) => history(raw)
            .map_err(|error| error.to_string())?
            .iter()
            .map(|event| format!("{}:{}", event.revision.0, event.renames.len()))
            .collect::<Vec<_>>()
            .join(" "),
        Question::Introductions(raw) => introductions(raw)
            .map_err(|error| error.to_string())?
            .iter()
            .map(|introduction| format!("{}:{:?}", introduction.path.0, introduction.parent))
            .collect::<Vec<_>>()
            .join(" "),
        Question::Ignored(raw) => ignored(raw).join(" "),
    })
}

fn subject(input: Input) -> Result<Value, Failure> {
    told(input).map(std::borrow::Cow::Owned)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
