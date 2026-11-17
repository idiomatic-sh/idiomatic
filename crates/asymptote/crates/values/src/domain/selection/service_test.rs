//! What the selectors choose and what an unmatched location names.

use super::service::{chosen, nearest, unmatched};
use super::types::{At, PathMatch, Selectors};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::violations::service::anchor;
use crate::domain::violations::types::{
    ColumnNumber, Fingerprinted, Level, LineNumber, MessageText, Place, RepoPath, RuleId,
    Suppression, ToolId, Violation,
};

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// No selector chooses everything.
    Everything,
    /// A rule selector leaves another rule.
    OtherRule,
    /// A location chooses the violation at it.
    Located,
    /// An unmatched path pattern leaves it.
    Unmatched,
    /// A location nothing is at is named, with the nearest violations.
    Nothing,
}

type Input = Question;
type Value = std::borrow::Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_selector_chooses_every_new_violation"),
        word: Word::Succeeds,
        input: || Question::Everything,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Chosen")),
    },
    Case {
        name: CaseName("a_rule_selector_leaves_another_rule"),
        word: Word::Succeeds,
        input: || Question::OtherRule,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Left")),
    },
    Case {
        name: CaseName("a_location_chooses_the_violation_at_it"),
        word: Word::Succeeds,
        input: || Question::Located,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Chosen")),
    },
    Case {
        name: CaseName("a_path_pattern_that_does_not_match_leaves_it"),
        word: Word::Succeeds,
        input: || Question::Unmatched,
        expect: Expect::Value(std::borrow::Cow::Borrowed("Left")),
    },
    Case {
        name: CaseName("a_location_nothing_is_at_names_the_nearest"),
        word: Word::Succeeds,
        input: || Question::Nothing,
        expect: Expect::Value(std::borrow::Cow::Borrowed("a.rs:9 nearest 10 2")),
    },
];

fn violation(line: u32) -> Violation {
    Violation {
        path: RepoPath("a.rs".into()),
        place: Place::At(LineNumber(line), ColumnNumber(1)),
        tool: ToolId("t".into()),
        rule: RuleId("r".into()),
        level: Level::Error,
        suppression: Suppression::Unsuppressed,
        message: MessageText("m".into()),
        fingerprint: Fingerprinted::Unknown,
    }
}

fn at(line: u32) -> At {
    At {
        path: RepoPath("a.rs".into()),
        line: LineNumber(line),
    }
}

fn told(question: Input) -> Result<String, Failure> {
    let one = violation(2);
    let choice = |selectors: &Selectors, path| format!("{:?}", chosen(&one, selectors, path));
    Ok(match question {
        Question::Everything => choice(&Selectors::default(), PathMatch::Matched),
        Question::OtherRule => choice(
            &Selectors {
                rules: vec![RuleId("other".into())],
                ..Selectors::default()
            },
            PathMatch::Matched,
        ),
        Question::Located => choice(
            &Selectors {
                at: vec![at(2)],
                ..Selectors::default()
            },
            PathMatch::Matched,
        ),
        Question::Unmatched => choice(&Selectors::default(), PathMatch::Unmatched),
        Question::Nothing => {
            let new = [violation(2), violation(10)];
            let wanted = [at(9)];
            let missing = unmatched(&[], &wanted);
            let first = missing.first().ok_or(())?;
            let lines = nearest(&new, first)
                .iter()
                .map(|violation| anchor(violation.place).0.to_string())
                .collect::<Vec<_>>();
            format!(
                "{}:{} nearest {}",
                first.path.0,
                first.line.0,
                lines.join(" ")
            )
        }
    })
}

fn subject(input: Input) -> Result<Value, Failure> {
    told(input).map(std::borrow::Cow::Owned)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
