//! What the `violations` service normalizes, relates and keys.

use super::service::{anchor, key, keys, normalize, relative};
use super::types::{
    ColumnNumber, Fingerprint, Fingerprinted, Keyed, Level, LineNumber, MessageText, Place,
    RepoPath, RuleId, Suppression, ToolId, Violation,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    Normalize(&'static str),
    Relative(&'static str, &'static str),
    Anchor(Place),
    Keyed(Fingerprinted),
    Keys,
}

type Input = Question;
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("dots_and_backslashes_are_normalized_lexically"),
        word: Word::Succeeds,
        input: || Question::Normalize("./src\\a/../b.rs"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_parent_a_relative_path_cannot_resolve_is_kept"),
        word: Word::Succeeds,
        input: || Question::Normalize("../x.rs"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("an_absolute_path_beneath_the_root_becomes_relative"),
        word: Word::Succeeds,
        input: || Question::Relative("/repo/crates/p/src/a.rs", "/repo/crates/p"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_path_outside_the_root_stays_as_it_is"),
        word: Word::Succeeds,
        input: || Question::Relative("/elsewhere/a.rs", "/repo/crates/p"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_whole_file_is_anchored_at_its_first_line"),
        word: Word::Succeeds,
        input: || Question::Anchor(Place::File),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_violation_with_its_fingerprint_has_a_key"),
        word: Word::Succeeds,
        input: || Question::Keyed(Fingerprinted::Known(Fingerprint(7))),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_violation_without_one_is_unkeyed"),
        word: Word::Succeeds,
        input: || Question::Keyed(Fingerprinted::Unknown),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_keys_of_violations_leave_out_the_unkeyed"),
        word: Word::Succeeds,
        input: || Question::Keys,
        expect: Expect::Value(String::new()),
    },
];

fn violation(fingerprint: Fingerprinted) -> Violation {
    Violation {
        path: RepoPath("src/a.rs".into()),
        place: Place::At(LineNumber(3), ColumnNumber(1)),
        tool: ToolId("t".into()),
        rule: RuleId("r".into()),
        level: Level::Error,
        suppression: Suppression::Unsuppressed,
        message: MessageText("m".into()),
        fingerprint,
    }
}

/// The empty text when the answer is the expected one, and the answer otherwise.
fn subject(question: Input) -> Value {
    let (said, expected) = match question {
        Question::Normalize(raw) => (
            normalize(&RepoPath(raw.into())).0.to_string(),
            if raw.starts_with("..") {
                "../x.rs"
            } else {
                "src/b.rs"
            }
            .to_owned(),
        ),
        Question::Relative(path, root) => {
            let answer = relative(&RepoPath(path.into()), &RepoPath(root.into()));
            let expected = if path.starts_with(root) {
                "src/a.rs"
            } else {
                path
            };
            (answer.0.to_string(), expected.to_owned())
        }
        Question::Anchor(place) => (
            format!("{:?}", anchor(place)),
            format!("{:?}", LineNumber(1)),
        ),
        Question::Keyed(fingerprint) => {
            let keyed = key(&violation(fingerprint));
            let has = matches!(keyed, Keyed::Key(_));
            (
                has.to_string(),
                matches!(fingerprint, Fingerprinted::Known(_)).to_string(),
            )
        }
        Question::Keys => {
            let found = keys(&[
                violation(Fingerprinted::Known(Fingerprint(7))),
                violation(Fingerprinted::Unknown),
                violation(Fingerprinted::Known(Fingerprint(7))),
            ]);
            (found.len().to_string(), 1_usize.to_string())
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
